use super::ReserveError;

use core::{mem, ptr, slice, str};

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

    pub(crate) fn with_capacity(capacity: usize) -> Result<Self, ReserveError> {
        if capacity <= MAX_INLINE_SIZE {
            Ok(Repr::new())
        } else {
            HeapBuffer::with_capacity(capacity).map(Repr::from_heap)
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
            // SAFETY: We just checked the discriminant to make sure we're heap allocated
            unsafe { self.as_heap_buffer() }.capacity()
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

        if self.is_heap_buffer() {
            // SAFETY: We just checked that `self` is HeapBuffer
            let heap = unsafe { self.as_heap_buffer_mut() };

            if !heap.is_unique() {
                // We should decrement the reference count of the current HeapBuffer, because we
                // reallocate a new HeapBuffer based on the current HeapBuffer.
                //
                // SAFETY:
                // We just checked that `heap` is not unique, so the reference count is at least
                // 2. And then, the return value cannot be `1`, no need to check it and free the
                // HeapBuffer.
                unsafe { heap.decrement_reference_count() };

                let str = heap.as_str();
                let new_heap = HeapBuffer::with_additional(str, additional)?;
                *self = Repr::from_heap(new_heap);
            } else if needed_capacity > heap.capacity() {
                // heap is unique, and we need to reserve more capacity.

                let amortized_capacity = heap_buffer::amortized_growth(len, additional);
                // SAFETY:
                // - `heap` is unique.
                // - `amortized_capacity` is greater than `len`.
                unsafe { heap.realloc(amortized_capacity)? };
            } else {
                // We have enough capacity, no need to reserve.
            }
            Ok(())
        } else if self.is_static_buffer() {
            // We can't modify it, need to convert to other buffer.

            if needed_capacity <= MAX_INLINE_SIZE {
                // SAFETY: `len <= needed_capacity <= MAX_INLINE_SIZE`
                let inline = unsafe { InlineBuffer::new(self.as_str()) };
                *self = Repr::from_inline(inline);
            } else {
                let heap = HeapBuffer::with_additional(self.as_str(), additional)?;
                *self = Repr::from_heap(heap);
            }
            Ok(())
        } else {
            // self is InlineBuffer

            if needed_capacity > MAX_INLINE_SIZE {
                let heap = HeapBuffer::with_additional(self.as_str(), additional)?;
                *self = Repr::from_heap(heap);
            } else {
                // We have enough capacity, no need to reserve.
            }
            Ok(())
        }
    }

    pub fn shrink_to(&mut self, min_capacity: usize) -> Result<(), ReserveError> {
        // If the buffer is not heap allocated, we can't shrink it.
        if !self.is_heap_buffer() {
            return Ok(());
        }

        // SAFETY: We did early return if the buffer is not HeapBuffer.
        let heap = unsafe { self.as_heap_buffer_mut() };

        let new_capacity = heap.len().max(min_capacity);
        let old_capacity = heap.capacity();

        if new_capacity <= MAX_INLINE_SIZE {
            // We can convert the HeapBuffer to InlineBuffer.

            // SAFETY:
            // `heap.len() <= new_capacity` and `new_capacity <= MAX_INLINE_SIZE`
            // thus, `heap.len() <= MAX_INLINE_SIZE`
            let inline = unsafe {
                let str = heap.as_str();
                InlineBuffer::new(str)
            };

            // We should release the reference to the HeapBuffer and deallocate it if needed.
            // SAFETY:
            // - We just have reference to the HeapBuffer, so the reference count is at least 1.
            // - We deallocate the HeapBuffer if the reference count was `1`.
            unsafe {
                let count = heap.decrement_reference_count();
                if count == 1 {
                    heap.dealloc();
                }
            }

            *self = Repr::from_inline(inline);
            return Ok(());
        }

        // No need to shrink the buffer.
        if new_capacity >= old_capacity {
            return Ok(());
        }

        if heap.is_unique() {
            // Try to extend the buffer in place.
            // SAFETY: `heap` is unique, and `new_capacity < old_capacity`
            unsafe { heap.realloc(new_capacity)? };
            Ok(())
        } else {
            // We need to create a new buffer because the current buffer is shared with others.
            let str = heap.as_str();
            let additional = new_capacity - str.len();
            let new_heap = HeapBuffer::with_additional(str, additional)?;
            *self = Repr::from_heap(new_heap);
            Ok(())
        }
    }

    pub(crate) fn push_str(&mut self, string: &str) -> Result<(), ReserveError> {
        if string.is_empty() {
            return Ok(());
        }
        let len = self.len();
        let str_len = string.len();

        self.reserve(str_len)?;

        let push_buffer = {
            // SAFETY: by calling `self.reserve()`:
            // - The buffer is not StaticBuffer
            // - If the buffer is HeapBuffer, it must be unique.
            let slice = unsafe { self.as_slice_mut() };
            &mut slice[len..len + str_len]
        };

        debug_assert_eq!(push_buffer.len(), string.as_bytes().len());
        push_buffer.copy_from_slice(string.as_bytes());

        // SAFETY:
        // by calling `self.reserve()`
        // - We have reserved enough capacity.
        // - Make buffer unique if it is HeapBuffer.
        // and by `copy_from_slice`:
        // - `0..(len + str_len)` is initialized.
        unsafe { self.set_len(len + str_len) };

        Ok(())
    }

    pub(crate) fn pop(&mut self) -> Result<Option<char>, ReserveError> {
        let ch = match self.as_str().chars().next_back() {
            Some(ch) => ch,
            None => return Ok(None),
        };

        // SAFETY: We know this is is a valid length which falls on a char boundary
        let new_len = self.len() - ch.len_utf8();

        if self.is_heap_buffer() {
            // SAFETY: We just checked that `self` is HeapBuffer
            let heap = unsafe { self.as_heap_buffer_mut() };

            if heap.is_unique() {
                // SAFETY:
                // - `new_len <= len <= capacity`
                // - We just checked that `self` is unique.
                unsafe { heap.set_len(new_len) };
            } else {
                // We need to reallocate a new buffer (not only HeapBuffer) because the current
                // buffer is shared with others.

                // SAFETY:
                // We just checked that `heap` is not unique, so the reference count is at least
                // 2. And then, the return value cannot be `1`, no need to check it and free the
                // HeapBuffer.
                unsafe { heap.decrement_reference_count() };

                // SAFETY: `ptr` is valid for `len` bytes, and `HeapBuffer` contains valid UTF-8.
                let str = unsafe {
                    let ptr = self.0 as *mut u8;
                    let slice = slice::from_raw_parts_mut(ptr, new_len);
                    str::from_utf8_unchecked_mut(slice)
                };
                *self = Repr::from_str(str)?;
            }
        } else if self.is_static_buffer() {
            // SAFETY:
            // - We just checked that `self` is StaticBuffer
            // - `new_len <= len <= capacity`
            unsafe { self.as_static_buffer_mut().set_len(new_len) };
        } else {
            // SAFETY:
            // - The number of types of buffer is 3, and the remaining is InlineBuffer.
            // - From `#Safety`, `new_len <= MAX_INLINE_SIZE` is true.
            unsafe { self.as_inline_buffer_mut().set_len(new_len) };
        }

        Ok(Some(ch))
    }

    pub(crate) fn insert_str(&mut self, idx: usize, string: &str) -> Result<(), ReserveError> {
        assert!(self.as_str().is_char_boundary(idx));

        let new_len = self.len().checked_add(string.len()).ok_or(ReserveError)?;

        // reserve makes self unique and modifiable
        self.reserve(string.len())?;
        debug_assert!(self.is_unique());
        debug_assert!(!self.is_static_buffer());

        // SAFETY:
        // - We contracted that we can split self at `idx`.
        // - We just reserved enough capacity and set length after reserving.
        // - The gap is filled by valid UTF-8 bytes.
        unsafe {
            // first move the tail to the new back
            let data = self.as_slice_mut().as_mut_ptr();
            ptr::copy(
                data.add(idx),
                data.add(idx + string.len()),
                new_len - idx - string.len(),
            );

            // then insert the new bytes
            ptr::copy_nonoverlapping(string.as_ptr(), data.add(idx), string.len());

            // and lastly resize the string
            self.set_len(new_len);
        }
        Ok(())
    }

    pub(crate) fn is_unique(&self) -> bool {
        if self.is_heap_buffer() {
            // SAFETY: We just checked the discriminant to make sure we're heap allocated
            unsafe { self.as_heap_buffer() }.is_unique()
        } else {
            true
        }
    }

    pub(crate) fn make_shallow_clone(&self) -> Self {
        if self.is_heap_buffer() {
            // SAFETY: We just checked that `self` is HeapBuffer.
            let heap = unsafe { self.as_heap_buffer() };
            heap.increment_reference_count();
        }

        // SAFETY:
        // - if `self` is HeapBuffer, we just incremented the reference count.
        // - if `self` is InlineBuffer or StaticBuffer, we just copied the bytes.
        unsafe { ptr::read(self) }
    }

    pub(crate) fn replace_inner(&mut self, other: Self) {
        if self.is_heap_buffer() {
            // SAFETY: We just checked the discriminant to make sure we're heap allocated
            let heap = unsafe { self.as_heap_buffer_mut() };

            // SAFETY:
            // - We just have reference to the HeapBuffer, so the reference count is at least 1.
            // - We deallocate the HeapBuffer if the reference count was `1`.
            unsafe {
                let count = heap.decrement_reference_count();
                if count == 1 {
                    heap.dealloc();
                }
            }
        }

        *self = other;
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
    /// - The buffer is not StaticBuffer
    /// - If the buffer is HeapBuffer, it must be unique.
    unsafe fn as_slice_mut(&mut self) -> &mut [u8] {
        debug_assert!(!self.is_static_buffer());

        let (ptr, cap) = if self.is_heap_buffer() {
            let ptr = self.0 as *mut u8;
            // SAFETY: We just checked that `self` is HeapBuffer
            let heap = unsafe { self.as_heap_buffer() };
            debug_assert!(heap.is_unique());
            (ptr, heap.capacity())
        } else {
            let ptr = self as *mut _ as *mut u8;
            (ptr, MAX_INLINE_SIZE)
        };

        slice::from_raw_parts_mut(ptr, cap)
    }

    /// # Safety
    /// - `new_len` must be less than or equal to `capacity()`
    /// - The elements at `0..new_len` must be initialized.
    /// - If the underlying buffer is a `HeapBuffer`, it must be unique.
    /// - If the underlying buffer is a `InlineBuffer`, `new_len <= MAX_INLINE_SIZE` must be true.
    pub(crate) unsafe fn set_len(&mut self, new_len: usize) {
        debug_assert!(new_len <= self.capacity());

        if self.is_static_buffer() {
            // SAFETY:
            // - We just checked that `self` is StaticBuffer
            // - `new_len` is less than or equal to `capacity()`
            unsafe { self.as_static_buffer_mut().set_len(new_len) };
        } else if self.is_heap_buffer() {
            // SAFETY:
            // - We just checked that `self` is HeapBuffer.
            // - From `#Safety`, the buffer is unique.
            unsafe { self.as_heap_buffer_mut().set_len(new_len) };
        } else {
            // SAFETY:
            // - The number of types of buffer is 3, and the remaining is InlineBuffer.
            // - From `#Safety`, `new_len <= MAX_INLINE_SIZE` is true.
            unsafe { self.as_inline_buffer_mut().set_len(new_len) };
        }
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
    unsafe fn as_inline_buffer_mut(&mut self) -> &mut InlineBuffer {
        // SAFETY: A `Repr` is transmuted from `InlineBuffer`
        &mut *(self as *mut _ as *mut InlineBuffer)
    }

    #[inline(always)]
    unsafe fn as_heap_buffer(&self) -> &HeapBuffer {
        // SAFETY: A `Repr` is transmuted from `HeapBuffer`
        &*(self as *const _ as *const HeapBuffer)
    }

    #[inline(always)]
    unsafe fn as_heap_buffer_mut(&mut self) -> &mut HeapBuffer {
        // SAFETY: A `Repr` is transmuted from `HeapBuffer`
        &mut *(self as *mut _ as *mut HeapBuffer)
    }

    #[inline(always)]
    unsafe fn as_static_buffer(&self) -> &StaticBuffer {
        // SAFETY: A `Repr` is transmuted from `StaticBuffer`
        &*(self as *const _ as *const StaticBuffer)
    }

    #[inline(always)]
    unsafe fn as_static_buffer_mut(&mut self) -> &mut StaticBuffer {
        // SAFETY: A `Repr` is transmuted from `StaticBuffer`
        &mut *(self as *mut _ as *mut StaticBuffer)
    }
}
