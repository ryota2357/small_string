use core::{error::Error, fmt};

use super::ReserveError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToLeanStringError {
    Reserve(ReserveError),
    Fmt(fmt::Error),
}

impl Error for ToLeanStringError {}

impl fmt::Display for ToLeanStringError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ToLeanStringError::Reserve(e) => e.fmt(f),
            ToLeanStringError::Fmt(e) => e.fmt(f),
        }
    }
}

impl From<ReserveError> for ToLeanStringError {
    fn from(value: ReserveError) -> Self {
        ToLeanStringError::Reserve(value)
    }
}

impl From<fmt::Error> for ToLeanStringError {
    fn from(value: fmt::Error) -> Self {
        ToLeanStringError::Fmt(value)
    }
}
