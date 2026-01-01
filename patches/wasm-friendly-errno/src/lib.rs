#![cfg_attr(not(feature = "std"), no_std)]

// Minimal wasm-friendly implementation of `errno` crate API surface required
// by downstream dependencies. This avoids compile_error! on wasm32 targets.

/// Opaque errno wrapper
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Errno(pub i32);

impl Errno {
    pub fn as_i32(&self) -> i32 {
        self.0
    }
}

/// Return a zero Errno on wasm targets
pub fn errno() -> Errno {
    Errno(0)
}

/// Set errno (no-op on wasm)
pub fn set_errno(_: Errno) {}

// Common constants
pub const ENOENT: i32 = 2;
pub const EINVAL: i32 = 22;
