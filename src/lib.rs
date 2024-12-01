use core::{fmt, str::FromStr};

mod repr;
use repr::Repr;

mod reserve_error;
pub use reserve_error::ReserveError;

#[cfg(feature = "last_byte")]
pub use repr::LastByte;

#[repr(transparent)]
pub struct SmallString(Repr);

fn _static_assert() {
    const {
        assert!(size_of::<SmallString>() == 2 * size_of::<usize>());
        assert!(size_of::<Option<SmallString>>() == 2 * size_of::<usize>());
        assert!(align_of::<SmallString>() == align_of::<usize>());
        assert!(align_of::<Option<SmallString>>() == align_of::<usize>());
    }
}

impl SmallString {
    /// Creates a new empty `SmallString`.
    ///
    /// Same as `String::new()`, this will not allocate on the heap.
    ///
    /// # Examples
    ///
    /// ```
    /// # use small_string::SmallString;
    /// let s = SmallString::new();
    /// assert!(s.is_empty());
    /// assert!(!s.is_heap_allocated());
    /// ```
    pub const fn new() -> Self {
        Self(Repr::new())
    }

    pub const fn from_static_str(text: &'static str) -> Self {
        match Repr::from_static_str(text) {
            Ok(repr) => Self(repr),
            Err(_) => panic!("text is too long"),
        }
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn capacity(&self) -> usize {
        self.0.capacity()
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }

    pub fn reserve(&mut self, additional: usize) {
        self.0.reserve(additional)
    }

    pub fn push(&mut self, ch: char) {
        self.0.push(ch)
    }

    pub fn pop(&mut self) -> Option<char> {
        self.0.pop()
    }

    pub fn push_str(&mut self, string: &str) {
        self.0.push_str(string)
    }

    pub fn clear(&mut self) {
        unsafe { self.0.set_len(0) }
    }

    pub fn is_heap_allocated(&self) -> bool {
        self.0.is_heap_buffer()
    }
}

impl Default for SmallString {
    fn default() -> Self {
        Self::new()
    }
}

impl From<&str> for SmallString {
    fn from(text: &str) -> Self {
        Self(Repr::from_str(text).unwrap_with_msg())
    }
}

impl FromStr for SmallString {
    type Err = ReserveError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Repr::from_str(s).map(Self)
    }
}

trait UnwrapWithMsg {
    type T;
    fn unwrap_with_msg(self) -> Self::T;
}

impl<T, E: fmt::Display> UnwrapWithMsg for Result<T, E> {
    type T = T;
    #[inline(always)]
    #[track_caller]
    fn unwrap_with_msg(self) -> T {
        #[inline(never)]
        #[cold]
        #[track_caller]
        fn do_panic_with_msg<E: fmt::Display>(error: E) -> ! {
            panic!("{error}")
        }

        match self {
            Ok(value) => value,
            Err(err) => do_panic_with_msg(err),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke() {
        let s = SmallString::from("hello asdfasdfasdfasdfasdf");
        assert_eq!(s.as_str(), "hello asdfasdfasdfasdfasdf");
    }
}
