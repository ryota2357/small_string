use super::*;
use core::{
    alloc::Layout,
    hint, ptr,
    ptr::NonNull,
    sync::atomic::{AtomicUsize, Ordering::*},
};
use std::alloc;

use internal::TextSize;

/// [`HeapBuffer`] grows at an amortized rates of 1.5x
#[inline(always)]
pub(crate) fn amortized_growth(cur_len: usize, additional: usize) -> usize {
    let required = cur_len.saturating_add(additional);
    let amortized = cur_len.saturating_mul(3) / 2;
    amortized.max(required)
}

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
    capacity: usize,
}

fn _static_assert() {
    const {
        assert!(size_of::<HeapBuffer>() == MAX_INLINE_SIZE);
        assert!(align_of::<HeapBuffer>() == align_of::<usize>());
    }
}

impl HeapBuffer {
    #[cfg(target_pointer_width = "64")]
    pub(super) fn new(text: &str) -> Result<Self, ReserveError> {
        let text_len = text.len();

        let len = TextSize::new(text_len);
        let ptr = HeapBuffer::allocate_ptr(text_len)?;

        // SAFETY:
        // - src (`text`) and dst (`ptr`) is valid for `text_len` bytes because `text_len` comes
        //   from `text`, and `ptr` was allocated to be at least that length.
        // - Both src and dst is aligned for u8.
        // - src and dst don't overlap because we allocated dst just now.
        unsafe { ptr::copy_nonoverlapping(text.as_ptr(), ptr.as_ptr(), text_len) };

        Ok(HeapBuffer { ptr, len })
    }

    pub(super) fn with_additional(text: &str, additional: usize) -> Result<Self, ReserveError> {
        let text_len = text.len();

        let len = TextSize::new(text_len);
        let ptr = {
            let new_capacity = amortized_growth(text_len, additional);
            HeapBuffer::allocate_ptr(new_capacity)?
        };

        // SAFETY:
        // - src (`text`) and dst (`ptr`) is valid for `text_len` bytes because `text_len` comes
        //   from `text`, and `ptr` was allocated to be at least `new_capacity` bytes, which is
        //   greater than `text_len`.
        // - Both src and dst is aligned for u8.
        // - src and dst don't overlap because we allocated dst just now.
        unsafe { ptr::copy_nonoverlapping(text.as_ptr(), ptr.as_ptr(), text_len) };

        Ok(HeapBuffer { ptr, len })
    }

    pub(super) fn capacity(&self) -> usize {
        self.header().capacity
    }

    /// # Safety
    /// The buffer must be unique. (HeapBuffer::is_unique() == true)
    pub(super) unsafe fn realloc(&mut self, new_capacity: usize) -> Result<(), ReserveError> {
        debug_assert!(self.is_unique());

        let cur_layout = match Self::layout_from_capacity(self.header().capacity) {
            Ok(layout) => layout,
            Err(_) => {
                if cfg!(debug_assertions) {
                    panic!("invalid layout, unexpected `capacity` modification may have occurred");
                }
                // SAFETY:
                // `layout_from_capacity` should not return `Err` because this layout should not
                // have been changed since it was used in the previous allocation.
                unsafe { hint::unreachable_unchecked() }
            }
        };

        const ALLOC_LIMIT: usize = (isize::MAX as usize + 1) - HeapBuffer::align();
        let new_alloc_size = size_of::<Header>() + new_capacity;
        if new_alloc_size > ALLOC_LIMIT {
            capacity_overflow();
        }

        // SAFETY:
        // - `self.ptr` is already allocated by global allocator.
        // - current allocation is allocated by `cur_layout`.
        // - `new_alloc_size` is greater than zero.
        // - `new_alloc_size` is ensured not to overflow when rounded up to the nearest multiple of
        //    alignment by `ALLOC_LIMIT`.
        let allocation = unsafe { alloc::realloc(self.ptr.as_ptr(), cur_layout, new_alloc_size) };
        if allocation.is_null() {
            return Err(ReserveError);
        }

        // SAFETY:
        // - `allocation` is non-null.
        // - the allocation size is larger than or equal to the size of Header.
        unsafe {
            ptr::write(
                allocation.cast(),
                Header {
                    count: AtomicUsize::new(1), // is_unique() is true.
                    capacity: new_capacity,
                },
            );
            let ptr = allocation.add(HeapBuffer::header_offset());
            self.ptr = NonNull::new_unchecked(ptr);
        }

        Ok(())
    }

    /// # Safety
    /// The reference count must be 0.
    pub(super) unsafe fn dealloc(&mut self) {
        debug_assert!(self.header().count.load(Acquire) == 0);

        let layout = match Self::layout_from_capacity(self.header().capacity) {
            Ok(layout) => layout,
            Err(_) => {
                if cfg!(debug_assertions) {
                    panic!("invalid layout, unexpected `capacity` modification may have occurred");
                }
                // SAFETY:
                // `layout_from_capacity` should not return `Err` because this layout should not
                // have been changed since it was used in the previous allocation.
                unsafe { hint::unreachable_unchecked() }
            }
        };
        let ptr = self.ptr.as_ptr().cast();
        alloc::dealloc(ptr, layout);
    }

    pub(super) fn is_unique(&self) -> bool {
        self.header().count.load(Acquire) == 1
    }

    /// # Safety
    /// Caller must ensure tha following:
    ///  - The current reference count is greater than 0 when calling this method.
    ///  - If the return value is 0, this HeapBuffer is properly destroyed.
    pub(super) unsafe fn decrement_reference_count(&self) -> usize {
        debug_assert!(self.header().count.load(Acquire) > 0);
        self.header().count.fetch_sub(1, Release)
    }

    pub(super) fn increment_reference_count(&self) -> usize {
        self.header().count.fetch_add(1, Release)
    }

    /// # Safety
    /// `len` bytes in the buffer must be valid UTF-8.
    pub(super) unsafe fn set_len(&mut self, len: usize) {
        debug_assert!(self.is_unique());
        self.len = TextSize::new(len);
    }

    fn allocate_ptr(capacity: usize) -> Result<NonNull<u8>, ReserveError> {
        let layout = HeapBuffer::layout_from_capacity(capacity)?;

        // SAFETY: layout is non-zero.
        let allocation = unsafe { alloc::alloc(layout) };
        if allocation.is_null() {
            return Err(ReserveError);
        }

        // SAFETY:
        // - allocation is non-null.
        // - allocation size is larger than or equal to the size of Header.
        unsafe {
            ptr::write(
                allocation.cast(),
                Header {
                    count: AtomicUsize::new(1),
                    capacity,
                },
            );
            let ptr = allocation.add(HeapBuffer::header_offset());
            Ok(NonNull::new_unchecked(ptr))
        }
    }

    fn layout_from_capacity(capacity: usize) -> Result<Layout, ReserveError> {
        let alloc_size = size_of::<Header>() + capacity;
        let align = HeapBuffer::align();
        Layout::from_size_align(alloc_size, align).map_err(
            #[cold]
            |_| ReserveError,
        )
    }

    unsafe fn allocation(&self) -> *mut u8 {
        unsafe { self.ptr.as_ptr().cast::<u8>().sub(Self::header_offset()) }
    }

    fn header(&self) -> &Header {
        unsafe { &*(self.allocation() as *const Header) }
    }

    const fn align() -> usize {
        const {
            assert!(align_of::<Header>() == align_of::<usize>());
            assert!(align_of::<NonNull<u8>>() == align_of::<usize>());
        }
        align_of::<usize>()
    }

    const fn header_offset() -> usize {
        max(size_of::<Header>(), HeapBuffer::align())
    }
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

        #[cfg(target_pointer_width = "64")]
        pub(super) const fn new(size: usize) -> Self {
            if size > Self::MAX {
                capacity_overflow();
            }
            TextSize(size.to_le() | Self::TAG)
        }
    }
}
