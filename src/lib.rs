use core::{
    borrow::Borrow,
    cmp, fmt,
    hash::{Hash, Hasher},
    ops::Deref,
    str,
};
use std::{borrow::Cow, ffi::OsStr};

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
    /// Same as [`String::new()`], this will not allocate on the heap.
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
        SmallString(Repr::new())
    }

    /// Creates a new `SmallString` from a `&'static str`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use small_string::SmallString;
    /// let s = SmallString::from_static_str("This is a static lifetime string");
    /// assert_eq!(s.as_str(), "This is a static lifetime string");
    /// assert_eq!(s.len(), 32);
    /// assert!(!s.is_heap_allocated());
    /// ```
    pub const fn from_static_str(text: &'static str) -> Self {
        match Repr::from_static_str(text) {
            Ok(repr) => SmallString(repr),
            Err(_) => panic!("text is too long"),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        SmallString::try_with_capacity(capacity).unwrap_with_msg()
    }

    pub fn try_with_capacity(capacity: usize) -> Result<Self, ReserveError> {
        Repr::with_capacity(capacity).map(SmallString)
    }

    /// Return the length of the string in bytes, not [`char`] or graphemes.
    ///
    /// # Examples
    /// ```
    /// # use small_string::SmallString;
    /// let a = SmallString::from("foo");
    /// assert_eq!(a.len(), 3);
    ///
    /// let fancy_f = SmallString::from("ƒoo");
    /// assert_eq!(fancy_f.len(), 4);
    /// assert_eq!(fancy_f.chars().count(), 3);
    /// ```
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
        self.0.reserve(additional).unwrap_with_msg()
    }

    pub fn try_reserve(&mut self, additional: usize) -> Result<(), ReserveError> {
        self.0.reserve(additional)
    }

    pub fn push(&mut self, ch: char) {
        self.0
            .push_str(ch.encode_utf8(&mut [0; 4]))
            .unwrap_with_msg()
    }

    pub fn pop(&mut self) -> Option<char> {
        self.0.pop().unwrap_with_msg()
    }

    pub fn push_str(&mut self, string: &str) {
        self.0.push_str(string).unwrap_with_msg()
    }

    pub fn clear(&mut self) {
        if self.0.is_unique() {
            // SAFETY:
            // - `self` is unique.
            // - 0 bytes is always valid UTF-8, and initialized.
            unsafe { self.0.set_len(0) }
        } else {
            self.0.replace_inner(Repr::new());
        }
    }

    pub fn is_heap_allocated(&self) -> bool {
        self.0.is_heap_buffer()
    }
}

impl Clone for SmallString {
    #[inline]
    fn clone(&self) -> Self {
        SmallString(self.0.make_shallow_clone())
    }

    #[inline]
    fn clone_from(&mut self, source: &Self) {
        self.0.replace_inner(source.0.make_shallow_clone());
    }
}

impl Drop for SmallString {
    fn drop(&mut self) {
        self.0.replace_inner(Repr::new());
    }
}

impl Default for SmallString {
    fn default() -> Self {
        Self::new()
    }
}

impl Deref for SmallString {
    type Target = str;
    fn deref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Debug for SmallString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_str(), f)
    }
}

impl fmt::Display for SmallString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.as_str(), f)
    }
}

impl AsRef<str> for SmallString {
    #[inline]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl AsRef<OsStr> for SmallString {
    #[inline]
    fn as_ref(&self) -> &OsStr {
        OsStr::new(self.as_str())
    }
}

impl AsRef<[u8]> for SmallString {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl Borrow<str> for SmallString {
    #[inline]
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl Eq for SmallString {}

impl PartialEq for SmallString {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.as_str().eq(other.as_str())
    }
}

impl PartialEq<str> for SmallString {
    #[inline]
    fn eq(&self, other: &str) -> bool {
        self.as_str().eq(other)
    }
}

impl PartialEq<SmallString> for str {
    #[inline]
    fn eq(&self, other: &SmallString) -> bool {
        self.eq(other.as_str())
    }
}

impl PartialEq<&str> for SmallString {
    #[inline]
    fn eq(&self, other: &&str) -> bool {
        self.as_str().eq(*other)
    }
}

impl PartialEq<SmallString> for &str {
    #[inline]
    fn eq(&self, other: &SmallString) -> bool {
        (*self).eq(other.as_str())
    }
}

impl PartialEq<String> for SmallString {
    #[inline]
    fn eq(&self, other: &String) -> bool {
        self.as_str().eq(other.as_str())
    }
}

impl PartialEq<SmallString> for String {
    #[inline]
    fn eq(&self, other: &SmallString) -> bool {
        self.as_str().eq(other.as_str())
    }
}

impl PartialEq<Cow<'_, str>> for SmallString {
    #[inline]
    fn eq(&self, other: &Cow<'_, str>) -> bool {
        self.as_str().eq(other.as_ref())
    }
}

impl PartialEq<SmallString> for Cow<'_, str> {
    #[inline]
    fn eq(&self, other: &SmallString) -> bool {
        self.as_ref().eq(other.as_str())
    }
}

impl Ord for SmallString {
    #[inline]
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.as_str().cmp(other.as_str())
    }
}

impl PartialOrd for SmallString {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Hash for SmallString {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_str().hash(state)
    }
}

impl From<char> for SmallString {
    fn from(value: char) -> Self {
        SmallString(Repr::from_str(value.encode_utf8(&mut [0; 4])).unwrap_with_msg())
    }
}

impl From<&str> for SmallString {
    #[inline]
    #[track_caller]
    fn from(value: &str) -> Self {
        SmallString(Repr::from_str(value).unwrap_with_msg())
    }
}

impl str::FromStr for SmallString {
    type Err = ReserveError;

    #[inline]
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
