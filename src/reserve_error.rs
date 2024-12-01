use core::{error::Error, fmt};

/// A possible error value if allocating or resizing a [`SmallString`] failed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReserveError;

impl fmt::Display for ReserveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Cannot allocate memory to hold SmallString")
    }
}

impl Error for ReserveError {}
