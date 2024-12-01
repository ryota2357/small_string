use super::*;
use core::{mem, slice};

mod heap_buffer;
use heap_buffer::HeapBuffer;

mod inline_buffer;
use inline_buffer::InlineBuffer;

mod static_buffer;
use static_buffer::StaticBuffer;

mod last_byte;
#[cfg(not(feature = "last_byte"))]
use last_byte::LastByte;
#[cfg(feature = "last_byte")]
pub use last_byte::LastByte;

const MAX_INLINE_SIZE: usize = 2 * size_of::<usize>();

#[repr(C)]
#[cfg(target_pointer_width = "64")]
pub(crate) struct Repr(*const (), [u8; 7], LastByte);

fn _static_assert() {
    const {
        assert!(size_of::<Repr>() == MAX_INLINE_SIZE);
        assert!(size_of::<Option<Repr>>() == MAX_INLINE_SIZE);
        assert!(align_of::<Repr>() == align_of::<usize>());
        assert!(align_of::<Option<Repr>>() == align_of::<usize>());
    }
}

impl Repr {
    pub(crate) const fn new() -> Self {
        Repr::from_inline(InlineBuffer::empty())
    }

    pub(crate) fn from_str(text: &str) -> Result<Self, ReserveError> {
        if text.len() <= MAX_INLINE_SIZE {
            // SAFETY: `text.len()` is less than or equal to `MAX_INLINE_SIZE`
            Ok(Repr::from_inline(unsafe { InlineBuffer::new(text) }))
        } else {
            HeapBuffer::new(text).map(Repr::from_heap)
        }
    }

    pub(crate) const fn from_static_str(text: &'static str) -> Result<Self, ReserveError> {
        if text.len() <= MAX_INLINE_SIZE {
            // SAFETY: `text.len()` is less than or equal to `MAX_INLINE_SIZE`
            Ok(Repr::from_inline(unsafe { InlineBuffer::new(text) }))
        } else {
            // NOTE: .map(Repr::from_heap) is not possible in a `const fn`
            match StaticBuffer::new(text) {
                Ok(buffer) => Ok(Repr::from_static(buffer)),
                Err(e) => Err(e),
            }
        }
    }

    #[cfg(target_pointer_width = "64")]
    pub(crate) fn len(&self) -> usize {
        let mut len = {
            // SAFETY:`Repr` is same size of [usize; 2], and aligned as usize
            let mut tail_bytes = unsafe {
                let tail = (self as *const _ as *const usize).add(1);
                *(tail as *const [u8; 8])
            };
            tail_bytes[7] = 0;
            usize::from_le_bytes(tail_bytes)
        };

        let last_byte = self.last_byte();

        let inline_len = (last_byte as usize)
            .wrapping_sub(LastByte::MASK_1100_0000 as usize)
            .min(MAX_INLINE_SIZE);

        // This code is compiled to a single branchless instruction, such as `cmov`
        if last_byte < LastByte::HeapMarker as u8 {
            len = inline_len
        }

        len
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub(crate) fn capacity(&self) -> usize {
        if self.is_heap_buffer() {
            #[cold]
            fn heap_capacity(this: &Repr) -> usize {
                // SAFETY: We just checked the discriminant to make sure we're heap allocated
                let heap_buffer = unsafe { this.as_heap_buffer() };
                heap_buffer.capacity()
            }
            heap_capacity(self)
        } else if self.is_static_buffer() {
            // SAFETY: we just checked that `self` is StaticBuffer
            unsafe { self.as_static_buffer() }.len()
        } else {
            MAX_INLINE_SIZE
        }
    }

    pub fn as_str(&self) -> &str {
        // SAFETY: A `Repr` contains valid UTF-8
        unsafe { str::from_utf8_unchecked(self.as_bytes()) }
    }

    pub(crate) fn as_bytes(&self) -> &[u8] {
        let len = self.len();

        let ptr = if self.last_byte() >= LastByte::HeapMarker as u8 {
            self.0 as *const u8
        } else {
            self as *const _ as *const u8
        };

        // SAFETY: data (`ptr`) is valid, aligned, and part of the same contiguous allocated `len`
        // chunk
        unsafe { slice::from_raw_parts(ptr, len) }
    }

    pub(crate) fn reserve(&mut self, additional: usize) -> Result<(), ReserveError> {
        let len = self.len();
        let needed_capacity = len.checked_add(additional).ok_or(ReserveError)?;

        if !self.is_static_buffer() && needed_capacity <= self.capacity() {
            // - StaticBuffer: we can't modify it, need to convert to other buffer.
            // - Other buffer: we have enough capacity, no need to reserve.
            Ok(())
        } else if needed_capacity <= MAX_INLINE_SIZE {
            // We can use an inline buffer instead of heap allocating.

            // SAFETY:
            //  - In general, `capacity >= len` must be true.
            //  - In this case, `needed_capacity <= MAX_INLINE_SIZE` is true.
            //  So, `self.as_str().len() <= MAX_INLINE_SIZE` is true.
            let inline = unsafe { InlineBuffer::new(self.as_str()) };
            *self = Repr::from_inline(inline);
            Ok(())
        } else if !self.is_heap_buffer() {
            // We're not heap allocated, but need to be, create a HeapBuffer
            let heap = HeapBuffer::with_additional(self.as_str(), additional)?;
            *self = Repr::from_heap(heap);
            Ok(())
        } else {
            // We're already heap allocated, but we need more capacity

            // SAFETY: We checked above to see if we're heap allocated
            unsafe { self.as_heap_buffer_mut() }.reserve(additional)
        }
    }

    pub(crate) fn push_str(&mut self, string: &str) {
        if string.is_empty() {
            return;
        }
        let len = self.len();
        let str_len = string.len();

        self.reserve(str_len).unwrap_with_msg();

        // SAFETY: TODO
        let slice = unsafe { self.as_slice_mut() };
        let push_buffer = &mut slice[len..len + str_len];

        debug_assert_eq!(push_buffer.len(), string.as_bytes().len());
        push_buffer.copy_from_slice(string.as_bytes());

        unsafe { self.set_len(len + str_len) };
    }

    pub(crate) fn pop(&mut self) -> Option<char> {
        let ch = self.as_str().chars().next_back()?;

        // SAFETY: We know this is is a valid length which falls on a char boundary
        let new_len = self.len() - ch.len_utf8();
        unsafe { self.set_len(new_len) };

        Some(ch)
    }

    #[inline(always)]
    pub(crate) fn is_heap_buffer(&self) -> bool {
        self.last_byte() == LastByte::HeapMarker as u8
    }

    #[inline(always)]
    const fn is_static_buffer(&self) -> bool {
        self.last_byte() == LastByte::StaticMarker as u8
    }

    /// # Safety
    unsafe fn as_slice_mut(&mut self) -> &mut [u8] {
        debug_assert!(!self.is_static_buffer());

        let (ptr, cap) = if self.is_heap_buffer() {
            // SAFETY: We just checked that `self` is HeapBuffer
            let heap = unsafe { self.as_heap_buffer() };
            (heap.non_null_ptr().as_ptr(), heap.capacity())
        } else {
            let ptr = self as *mut _ as *mut u8;
            (ptr, MAX_INLINE_SIZE)
        };

        slice::from_raw_parts_mut(ptr, cap)
    }

    /// # Safety
    /// - `new_len` must be less than or equal to `capacity()`
    /// - The elements at `old_len..new_len` must be initialized
    pub(crate) unsafe fn set_len(&mut self, new_len: usize) {
        debug_assert!(new_len <= self.capacity());
        todo!()
    }

    #[inline(always)]
    const fn from_inline(buffer: InlineBuffer) -> Self {
        unsafe { mem::transmute(buffer) }
    }

    #[inline(always)]
    const fn from_heap(buffer: HeapBuffer) -> Self {
        unsafe { mem::transmute(buffer) }
    }

    #[inline(always)]
    const fn from_static(buffer: StaticBuffer) -> Self {
        unsafe { mem::transmute(buffer) }
    }

    #[inline(always)]
    const fn last_byte(&self) -> u8 {
        self.2 as u8
    }

    #[inline(always)]
    unsafe fn as_heap_buffer(&self) -> &HeapBuffer {
        // SAFETY: A `Repr` is transmuted from `StaticStr`
        &*(self as *const _ as *const HeapBuffer)
    }

    #[inline(always)]
    unsafe fn as_heap_buffer_mut(&mut self) -> &mut HeapBuffer {
        // SAFETY: A `HeapBuffer` and `Repr` have the same size
        &mut *(self as *mut _ as *mut HeapBuffer)
    }

    #[inline(always)]
    unsafe fn as_static_buffer(&self) -> &StaticBuffer {
        // SAFETY: A `Repr` is transmuted from `StaticStr`
        &*(self as *const _ as *const StaticBuffer)
    }
}
