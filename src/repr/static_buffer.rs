use super::*;
use core::ptr;

// len は 2^(sizeof(usize) * 7) - 1 以下 まで。HeapBuffer のように heap に置き直すことはしない。
// なぜなら、これの利用目的は、`&'static str` に対する最適化であり、`&'static str` は基本的にソース
// コード上に直接書かれるものであるため、その長さはそこまで大きくないと考えられるため。
#[repr(C)]
pub(super) struct StaticBuffer {
    ptr: ptr::NonNull<u8>,
    len: usize, // stored as little-endian
}

const USIZE_SIZE: usize = size_of::<usize>();

impl StaticBuffer {
    const MAX_LENGTH: usize = {
        let mut bytes = [255; USIZE_SIZE];
        bytes[USIZE_SIZE - 1] = 0;
        usize::from_le_bytes(bytes)
    };
    const LENGTH_MASK: usize = Self::MAX_LENGTH;

    const TAG: usize = {
        const USIZE_SIZE: usize = size_of::<usize>();
        let mut bytes = [0; USIZE_SIZE];
        bytes[USIZE_SIZE - 1] = LastByte::StaticMarker as u8;
        usize::from_ne_bytes(bytes)
    };

    pub(super) const fn new(text: &'static str) -> Result<Self, ReserveError> {
        let text_len = text.len();

        if text_len > Self::MAX_LENGTH {
            return Err(ReserveError);
        }
        let len = text_len.to_le() | Self::TAG;

        // SAFETY: `&'static str` must have a non-null, properly aligned address
        let ptr = unsafe { ptr::NonNull::new_unchecked(text.as_ptr() as *mut _) };

        Ok(Self { ptr, len })
    }

    pub(super) fn len(&self) -> usize {
        self.len & Self::LENGTH_MASK
    }

    /// # Safety
    /// `len` bytes in the buffer must be valid UTF-8.
    pub(super) unsafe fn set_len(&mut self, len: usize) {
        self.len = len.to_le() | Self::TAG;
    }
}
