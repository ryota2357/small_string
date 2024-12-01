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

    #[inline(always)]
    pub(crate) fn len(&self) -> usize {
        self.len.as_usize()
    }

    pub(crate) fn capacity(&self) -> usize {
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

    /// The wrapper of `usize` to represent the size of the text of `SmallString`.
    ///
    /// This is ensured to be `usize`, plus the following:
    /// - Less than or equal to:
    ///     - 2^56 - 1 if `usize` is 64-bit.
    ///     - i.e. Two bits on the MBS side are 0.
    /// - Little-endian.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub(super) struct TextSize(usize);

    const USIZE_SIZE: usize = size_of::<usize>();

    impl TextSize {
        const MAX: usize = {
            let mut bytes = [255; USIZE_SIZE];
            bytes[USIZE_SIZE - 1] = 0;
            usize::from_le_bytes(bytes) - 1
        };
        const TAG: usize = {
            let mut bytes = [0; USIZE_SIZE];
            bytes[USIZE_SIZE - 1] = LastByte::Heap as u8;
            usize::from_ne_bytes(bytes)
        };
        const MASK: usize = (1 << (USIZE_SIZE * 7)) - 1;

        pub(super) const ON_THE_HEAP: TextSize = TextSize(Self::MASK | Self::TAG);

        #[cfg(target_pointer_width = "64")]
        pub(super) const fn new(size: usize) -> Self {
            if size > Self::MAX {
                capacity_overflow();
            }
            TextSize(size.to_le() | Self::TAG)
        }
        #[cfg(target_pointer_width = "32")]
        pub(super) const fn new(size: usize) -> Self {
            if size > Self::MAX {
                TextSize::ON_THE_HEAP
            } else {
                TextSize(size.to_le() | Self::TAG)
            }
        }

        #[inline(always)]
        pub(super) fn as_usize(&self) -> usize {
            usize::from_le(self.0 & Self::MASK)
        }
    }
}
