//! Patched getrandom v0.3.4 for Substrate WASM runtime builds.
//! Provides stub implementation for wasm32-unknown-unknown target.

#![no_std]

use core::mem::MaybeUninit;

mod backends;
mod error;
mod util;

pub use crate::error::Error;

/// Fill `dest` with random bytes from the system's preferred random number source.
#[inline]
pub fn fill(dest: &mut [u8]) -> Result<(), Error> {
    fill_uninit(unsafe { util::slice_as_uninit_mut(dest) })?;
    Ok(())
}

/// Fill potentially uninitialized buffer `dest` with random bytes.
#[inline]
pub fn fill_uninit(dest: &mut [MaybeUninit<u8>]) -> Result<&mut [u8], Error> {
    if !dest.is_empty() {
        backends::fill_inner(dest)?;
    }
    Ok(unsafe { util::slice_assume_init_mut(dest) })
}

/// Get random `u32` from the system's preferred random number source.
#[inline]
pub fn u32() -> Result<u32, Error> {
    backends::inner_u32()
}

/// Get random `u64` from the system's preferred random number source.
#[inline]
pub fn u64() -> Result<u64, Error> {
    backends::inner_u64()
}
