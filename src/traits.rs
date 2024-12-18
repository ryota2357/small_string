use crate::LeanString;
use core::fmt;

/// A trait for converting a value to a [`LeanString`].
pub trait ToLeanString {
    fn to_lean_string(&self) -> LeanString;
}

// TODO: optimize for some types using `castaway` crate or similar.
impl<T: fmt::Display> ToLeanString for T {
    fn to_lean_string(&self) -> LeanString {
        LeanString::from(format!("{}", self))
    }
}
