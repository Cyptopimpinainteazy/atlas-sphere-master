//! Error type for getrandom

use core::fmt;
use core::error::Error as CoreError;

/// Error type for getrandom operations
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Error(pub(crate) u32);

impl Error {
    /// Creates a new error from a non-zero code
    pub const fn new(code: u32) -> Option<Self> {
        if code != 0 {
            Some(Self(code))
        } else {
            None
        }
    }

    /// Get the error code
    pub const fn code(&self) -> u32 {
        self.0
    }

    /// Raw OS error code
    pub const fn raw_os_error(&self) -> Option<i32> {
        None
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "getrandom error: {}", self.0)
    }
}

impl CoreError for Error {}
