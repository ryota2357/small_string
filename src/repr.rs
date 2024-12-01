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

    pub(crate) fn len(&self) -> usize {
        todo!()
        // if self.is_heap_buffer() {
        //     // SAFETY: we just checked that `self` is heap-allocated
        //     unsafe { self.as_heap() }.len()
        // } else if self.is_static_buffer() {
        //     // SAFETY: we just checked that `self` is static
        //     unsafe { self.as_static() }.len()
        // } else {
        //     (self.last_byte() & LastByte::LENGTH_MASK_0011_1111) as usize
        // }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub(crate) fn capacity(&self) -> usize {
        if self.is_heap_buffer() {
            // SAFETY: we just checked that `self` is heap-allocated
            unsafe { self.as_heap() }.capacity()
        } else if self.is_static_buffer() {
            // SAFETY: we just checked that `self` is static
            unsafe { self.as_static() }.len()
        } else {
            MAX_INLINE_SIZE
        }
    }

    pub(crate) fn as_str(&self) -> &str {
        // SAFETY: A `Repr` contains valid UTF-8
        unsafe { core::str::from_utf8_unchecked(self.as_bytes()) }
    }

    pub(crate) fn as_bytes(&self) -> &[u8] {
        // TODO: fix for static buffer
        let (ptr, len) = if self.is_heap_buffer() {
            let ptr = self.0 as *const u8;
            // SAFETY: we just checked that `self` is heap-allocated
            let len = unsafe { self.as_heap() }.len();
            (ptr, len)
        } else {
            let ptr = self as *const Self as *const u8;
            let len = self.last_byte() & LastByte::LENGTH_MASK_0011_1111;
            (ptr, len as usize)
        };

        // SAFETY: data (`ptr`) is valid, aligned, and part of the same contiguous allocated `len` chunk
        unsafe { slice::from_raw_parts(ptr, len) }
    }

    pub(crate) fn reserve(&mut self, _additional: usize) {
        todo!()
    }

    pub(crate) fn push(&mut self, _ch: char) {
        todo!()
    }

    pub(crate) fn pop(&mut self) -> Option<char> {
        todo!()
    }

    pub(crate) fn push_str(&mut self, _string: &str) {
        todo!()
    }

    #[inline(always)]
    pub(crate) fn is_heap_buffer(&self) -> bool {
        self.last_byte() == LastByte::Heap as u8
    }

    #[inline(always)]
    const fn is_static_buffer(&self) -> bool {
        self.last_byte() == LastByte::Static as u8
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
    unsafe fn as_heap(&self) -> &HeapBuffer {
        // SAFETY: A `Repr` is transmuted from `StaticStr`
        &*(self as *const _ as *const HeapBuffer)
    }

    #[inline(always)]
    unsafe fn as_static(&self) -> &StaticBuffer {
        // SAFETY: A `Repr` is transmuted from `StaticStr`
        &*(self as *const _ as *const StaticBuffer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke() {
        let repr = Repr::from_str("012345678901234a").unwrap();
        println!("{}", repr.capacity());
        todo!()
    }
}
