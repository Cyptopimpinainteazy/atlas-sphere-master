// crates/tree-sitter-x3/lib.rs
// Tree-sitter bindings for X3 language

use tree_sitter::Language;

extern "C" {
    fn tree_sitter_x3() -> Language;
}

pub fn language() -> Language {
    unsafe { tree_sitter_x3() }
}

pub const NODE_TYPES: &[u8] = include_bytes!("node-types.json");
