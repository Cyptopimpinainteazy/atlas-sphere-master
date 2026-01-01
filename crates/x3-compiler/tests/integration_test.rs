//! Integration test for the optimization pipeline

#[cfg(test)]
mod integration_tests {
    use x3_compiler::{CompilationOptions, Compiler, OptLevel};
    use x3_mir::MirModule;

    /// Test that the compiler can handle no optimization
    #[test]
    fn test_compile_no_opt() {
        // Create a simple MIR module
        let mir = create_simple_mir();
        let opts = CompilationOptions::no_opt();

        // Should compile without optimization
        match Compiler::compile_mir(&mir, opts) {
            Ok(_) => {
                println!("✓ No-opt compilation succeeded");
            }
            Err(e) => {
                panic!("Compilation failed: {:?}", e);
            }
        }
    }

    /// Test that the compiler can handle default optimization (O2)
    #[test]
    fn test_compile_opt2() {
        let mir = create_simple_mir();
        let opts = CompilationOptions::opt2().with_verbose(true);

        match Compiler::compile_mir(&mir, opts) {
            Ok(_) => {
                println!("✓ O2 optimization compilation succeeded");
            }
            Err(e) => {
                panic!("Compilation failed: {:?}", e);
            }
        }
    }

    /// Test that the compiler can handle aggressive optimization (O3)
    #[test]
    fn test_compile_opt3() {
        let mir = create_simple_mir();
        let opts = CompilationOptions::opt3().with_verbose(true);

        match Compiler::compile_mir(&mir, opts) {
            Ok(_) => {
                println!("✓ O3 optimization compilation succeeded");
            }
            Err(e) => {
                panic!("Compilation failed: {:?}", e);
            }
        }
    }

    /// Create a simple MIR module for testing
    fn create_simple_mir() -> MirModule {
        // This would be a real MIR module in production
        // For now, we're just testing the compilation pipeline structure
        MirModule {
            functions: vec![],
            span: x3_common::Span::dummy(),
        }
    }
}
