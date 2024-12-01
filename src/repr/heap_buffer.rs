use super::*;
use core::{alloc::Layout, ptr, ptr::NonNull, sync::atomic::AtomicUsize};
use std::alloc;

use internal::TextSize;

#[repr(C)]
pub(crate) struct HeapBuffer {
    // | Header | Data (array of `u8`) |
    //          ^ ptr
    ptr: NonNull<u8>,
    len: TextSize,
}

#[cfg(target_pointer_width = "64")]
struct Header {
    count: AtomicUsize,
    capacity: TextSize,
}

fn _static_assert() {
    const {
        assert!(size_of::<HeapBuffer>() == MAX_INLINE_SIZE);
        assert!(align_of::<HeapBuffer>() == align_of::<usize>());
    }
}

impl HeapBuffer {
    #[cfg(target_pointer_width = "64")]
    pub(crate) fn new(text: &str) -> Result<Self, ReserveError> {
        let text_len = text.len();
        let len = TextSize::new(text_len);
        let ptr = unsafe {
            let allocation = allocate_non_zero(size_of::<Header>() + text_len);
            *(allocation as *mut Header) = Header {
                count: AtomicUsize::new(1),
                capacity: len,
            };
            NonNull::new_unchecked(allocation.add(Self::header_offset()))
        };
        unsafe {
            // SAFETY:
            // - src (`text`) and dst (`ptr`) is valid for `text_len` bytes.
            // - Both src and dst is aligned for u8.
            // - src and dst don't overlap because we created dst.
            ptr::copy_nonoverlapping(text.as_ptr(), ptr.as_ptr(), text_len);
        }
        Ok(Self { ptr, len })
    }

    pub(crate) fn with_additional(text: &str, additional: usize) -> Result<Self, ReserveError> {
        todo!()
    }

    #[inline(always)]
    pub(crate) fn len(&self) -> usize {
        self.len.as_usize()
    }

    pub(crate) fn capacity(&self) -> usize {
        todo!()
    }

    pub(crate) fn non_null_ptr(&self) -> NonNull<u8> {
        self.ptr
    }

    pub(crate) fn reserve(&mut self, additional: usize) -> Result<(), ReserveError> {
        todo!()
    }

    const fn align() -> usize {
        const {
            assert!(align_of::<Header>() == align_of::<usize>());
            assert!(align_of::<NonNull<u8>>() == align_of::<usize>());
        }
        align_of::<usize>()
    }

    const fn header_offset() -> usize {
        max(size_of::<Header>(), Self::align())
    }
}

/// # Safety
/// - `size` must be non-zero.
unsafe fn allocate_non_zero(size: usize) -> *mut u8 {
    debug_assert!(size > 0);

    let align = HeapBuffer::align();
    assert!(size < (isize::MAX as usize - align), "size overflow");

    // SAFETY:
    // - align (`ALIGN`) is non-zero,
    // - align is 8 or 4 which is a power of two,
    // - size is ensured not to overflow isize when rounded up to the nearest multiple of align by
    //   above assertion.
    let layout = unsafe { Layout::from_size_align_unchecked(size, align) };

    // SAFETY: the size of layout is non-zero, it is ensured in the first line of this function.
    let allocation = unsafe { alloc::alloc(layout) };
    if allocation.is_null() {
        alloc::handle_alloc_error(layout);
    }
    allocation
}

/// const version of `std::cmp::max::<usize>(x, y)`.
const fn max(x: usize, y: usize) -> usize {
    if x > y {
        x
    } else {
        y
    }
}

#[cold]
const fn capacity_overflow() -> ! {
    panic!("capacity overflow");
}

mod internal {
    use super::*;

    /// The length and capacity of a [`HeapBuffer`].
    ///
    /// An unsinged integer that uses `size_of::<usize>() - 1` bytes, and the rest 1 byte is used
    /// as a tag.
    ///
    /// Internally, the integer is stored in little-endian order, so the memory layout is like:
    ///
    /// +--------------------------------+--------+
    /// |        unsinged integer        |   tag  |
    /// | (size_of::<usize>() - 1) bytes | 1 byte |
    /// +--------------------------------+--------+
    ///
    /// And the tag is [`LastByte::Heap`].
    ///
    /// In this representation, the max value is limited to:
    ///
    /// - (on 64-bit architecture) 2^56 - 1 = 72057594037927935 = 64 PiB
    /// - (on 32-bit architecture) 2^24 - 2 = 16777214          ≈ 16 MiB
    ///
    /// Practically speaking, on 64-bit architecture, this max value is enough for the
    /// length/capacity of a HeapBuffer. However, it is not enough for 32-bit architectures, and if
    /// more than 3 bytes are needed, the length/capacity must be switched to be stored using the
    /// heap. Therefore, on 32-bit architecture, we use 2^24 - 2 as the maximum value, and 2^24 - 1
    /// as the tag that indicates the length/capacity is stored in the heap.
    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub(super) struct TextSize(usize);

    const USIZE_SIZE: usize = size_of::<usize>();

    impl TextSize {
        #[cfg(target_pointer_width = "64")]
        const MAX: usize = {
            let mut bytes = [255; USIZE_SIZE];
            bytes[USIZE_SIZE - 1] = 0;
            usize::from_le_bytes(bytes)
        };

        const TAG: usize = {
            let mut bytes = [0; USIZE_SIZE];
            bytes[USIZE_SIZE - 1] = LastByte::HeapMarker as u8;
            usize::from_ne_bytes(bytes)
        };

        const MASK: usize = (1 << ((USIZE_SIZE - 1) * 8)) - 1;

        #[cfg(target_pointer_width = "64")]
        pub(super) const fn new(size: usize) -> Self {
            if size > Self::MAX {
                capacity_overflow();
            }
            TextSize(size.to_le() | Self::TAG)
        }

        #[inline(always)]
        pub(super) fn as_usize(&self) -> usize {
            usize::from_le(self.0 & Self::MASK)
        }
    }
}
