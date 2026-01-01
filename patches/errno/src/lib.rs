//! Cross-platform interface to the POSIX `errno` variable.
//! This is a patched version that provides compatibility with WASM targets.

#![cfg_attr(target_arch = "wasm32", no_std)]

#[cfg(not(target_arch = "wasm32"))]
use core::fmt;

#[cfg(target_arch = "wasm32")]
use core::fmt;

/// Wraps a platform-specific error code.
/// The inner field is public to match the expected API from rustix.
#[derive(Copy, Clone, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct Errno(pub i32);

impl Errno {
    /// Returns the platform-specific error number.
    pub fn into_raw(self) -> i32 {
        self.0
    }

    /// Creates a new `Errno` from a raw error code.
    pub const fn from_raw(code: i32) -> Errno {
        Errno(code)
    }
}

impl fmt::Debug for Errno {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Errno").field(&self.0).finish()
    }
}

impl fmt::Display for Errno {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "errno {}", self.0)
    }
}

impl From<Errno> for i32 {
    fn from(err: Errno) -> Self {
        err.0
    }
}

// Unix implementation
#[cfg(all(not(target_arch = "wasm32"), unix))]
mod unix {
    use super::Errno;
    use libc::c_int;

    extern "C" {
        #[cfg_attr(target_os = "linux", link_name = "__errno_location")]
        #[cfg_attr(target_os = "macos", link_name = "__error")]
        #[cfg_attr(target_os = "freebsd", link_name = "__error")]
        #[cfg_attr(target_os = "netbsd", link_name = "__errno")]
        #[cfg_attr(target_os = "openbsd", link_name = "__errno")]
        #[cfg_attr(target_os = "android", link_name = "__errno")]
        fn errno_location() -> *mut c_int;
    }

    /// Returns the platform-specific value of `errno`.
    pub fn errno() -> Errno {
        unsafe { Errno(*errno_location()) }
    }

    /// Sets the value of `errno`.
    pub fn set_errno(Errno(errno): Errno) {
        unsafe {
            *errno_location() = errno;
        }
    }
}

#[cfg(all(not(target_arch = "wasm32"), unix))]
pub use self::unix::{errno, set_errno};

// Windows implementation
#[cfg(all(not(target_arch = "wasm32"), windows))]
mod windows {
    use super::Errno;

    extern "system" {
        fn GetLastError() -> u32;
        fn SetLastError(dwErrCode: u32);
    }

    /// Returns the platform-specific value of the last error.
    pub fn errno() -> Errno {
        unsafe { Errno(GetLastError() as i32) }
    }

    /// Sets the value of the last error.
    pub fn set_errno(Errno(errno): Errno) {
        unsafe {
            SetLastError(errno as u32);
        }
    }
}

#[cfg(all(not(target_arch = "wasm32"), windows))]
pub use self::windows::{errno, set_errno};

// WASM stub implementation
#[cfg(target_arch = "wasm32")]
mod wasm {
    use super::Errno;

    /// Returns a stub errno value for WASM (always 0).
    pub fn errno() -> Errno {
        Errno(0)
    }

    /// No-op for WASM targets.
    pub fn set_errno(_errno: Errno) {}
}

#[cfg(target_arch = "wasm32")]
pub use self::wasm::{errno, set_errno};
