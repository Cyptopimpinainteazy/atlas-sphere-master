//! Golden tests for semantic analysis.
//!
//! These tests verify that the resolver produces expected results for various
//! X3 programs, including scope structure, symbol tables, and error detection.

use std::fs;
use std::path::PathBuf;

use x3_parser::Parser;
use x3_semantics::{Resolver, SemanticError, SemanticErrorKind};

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn get_fixtures() -> Vec<(String, String)> {
    let mut fixtures = Vec::new();
    let dir = fixtures_dir();

    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "x3").unwrap_or(false) {
                if let Ok(content) = fs::read_to_string(&path) {
                    let name = path.file_stem().unwrap().to_string_lossy().to_string();
                    fixtures.push((name, content));
                }
            }
        }
    }

    fixtures.sort_by(|a, b| a.0.cmp(&b.0));
    fixtures
}

/// Test that all valid fixtures resolve without errors.
#[test]
fn test_valid_fixtures_resolve() {
    let fixtures = get_fixtures();

    for (name, source) in &fixtures {
        // Skip error test fixtures
        if name.contains("error") {
            continue;
        }

        println!("Testing fixture: {}", name);

        let mut parser = Parser::from_source(source);
        let module = parser
            .parse_module()
            .unwrap_or_else(|e| panic!("Failed to parse {}: {:?}", name, e));

        let resolver = Resolver::new();
        let result = resolver.resolve(&module);

        match result {
            Ok(resolved) => {
                println!(
                    "  ✓ {} - {} scopes, {} symbols",
                    name,
                    resolved.scopes.len(),
                    resolved.symbols.len()
                );
            }
            Err(errors) => {
                panic!(
                    "Fixture {} should resolve without errors, got: {:?}",
                    name, errors
                );
            }
        }
    }
}

/// Test scope counts for fixtures.
#[test]
fn test_scope_structure() {
    let fixtures = get_fixtures();

    for (name, source) in &fixtures {
        if name.contains("error") {
            continue;
        }

        let mut parser = Parser::from_source(source);
        let module = parser.parse_module().expect("should parse");

        let resolver = Resolver::new();
        if let Ok(resolved) = resolver.resolve(&module) {
            // Verify we have at least a global scope
            assert!(
                resolved.scopes.len() >= 1,
                "{} should have at least global scope",
                name
            );

            // Print scope structure for debugging
            println!("Fixture {}: {} scopes", name, resolved.scopes.len());
            for scope in resolved.scopes.iter() {
                println!(
                    "  Scope {:?} - {:?} (depth {})",
                    scope.id, scope.kind, scope.depth
                );
            }
        }
    }
}

/// Test symbol table population.
#[test]
fn test_symbol_table() {
    let fixtures = get_fixtures();

    for (name, source) in &fixtures {
        if name.contains("error") {
            continue;
        }

        let mut parser = Parser::from_source(source);
        let module = parser.parse_module().expect("should parse");

        let resolver = Resolver::new();
        if let Ok(resolved) = resolver.resolve(&module) {
            println!("Fixture {}: {} symbols", name, resolved.symbols.len());
            for symbol in resolved.symbols.iter() {
                println!("  {:?}: {} ({:?})", symbol.id, symbol.name, symbol.kind);
            }
        }
    }
}

#[cfg(test)]
mod error_tests {
    use super::*;

    fn parse_and_resolve(source: &str) -> Result<x3_semantics::ResolvedModule, Vec<SemanticError>> {
        let mut parser = Parser::from_source(source);
        let module = parser.parse_module().expect("should parse");
        let resolver = Resolver::new();
        resolver.resolve(&module)
    }

    #[test]
    fn test_undefined_variable() {
        let source = r#"
            fn test() {
                return undefined_var;
            }
        "#;

        let result = parse_and_resolve(source);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert!(matches!(
            errors[0].kind,
            SemanticErrorKind::UndefinedVariable(_)
        ));
    }

    #[test]
    fn test_duplicate_function() {
        let source = r#"
            fn test() { return 1; }
            fn test() { return 2; }
        "#;

        let result = parse_and_resolve(source);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert!(matches!(
            errors[0].kind,
            SemanticErrorKind::DuplicateName { .. }
        ));
    }

    #[test]
    fn test_duplicate_param() {
        let source = r#"
            fn test(a, a) {
                return a;
            }
        "#;

        let result = parse_and_resolve(source);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert!(matches!(
            errors[0].kind,
            SemanticErrorKind::DuplicateName { .. }
        ));
    }

    #[test]
    fn test_break_outside_loop() {
        let source = r#"
            fn test() {
                break;
            }
        "#;

        let result = parse_and_resolve(source);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0].kind, SemanticErrorKind::InvalidBreak));
    }

    #[test]
    fn test_continue_outside_loop() {
        let source = r#"
            fn test() {
                continue;
            }
        "#;

        let result = parse_and_resolve(source);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0].kind, SemanticErrorKind::InvalidContinue));
    }

    #[test]
    fn test_return_outside_function() {
        let source = r#"
            let x = 1;
            return x;
        "#;

        // This should fail at parse time, but let's verify our resolver catches it if AST allows
        let mut parser = Parser::from_source(source);
        // This will likely fail to parse since return is only valid in functions
        let result = parser.parse_module();
        // If it parses, the resolver should catch it
        if let Ok(module) = result {
            let resolver = Resolver::new();
            let resolve_result = resolver.resolve(&module);
            assert!(resolve_result.is_err());
        }
    }

    #[test]
    fn test_assignment_to_immutable() {
        let source = r#"
            fn test() {
                let x = 1;
                x = 2;
                return x;
            }
        "#;

        let result = parse_and_resolve(source);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert!(matches!(
            errors[0].kind,
            SemanticErrorKind::AssignmentToImmutable(_)
        ));
    }

    #[test]
    fn test_mutable_assignment_ok() {
        let source = r#"
            fn test() {
                let mut x = 1;
                x = 2;
                return x;
            }
        "#;

        let result = parse_and_resolve(source);
        assert!(result.is_ok());
    }

    #[test]
    fn test_nested_atomic_error() {
        let source = r#"
            fn test() {
                atomic {
                    atomic {
                        let x = 1;
                    }
                }
            }
        "#;

        let result = parse_and_resolve(source);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors
            .iter()
            .any(|e| matches!(e.kind, SemanticErrorKind::NestedAtomic)));
    }

    #[test]
    fn test_forward_reference_ok() {
        let source = r#"
            fn main() {
                return helper();
            }
            fn helper() {
                return 42;
            }
        "#;

        let result = parse_and_resolve(source);
        assert!(
            result.is_ok(),
            "Forward references should work: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_shadowing_in_nested_scope_ok() {
        let source = r#"
            fn test() {
                let x = 1;
                if (x > 0) {
                    let x = 2;
                    return x;
                }
                return x;
            }
        "#;

        let result = parse_and_resolve(source);
        assert!(
            result.is_ok(),
            "Shadowing in nested scope should be allowed: {:?}",
            result.err()
        );
    }
}
