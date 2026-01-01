// rustix patch for WASM compatibility
// Re-export rustix with conditional compilation to avoid errno issues on WASM

#[cfg(not(target_arch = "wasm32"))]
pub use rustix::*;

#[cfg(target_arch = "wasm32")]
pub mod fs {
    // Stub implementations for WASM
    pub fn openat() -> Result<(), ()> {
        Err(())
    }
    pub fn close() -> Result<(), ()> {
        Err(())
    }
    pub fn read() -> Result<(), ()> {
        Err(())
    }
    pub fn write() -> Result<(), ()> {
        Err(())
    }
}

#[cfg(target_arch = "wasm32")]
pub mod io {
    // Stub implementations for WASM
    pub fn dup2_stdin() -> Result<(), ()> {
        Err(())
    }
    pub fn dup2_stdout() -> Result<(), ()> {
        Err(())
    }
    pub fn dup2_stderr() -> Result<(), ()> {
        Err(())
    }
}

#[cfg(target_arch = "wasm32")]
pub mod process {
    // Stub implementations for WASM
    pub fn getpid() -> u32 {
        1
    }
    pub fn getppid() -> u32 {
        0
    }
}
