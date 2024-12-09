#![doc = include_str!("../README.md")]

use core::{
    borrow::Borrow,
    cmp, fmt,
    hash::{Hash, Hasher},
    ops::{Add, AddAssign, Deref},
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

    /// Creates a new [`LeanString`] from a `&'static str`.
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

    /// Creates a new empty [`LeanString`] with at least capacity bytes.
    ///
    /// A [`LeanString`] will inline strings if the length is less than or equal to
    /// `2 * size_of::<usize>()` bytes. This means that the minimum capacity of a [`LeanString`]
    /// is `2 * size_of::<usize>()` bytes.
    ///
    /// # Panics
    ///
    /// Panics the following conditions are met:
    ///
    /// - The system is out-of-memory.
    /// - On 64-bit architecture, the `capacity` is greater than `2^56 - 1`. Note that this is a
    ///   very rare case, as it means that 64 PiB of heap memory is required.
    ///
    /// If you want to handle such a problem manually, use [`LeanString::try_with_capacity()`].
    ///
    /// # Examples
    ///
    /// ## inline capacity
    ///
    /// ```
    /// # use lean_string::LeanString;
    /// let s = LeanString::with_capacity(4);
    /// assert_eq!(s.capacity(), 2 * size_of::<usize>());
    /// assert!(!s.is_heap_allocated());
    /// ```
    ///
    /// ## heap capacity
    ///
    /// ```
    /// # use lean_string::LeanString;
    /// let s = LeanString::with_capacity(100);
    /// assert_eq!(s.capacity(), 100);
    /// assert!(s.is_heap_allocated());
    /// ```
    pub fn with_capacity(capacity: usize) -> Self {
        LeanString::try_with_capacity(capacity).unwrap_with_msg()
    }

    /// Fallible version of [`LeanString::with_capacity()`]
    ///
    /// This method won't panic if the system is out-of-memory, or the `capacity` is too large in
    /// 64-bit architecture, but return an [`ReserveError`]. Otherwise it behaves the same as
    /// [`LeanString::with_capacity()`].
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

    /// Returns `true` if the [`LeanString`] has a length of 0, `false` otherwise
    ///
    /// # Examples
    ///
    /// ```
    /// # use lean_string::LeanString;
    /// let mut s = LeanString::new();
    /// assert!(s.is_empty());
    ///
    /// s.push('a');
    /// assert!(!s.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the capacity of the [`LeanString`], in bytes.
    ///
    /// A [`LeanString`] will inline strings if the length is less than or equal to
    /// `2 * size_of::<usize>()` bytes. This means that the minimum capacity of a [`LeanString`]
    /// is `2 * size_of::<usize>()` bytes.
    ///
    /// # Examples
    ///
    /// ### inline capacity
    ///
    /// ```
    /// # use lean_string::LeanString;
    /// let s = LeanString::new();
    /// assert_eq!(s.capacity(), 2 * size_of::<usize>());
    /// ```
    ///
    /// ### heap capacity
    ///
    /// ```
    /// # use lean_string::LeanString;
    /// let s = LeanString::with_capacity(100);
    /// assert_eq!(s.capacity(), 100);
    /// ```
    pub fn capacity(&self) -> usize {
        self.0.capacity()
    }

    /// Returns a string slice containing the entire [`LeanString`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use lean_string::LeanString;
    /// let s = LeanString::from("foo");
    /// assert_eq!(s.as_str(), "foo");
    /// ```
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// Returns a byte slice containing the entire [`LeanString`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use lean_string::LeanString;
    /// let s = LeanString::from("hello");
    /// assert_eq!(&[104, 101, 108, 108, 111], s.as_bytes());
    /// ```
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
        self.try_push(ch).unwrap_with_msg()
    }

    pub fn try_push(&mut self, ch: char) -> Result<(), ReserveError> {
        self.0.push_str(ch.encode_utf8(&mut [0; 4]))
    }

    pub fn pop(&mut self) -> Option<char> {
        self.try_pop().unwrap_with_msg()
    }

    pub fn try_pop(&mut self) -> Result<Option<char>, ReserveError> {
        self.0.pop()
    }

    pub fn push_str(&mut self, string: &str) {
        self.try_push_str(string).unwrap_with_msg()
    }

    pub fn try_push_str(&mut self, string: &str) -> Result<(), ReserveError> {
        self.0.push_str(string)
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

impl FromIterator<char> for LeanString {
    fn from_iter<T: IntoIterator<Item = char>>(iter: T) -> Self {
        let iter = iter.into_iter();

        let (lower_bound, _) = iter.size_hint();
        let mut repr = match Repr::with_capacity(lower_bound) {
            Ok(buf) => buf,
            Err(_) => Repr::new(), // Ignore the error and hope that the lower_bound is incorrect.
        };

        for ch in iter {
            repr.push_str(ch.encode_utf8(&mut [0; 4])).unwrap_with_msg();
        }
        LeanString(repr)
    }
}

impl<'a> FromIterator<&'a char> for LeanString {
    fn from_iter<T: IntoIterator<Item = &'a char>>(iter: T) -> Self {
        iter.into_iter().copied().collect()
    }
}

impl<'a> FromIterator<&'a str> for LeanString {
    fn from_iter<I: IntoIterator<Item = &'a str>>(iter: I) -> Self {
        let mut buf = LeanString::new();
        buf.extend(iter);
        buf
    }
}

impl FromIterator<Box<str>> for LeanString {
    fn from_iter<I: IntoIterator<Item = Box<str>>>(iter: I) -> Self {
        let mut buf = LeanString::new();
        buf.extend(iter);
        buf
    }
}

impl<'a> FromIterator<Cow<'a, str>> for LeanString {
    fn from_iter<I: IntoIterator<Item = Cow<'a, str>>>(iter: I) -> Self {
        let mut buf = LeanString::new();
        buf.extend(iter);
        buf
    }
}

impl FromIterator<String> for LeanString {
    fn from_iter<I: IntoIterator<Item = String>>(iter: I) -> Self {
        let mut buf = LeanString::new();
        buf.extend(iter);
        buf
    }
}

impl FromIterator<LeanString> for LeanString {
    fn from_iter<T: IntoIterator<Item = LeanString>>(iter: T) -> Self {
        let mut buf = LeanString::new();
        buf.extend(iter);
        buf
    }
}

impl Extend<char> for LeanString {
    fn extend<T: IntoIterator<Item = char>>(&mut self, iter: T) {
        let iter = iter.into_iter();

        let (lower_bound, _) = iter.size_hint();
        // Ignore the error and hope that the lower_bound is incorrect.
        let _ = self.try_reserve(lower_bound);

        for ch in iter {
            self.push(ch);
        }
    }
}

impl<'a> Extend<&'a char> for LeanString {
    fn extend<T: IntoIterator<Item = &'a char>>(&mut self, iter: T) {
        self.extend(iter.into_iter().copied());
    }
}

impl<'a> Extend<&'a str> for LeanString {
    fn extend<T: IntoIterator<Item = &'a str>>(&mut self, iter: T) {
        iter.into_iter().for_each(|s| self.push_str(s));
    }
}

impl Extend<Box<str>> for LeanString {
    fn extend<T: IntoIterator<Item = Box<str>>>(&mut self, iter: T) {
        iter.into_iter().for_each(move |s| self.push_str(&s));
    }
}

impl<'a> Extend<Cow<'a, str>> for LeanString {
    fn extend<T: IntoIterator<Item = Cow<'a, str>>>(&mut self, iter: T) {
        iter.into_iter().for_each(move |s| self.push_str(&s));
    }
}

impl Extend<String> for LeanString {
    fn extend<T: IntoIterator<Item = String>>(&mut self, iter: T) {
        iter.into_iter().for_each(move |s| self.push_str(&s));
    }
}

impl Extend<LeanString> for LeanString {
    fn extend<T: IntoIterator<Item = LeanString>>(&mut self, iter: T) {
        for s in iter {
            self.push_str(&s);
        }
    }
}

impl Extend<LeanString> for String {
    fn extend<T: IntoIterator<Item = LeanString>>(&mut self, iter: T) {
        for s in iter {
            self.push_str(&s);
        }
    }
}

impl fmt::Write for LeanString {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.push_str(s);
        Ok(())
    }
}

impl Add<&str> for LeanString {
    type Output = Self;
    fn add(mut self, rhs: &str) -> Self::Output {
        self.push_str(rhs);
        self
    }
}

impl AddAssign<&str> for LeanString {
    fn add_assign(&mut self, rhs: &str) {
        self.push_str(rhs);
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
