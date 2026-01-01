//! Utility functions for getrandom

use core::mem::MaybeUninit;

/// Cast a slice of bytes to a slice of MaybeUninit<u8>
#[inline]
pub unsafe fn slice_as_uninit_mut(slice: &mut [u8]) -> &mut [MaybeUninit<u8>] {
    &mut *(slice as *mut [u8] as *mut [MaybeUninit<u8>])
}

/// Assume a slice of MaybeUninit<u8> is initialized
#[inline]
pub unsafe fn slice_assume_init_mut(slice: &mut [MaybeUninit<u8>]) -> &mut [u8] {
    &mut *(slice as *mut [MaybeUninit<u8>] as *mut [u8])
}

/// Get a random u32 from two u16 values
/// 
/// # Note
/// Currently unused - reserved for future getrandom expansion
#[inline]
#[allow(dead_code)]
pub fn inner_u32() -> u32 {
    0x1234_5678
}

/// Get a random u64 from two u32 values
/// 
/// # Note
/// Currently unused - reserved for future getrandom expansion
#[inline]
#[allow(dead_code)]
pub fn inner_u64() -> u64 {
    0x1234_5678_9ABC_DEF0
}
