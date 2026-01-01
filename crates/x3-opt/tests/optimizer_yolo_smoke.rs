//! YOLO optimizer smoke tests.
//! Lightweight end-to-end tests for the full optimization pipeline.

#[cfg(test)]
mod tests {
    use x3_common::{Literal, Span};
    use x3_mir::{
        MirBlock, MirBlockId, MirFunction, MirModule, MirRhs, MirStatement, MirTerminator, MirValue,
    };
    use x3_opt::{count_instructions, estimate_bytes, run_yolo_once, simulate_gas};

    fn simple_test_module() -> MirModule {
        MirModule {
            functions: vec![MirFunction {
                symbol: x3_mir::SymbolId(0),
                params: vec![],
                entry: MirBlockId(0),
                blocks: vec![MirBlock {
                    id: MirBlockId(0),
                    statements: vec![
                        MirStatement {
                            target: MirValue(1),
                            rhs: MirRhs::Literal(Literal::Integer(5)),
                        },
                        MirStatement {
                            target: MirValue(2),
                            rhs: MirRhs::Literal(Literal::Integer(5)),
                        },
                    ],
                    terminator: Some(MirTerminator::Return(Some(MirValue(1)))),
                }],
                span: Span::dummy(),
            }],
            span: Span::dummy(),
        }
    }

    #[test]
    fn yolo_smoke_on_simple_function() {
        let mut module = simple_test_module();
        let before_instr = count_instructions(&module);
        let before_gas = simulate_gas(&module);

        let report = run_yolo_once(&mut module).expect("yolo failed");

        let after_instr = count_instructions(&module);
        let after_gas = simulate_gas(&module);

        assert_eq!(report.instr_before, before_instr);
        assert_eq!(report.instr_after, after_instr);
        assert_eq!(report.gas_before, before_gas);
        assert_eq!(report.gas_after, after_gas);
        assert!(
            report.per_pass.len() > 0,
            "should have run at least one pass"
        );
    }

    #[test]
    fn yolo_metrics_are_monotone() {
        let mut module = simple_test_module();
        let report = run_yolo_once(&mut module).expect("yolo failed");

        // Instr and gas should never increase (optimization, not code bloat)
        assert!(report.instr_after <= report.instr_before);
        assert!(report.gas_after <= report.gas_before);
        assert!(report.bytes_after <= report.bytes_before);
    }

    #[test]
    fn yolo_empty_module_is_safe() {
        let mut module = MirModule {
            functions: vec![],
            span: Span::dummy(),
        };
        let report = run_yolo_once(&mut module).expect("yolo failed");
        assert!(!report.changed);
        assert_eq!(report.instr_before, 0);
    }
}
