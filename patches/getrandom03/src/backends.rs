//! Backend implementation for wasm32-unknown-unknown
//! Provides stub random number generation using simple deterministic values

use core::mem::MaybeUninit;
use crate::Error;

/// Fill buffer with random bytes (stub for WASM)
#[inline]
pub fn fill_inner(dest: &mut [MaybeUninit<u8>]) -> Result<(), Error> {
    for (i, byte) in dest.iter_mut().enumerate() {
        byte.write((i as u8).wrapping_mul(17).wrapping_add(31));
    }
    Ok(())
}

/// Get a random u32 (stub for WASM)
#[inline]
pub fn inner_u32() -> Result<u32, Error> {
    Ok(0x1234_5678)
}

/// Get a random u64 (stub for WASM)
#[inline]
pub fn inner_u64() -> Result<u64, Error> {
    Ok(0x1234_5678_9ABC_DEF0)
}
