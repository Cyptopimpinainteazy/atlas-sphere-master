//! Integration tests for x3-verifier

use x3_common::{Literal, Span};
use x3_mir::{
    MirBlock, MirBlockId, MirFunction, MirModule, MirRhs, MirStatement, MirTerminator, MirValue,
    SymbolId,
};
use x3_verifier::{SafetyRules, Verifier};

fn empty_span() -> Span {
    Span::new(0, 0)
}

fn make_simple_function(_name: &str, statements: Vec<MirStatement>) -> MirFunction {
    MirFunction {
        symbol: SymbolId(1),
        params: vec![],
        entry: MirBlockId(0),
        blocks: vec![MirBlock {
            id: MirBlockId(0),
            statements,
            terminator: Some(MirTerminator::Return(None)),
        }],
        span: empty_span(),
    }
}

#[test]
fn test_verify_empty_module() {
    let verifier = Verifier::default();
    let module = MirModule {
        functions: vec![],
        span: empty_span(),
    };

    let report = verifier.verify_mir(&module).unwrap();
    assert!(report.passed());
    assert_eq!(report.stats.functions_checked, 0);
}

#[test]
fn test_verify_simple_function() {
    let verifier = Verifier::default();

    let func = make_simple_function(
        "test",
        vec![MirStatement {
            target: MirValue(0),
            rhs: MirRhs::Literal(Literal::Integer(42)),
        }],
    );

    let module = MirModule {
        functions: vec![func],
        span: empty_span(),
    };

    let report = verifier.verify_mir(&module).unwrap();
    assert!(report.passed());
    assert_eq!(report.stats.functions_checked, 1);
    assert_eq!(report.stats.statements_checked, 1);
}

#[test]
fn test_gas_analysis() {
    let verifier = Verifier::default();

    // Create a function with multiple operations
    let func = make_simple_function(
        "compute",
        vec![
            MirStatement {
                target: MirValue(0),
                rhs: MirRhs::Literal(Literal::Integer(1)),
            },
            MirStatement {
                target: MirValue(1),
                rhs: MirRhs::Literal(Literal::Integer(2)),
            },
            MirStatement {
                target: MirValue(2),
                rhs: MirRhs::Binary(x3_ast::BinaryOp::Add, MirValue(0), MirValue(1)),
            },
        ],
    );

    let module = MirModule {
        functions: vec![func],
        span: empty_span(),
    };

    let report = verifier.verify_mir(&module).unwrap();
    assert!(report.passed());

    let gas_report = report.gas_report.unwrap();
    assert_eq!(gas_report.functions.len(), 1);
    assert!(gas_report.functions[0].max_gas > 0);
}

#[test]
fn test_safety_rules_default() {
    let rules = SafetyRules::default();

    // Check safe operations
    assert!(rules.is_safe("add"));
    assert!(rules.is_safe("sub"));
    assert!(rules.is_safe("mul"));

    // Check restricted operations
    assert!(rules.is_restricted("sstore"));

    // Check forbidden operations
    assert!(rules.is_forbidden("selfdestruct"));
}

#[test]
fn test_safety_rules_from_yaml() {
    let yaml = r#"
version: "1.0"
opcodes:
  custom_op: safe
  dangerous_op: forbidden
limits:
  max_function_gas: 1000000
"#;

    let rules = SafetyRules::from_yaml(yaml).unwrap();
    assert!(rules.is_safe("custom_op"));
    assert!(rules.is_forbidden("dangerous_op"));
    assert_eq!(rules.limits.max_function_gas, 1000000);
}

#[test]
fn test_verification_report_filtering() {
    let verifier = Verifier::default();
    let module = MirModule {
        functions: vec![],
        span: empty_span(),
    };

    let report = verifier.verify_mir(&module).unwrap();

    // Empty module should have no errors or warnings
    assert_eq!(report.errors().count(), 0);
    assert_eq!(report.warnings().count(), 0);
    assert_eq!(report.critical().count(), 0);
}

#[test]
fn test_instruction_limit_check() {
    let mut rules = SafetyRules::default();
    rules.limits.max_instructions_per_function = 2; // Very low limit

    let verifier = Verifier::new(rules);

    // Create function with 3 instructions (exceeds limit)
    let func = make_simple_function(
        "too_big",
        vec![
            MirStatement {
                target: MirValue(0),
                rhs: MirRhs::Literal(Literal::Integer(1)),
            },
            MirStatement {
                target: MirValue(1),
                rhs: MirRhs::Literal(Literal::Integer(2)),
            },
            MirStatement {
                target: MirValue(2),
                rhs: MirRhs::Literal(Literal::Integer(3)),
            },
        ],
    );

    let module = MirModule {
        functions: vec![func],
        span: empty_span(),
    };

    let report = verifier.verify_mir(&module).unwrap();

    // Should have an error about instruction limit
    assert!(!report.passed());
    assert!(report.errors.iter().any(|e| e.code == "E009"));
}
