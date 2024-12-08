use core::{
    borrow::Borrow,
    cmp, fmt,
    hash::{Hash, Hasher},
    ops::Deref,
    str,
    str::FromStr,
};
use std::{borrow::Cow, ffi::OsStr};

mod repr;
use repr::Repr;

mod reserve_error;
pub use reserve_error::ReserveError;

#[cfg(feature = "last_byte")]
pub use repr::LastByte;

#[repr(transparent)]
pub struct LeanString(Repr);

fn _static_assert() {
    const {
        assert!(size_of::<LeanString>() == 2 * size_of::<usize>());
        assert!(size_of::<Option<LeanString>>() == 2 * size_of::<usize>());
        assert!(align_of::<LeanString>() == align_of::<usize>());
        assert!(align_of::<Option<LeanString>>() == align_of::<usize>());
    }
}

impl LeanString {
    /// Creates a new empty `LeanString`.
    ///
    /// Same as [`String::new()`], this will not allocate on the heap.
    ///
    /// # Examples
    ///
    /// ```
    /// # use lean_string::LeanString;
    /// let s = LeanString::new();
    /// assert!(s.is_empty());
    /// assert!(!s.is_heap_allocated());
    /// ```
    pub const fn new() -> Self {
        LeanString(Repr::new())
    }

    /// Creates a new `LeanString` from a `&'static str`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use lean_string::LeanString;
    /// let s = LeanString::from_static_str("Long text but static lifetime");
    /// assert_eq!(s.as_str(), "Long text but static lifetime");
    /// assert_eq!(s.len(), 29);
    /// assert!(!s.is_heap_allocated());
    /// ```
    pub const fn from_static_str(text: &'static str) -> Self {
        match Repr::from_static_str(text) {
            Ok(repr) => LeanString(repr),
            Err(_) => panic!("text is too long"),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        LeanString::try_with_capacity(capacity).unwrap_with_msg()
    }

    pub fn try_with_capacity(capacity: usize) -> Result<Self, ReserveError> {
        Repr::with_capacity(capacity).map(LeanString)
    }

    /// Return the length of the string in bytes, not [`char`] or graphemes.
    ///
    /// # Examples
    ///
    /// ```
    /// # use lean_string::LeanString;
    /// let a = LeanString::from("foo");
    /// assert_eq!(a.len(), 3);
    ///
    /// let fancy_f = LeanString::from("ƒoo");
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
        self.try_reserve(additional).unwrap_with_msg()
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

impl Clone for LeanString {
    #[inline]
    fn clone(&self) -> Self {
        LeanString(self.0.make_shallow_clone())
    }

    #[inline]
    fn clone_from(&mut self, source: &Self) {
        self.0.replace_inner(source.0.make_shallow_clone());
    }
}

impl Drop for LeanString {
    fn drop(&mut self) {
        self.0.replace_inner(Repr::new());
    }
}

impl Default for LeanString {
    fn default() -> Self {
        Self::new()
    }
}

impl Deref for LeanString {
    type Target = str;
    fn deref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Debug for LeanString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_str(), f)
    }
}

impl fmt::Display for LeanString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.as_str(), f)
    }
}

impl AsRef<str> for LeanString {
    #[inline]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl AsRef<OsStr> for LeanString {
    #[inline]
    fn as_ref(&self) -> &OsStr {
        OsStr::new(self.as_str())
    }
}

impl AsRef<[u8]> for LeanString {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl Borrow<str> for LeanString {
    #[inline]
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl Eq for LeanString {}

impl PartialEq for LeanString {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.as_str().eq(other.as_str())
    }
}

impl PartialEq<str> for LeanString {
    #[inline]
    fn eq(&self, other: &str) -> bool {
        self.as_str().eq(other)
    }
}

impl PartialEq<LeanString> for str {
    #[inline]
    fn eq(&self, other: &LeanString) -> bool {
        self.eq(other.as_str())
    }
}

impl PartialEq<&str> for LeanString {
    #[inline]
    fn eq(&self, other: &&str) -> bool {
        self.as_str().eq(*other)
    }
}

impl PartialEq<LeanString> for &str {
    #[inline]
    fn eq(&self, other: &LeanString) -> bool {
        (*self).eq(other.as_str())
    }
}

impl PartialEq<String> for LeanString {
    #[inline]
    fn eq(&self, other: &String) -> bool {
        self.as_str().eq(other.as_str())
    }
}

impl PartialEq<LeanString> for String {
    #[inline]
    fn eq(&self, other: &LeanString) -> bool {
        self.as_str().eq(other.as_str())
    }
}

impl PartialEq<Cow<'_, str>> for LeanString {
    #[inline]
    fn eq(&self, other: &Cow<'_, str>) -> bool {
        self.as_str().eq(other.as_ref())
    }
}

impl PartialEq<LeanString> for Cow<'_, str> {
    #[inline]
    fn eq(&self, other: &LeanString) -> bool {
        self.as_ref().eq(other.as_str())
    }
}

impl Ord for LeanString {
    #[inline]
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.as_str().cmp(other.as_str())
    }
}

impl PartialOrd for LeanString {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Hash for LeanString {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_str().hash(state)
    }
}

impl From<char> for LeanString {
    fn from(value: char) -> Self {
        LeanString(Repr::from_str(value.encode_utf8(&mut [0; 4])).unwrap_with_msg())
    }
}

impl From<&str> for LeanString {
    #[inline]
    #[track_caller]
    fn from(value: &str) -> Self {
        LeanString(Repr::from_str(value).unwrap_with_msg())
    }
}

impl FromStr for LeanString {
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
