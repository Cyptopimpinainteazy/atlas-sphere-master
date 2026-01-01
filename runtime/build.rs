use substrate_wasm_builder::WasmBuilder;

fn main() {
    // Use the canonical Substrate wasm builder (keeps parity-wasm, wasm-opt handling consistent)
    WasmBuilder::new()
        .with_current_project()
        .export_heap_base()
        .import_memory()
        .build();
}
