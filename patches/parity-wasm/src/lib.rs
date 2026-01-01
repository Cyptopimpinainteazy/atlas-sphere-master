// Minimal parity-wasm stub for WASM builds
// This is a local patch to ensure WASM compatibility

#[derive(Debug, Clone)]
pub struct Module;

impl Module {
    pub fn new() -> Self {
        Module
    }
}

pub mod elements {
    #[derive(Debug, Clone)]
    pub enum Section {}
    
    pub use super::Module;
}

pub mod builder {
    use crate::Module;

    pub struct ModuleBuilder;

    impl ModuleBuilder {
        pub fn new() -> Self {
            ModuleBuilder
        }

        pub fn build(self) -> Module {
            Module::new()
        }
    }
}

pub use elements::{Module as ElementsModule, Section};
pub use builder::ModuleBuilder;
