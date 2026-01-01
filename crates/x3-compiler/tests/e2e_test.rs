//! End-to-end tests for the X3 compiler pipeline
//!
//! These tests verify the full Source → Bytecode compilation path,
//! testing various language features and optimization behaviors.

use x3_compiler::{CompilationOptions, Compiler, OptLevel};

/// Helper to compile a source string and return bytecode length
fn compile_and_measure(source: &str, opt_level: OptLevel) -> (usize, usize) {
    let options = CompilationOptions {
        opt_level,
        emit_stats: true,
        ..Default::default()
    };

    let result = Compiler::compile(source, options).expect("Compilation should succeed");

    let transformations = result
        .artifacts
        .as_ref()
        .and_then(|a| a.opt_stats.as_ref())
        .map(|s| s.total_transformations)
        .unwrap_or(0);

    (result.bytecode.code.len(), transformations)
}

/// Helper to load fixture and compile
fn compile_fixture(name: &str, opt_level: OptLevel) -> (usize, usize) {
    let path = format!("{}/tests/fixtures/{}.x3", env!("CARGO_MANIFEST_DIR"), name);
    let source = std::fs::read_to_string(&path).expect(&format!("Failed to read {}", path));
    compile_and_measure(&source, opt_level)
}

#[test]
fn test_fib_compilation() {
    // Test fibonacci - should compile successfully
    let (size, _) = compile_fixture("fib", OptLevel::Default);
    assert!(size > 0, "Fibonacci should produce non-empty bytecode");

    // O3 should not increase size (optimization)
    let (size_o3, _) = compile_fixture("fib", OptLevel::Aggressive);
    assert!(size_o3 <= size, "O3 should not increase code size");
}

#[test]
fn test_match_cond_compilation() {
    // Test multiple conditionals
    let (size, _) = compile_fixture("match_cond", OptLevel::Default);
    assert!(size > 0, "match_cond should produce non-empty bytecode");
}

#[test]
fn test_branch_fold_optimization() {
    // Test that branch folding reduces code size
    let (size_o0, trans_o0) = compile_fixture("branch_fold", OptLevel::None);
    let (size_o2, trans_o2) = compile_fixture("branch_fold", OptLevel::Default);

    // O2 should apply some transformations
    assert!(
        trans_o2 > trans_o0,
        "O2 should apply more transformations than O0"
    );

    // Optimized code should be <= unoptimized
    // (constant propagation should eliminate dead branches)
    assert!(
        size_o2 <= size_o0,
        "Optimized code should not be larger: O0={}, O2={}",
        size_o0,
        size_o2
    );
}

#[test]
fn test_loop_ops_compilation() {
    // Test loop operations compile correctly
    let (size, _) = compile_fixture("loop_ops", OptLevel::Default);
    assert!(size > 0, "loop_ops should produce non-empty bytecode");
}

#[test]
fn test_optimization_levels() {
    let source = r#"
        fn main() -> i64 {
            let a = 1;
            let b = 2;
            let c = a + b;
            let d = c * 2;
            return d;
        }
    "#;

    // All optimization levels should produce valid bytecode
    let (size_o0, _) = compile_and_measure(source, OptLevel::None);
    let (size_o1, _) = compile_and_measure(source, OptLevel::Basic);
    let (size_o2, _) = compile_and_measure(source, OptLevel::Default);
    let (size_o3, _) = compile_and_measure(source, OptLevel::Aggressive);

    assert!(size_o0 > 0, "O0 should produce bytecode");
    assert!(size_o1 > 0, "O1 should produce bytecode");
    assert!(size_o2 > 0, "O2 should produce bytecode");
    assert!(size_o3 > 0, "O3 should produce bytecode");
}

#[test]
fn test_deterministic_output() {
    // Same source should produce identical bytecode
    let source = r#"
        fn main() -> i64 {
            let x = 10;
            let y = 20;
            return x + y;
        }
    "#;

    let options = CompilationOptions::opt2();

    let result1 = Compiler::compile(source, options.clone()).unwrap();
    let result2 = Compiler::compile(source, options.clone()).unwrap();

    assert_eq!(
        result1.bytecode.code, result2.bytecode.code,
        "Compilation should be deterministic"
    );
}

#[test]
fn test_empty_function() {
    let source = r#"
        fn do_nothing() {}
        fn main() -> i64 {
            do_nothing();
            return 0;
        }
    "#;

    let options = CompilationOptions::opt2();
    let result = Compiler::compile(source, options);
    assert!(result.is_ok(), "Empty function should compile");
}

#[test]
fn test_nested_conditionals() {
    let source = r#"
        fn classify(x: i64, y: i64) -> i64 {
            if x > 0 {
                if y > 0 {
                    return 1;
                } else {
                    return 2;
                }
            } else {
                if y > 0 {
                    return 3;
                } else {
                    return 4;
                }
            }
        }
        
        fn main() -> i64 {
            return classify(1, -1);
        }
    "#;

    let options = CompilationOptions::opt2();
    let result = Compiler::compile(source, options);
    assert!(result.is_ok(), "Nested conditionals should compile");
}

#[test]
fn test_complex_expressions() {
    let source = r#"
        fn compute(a: i64, b: i64, c: i64) -> i64 {
            return (a + b) * c - (a - b) * (c + 1);
        }
        
        fn main() -> i64 {
            return compute(10, 5, 3);
        }
    "#;

    let options = CompilationOptions::opt2();
    let result = Compiler::compile(source, options);
    assert!(
        result.is_ok(),
        "Complex expressions should compile: {:?}",
        result.err()
    );
}
