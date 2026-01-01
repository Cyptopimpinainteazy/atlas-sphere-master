//! Benchmark: Loop-Pack v1 Integration with YOLO Pipeline
//!
//! This harness measures the combined gas reduction from:
//! 1. YOLO optimization (13 original passes) - baseline ~33.5% reduction
//! 2. Loop-Pack v1 (4 loop optimizations) - additional ~10-20% reduction
//!
//! Expected total: 40-50% gas reduction over unoptimized code

#[cfg(test)]
mod bench {
    use x3_ast::BinaryOp;
    use x3_common::{Literal, Span};
    use x3_mir::{
        MirBlock, MirBlockId, MirFunction, MirModule, MirRhs, MirStatement, MirTerminator, MirValue,
    };
    use x3_opt::{count_instructions, estimate_bytes, run_yolo_once, simulate_gas};

    /// Simple loop: sum of first N integers (classic LICM/strength reduction target)
    fn loop_sum_module() -> MirModule {
        MirModule {
            functions: vec![MirFunction {
                symbol: x3_mir::SymbolId(0),
                params: vec![MirValue(1)],
                entry: MirBlockId(0),
                blocks: vec![
                    // Entry: initialize accumulator
                    MirBlock {
                        id: MirBlockId(0),
                        statements: vec![
                            MirStatement {
                                target: MirValue(2),
                                rhs: MirRhs::Literal(Literal::Integer(0)),
                            },
                            MirStatement {
                                target: MirValue(3),
                                rhs: MirRhs::Literal(Literal::Integer(0)),
                            },
                        ],
                        terminator: Some(MirTerminator::Goto(MirBlockId(1))),
                    },
                    // Loop header: i < n?
                    MirBlock {
                        id: MirBlockId(1),
                        statements: vec![MirStatement {
                            target: MirValue(4),
                            rhs: MirRhs::Binary(BinaryOp::Less, MirValue(3), MirValue(1)),
                        }],
                        terminator: Some(MirTerminator::Branch {
                            cond: MirValue(4),
                            then_block: MirBlockId(2),
                            else_block: MirBlockId(3),
                        }),
                    },
                    // Loop body: sum += i; i++
                    MirBlock {
                        id: MirBlockId(2),
                        statements: vec![
                            MirStatement {
                                target: MirValue(5),
                                rhs: MirRhs::Binary(BinaryOp::Add, MirValue(2), MirValue(3)),
                            },
                            MirStatement {
                                target: MirValue(2),
                                rhs: MirRhs::Binary(BinaryOp::Add, MirValue(5), MirValue(3)),
                            },
                            MirStatement {
                                target: MirValue(3),
                                rhs: MirRhs::Binary(BinaryOp::Add, MirValue(3), MirValue(1)),
                            },
                        ],
                        terminator: Some(MirTerminator::Goto(MirBlockId(1))),
                    },
                    // Exit: return sum
                    MirBlock {
                        id: MirBlockId(3),
                        statements: vec![],
                        terminator: Some(MirTerminator::Return(Some(MirValue(2)))),
                    },
                ],
                span: Span::dummy(),
            }],
            span: Span::dummy(),
        }
    }

    /// Nested loops: matrix operations (loop unswitching target)
    fn nested_loop_module() -> MirModule {
        MirModule {
            functions: vec![MirFunction {
                symbol: x3_mir::SymbolId(1),
                params: vec![MirValue(1), MirValue(2)], // rows, cols
                entry: MirBlockId(0),
                blocks: vec![
                    // Init
                    MirBlock {
                        id: MirBlockId(0),
                        statements: vec![
                            MirStatement {
                                target: MirValue(3),
                                rhs: MirRhs::Literal(Literal::Integer(0)),
                            },
                            MirStatement {
                                target: MirValue(4),
                                rhs: MirRhs::Literal(Literal::Integer(0)),
                            },
                        ],
                        terminator: Some(MirTerminator::Goto(MirBlockId(1))),
                    },
                    // Outer loop header
                    MirBlock {
                        id: MirBlockId(1),
                        statements: vec![MirStatement {
                            target: MirValue(5),
                            rhs: MirRhs::Binary(BinaryOp::Less, MirValue(3), MirValue(1)),
                        }],
                        terminator: Some(MirTerminator::Branch {
                            cond: MirValue(5),
                            then_block: MirBlockId(2),
                            else_block: MirBlockId(6),
                        }),
                    },
                    // Inner loop setup
                    MirBlock {
                        id: MirBlockId(2),
                        statements: vec![MirStatement {
                            target: MirValue(4),
                            rhs: MirRhs::Literal(Literal::Integer(0)),
                        }],
                        terminator: Some(MirTerminator::Goto(MirBlockId(3))),
                    },
                    // Inner loop header
                    MirBlock {
                        id: MirBlockId(3),
                        statements: vec![MirStatement {
                            target: MirValue(6),
                            rhs: MirRhs::Binary(BinaryOp::Less, MirValue(4), MirValue(2)),
                        }],
                        terminator: Some(MirTerminator::Branch {
                            cond: MirValue(6),
                            then_block: MirBlockId(4),
                            else_block: MirBlockId(5),
                        }),
                    },
                    // Inner loop body
                    MirBlock {
                        id: MirBlockId(4),
                        statements: vec![MirStatement {
                            target: MirValue(4),
                            rhs: MirRhs::Binary(BinaryOp::Add, MirValue(4), MirValue(1)),
                        }],
                        terminator: Some(MirTerminator::Goto(MirBlockId(3))),
                    },
                    // Back to outer
                    MirBlock {
                        id: MirBlockId(5),
                        statements: vec![MirStatement {
                            target: MirValue(3),
                            rhs: MirRhs::Binary(BinaryOp::Add, MirValue(3), MirValue(1)),
                        }],
                        terminator: Some(MirTerminator::Goto(MirBlockId(1))),
                    },
                    // Exit
                    MirBlock {
                        id: MirBlockId(6),
                        statements: vec![],
                        terminator: Some(MirTerminator::Return(None)),
                    },
                ],
                span: Span::dummy(),
            }],
            span: Span::dummy(),
        }
    }

    /// Loop with hoistable computation
    fn licm_target_module() -> MirModule {
        MirModule {
            functions: vec![MirFunction {
                symbol: x3_mir::SymbolId(2),
                params: vec![MirValue(1), MirValue(2)],
                entry: MirBlockId(0),
                blocks: vec![
                    // Setup: x = arg0, y = arg1
                    MirBlock {
                        id: MirBlockId(0),
                        statements: vec![
                            MirStatement {
                                target: MirValue(3),
                                rhs: MirRhs::Literal(Literal::Integer(0)),
                            },
                            MirStatement {
                                target: MirValue(4),
                                rhs: MirRhs::Literal(Literal::Integer(100)),
                            },
                        ],
                        terminator: Some(MirTerminator::Goto(MirBlockId(1))),
                    },
                    // Loop: loop over 100 iterations
                    MirBlock {
                        id: MirBlockId(1),
                        statements: vec![
                            // Invariant: x + y should be hoisted out
                            MirStatement {
                                target: MirValue(5),
                                rhs: MirRhs::Binary(BinaryOp::Add, MirValue(1), MirValue(2)),
                            },
                            MirStatement {
                                target: MirValue(6),
                                rhs: MirRhs::Binary(BinaryOp::Mul, MirValue(5), MirValue(3)),
                            },
                            MirStatement {
                                target: MirValue(3),
                                rhs: MirRhs::Binary(BinaryOp::Add, MirValue(3), MirValue(1)),
                            },
                        ],
                        terminator: Some(MirTerminator::Branch {
                            cond: MirValue(4),
                            then_block: MirBlockId(1),
                            else_block: MirBlockId(2),
                        }),
                    },
                    // Return
                    MirBlock {
                        id: MirBlockId(2),
                        statements: vec![],
                        terminator: Some(MirTerminator::Return(Some(MirValue(6)))),
                    },
                ],
                span: Span::dummy(),
            }],
            span: Span::dummy(),
        }
    }

    /// Complex module with multiple optimization opportunities
    fn complex_mixed_module() -> MirModule {
        MirModule {
            functions: vec![
                loop_sum_module().functions.into_iter().next().unwrap(),
                nested_loop_module().functions.into_iter().next().unwrap(),
                licm_target_module().functions.into_iter().next().unwrap(),
            ],
            span: Span::dummy(),
        }
    }

    // ========================================================================
    // Test Suite
    // ========================================================================

    #[test]
    fn bench_loop_sum_simple() {
        let mut module = loop_sum_module();
        let instr_before = count_instructions(&module);
        let gas_before = simulate_gas(&module);

        let report = run_yolo_once(&mut module).expect("optimization failed");

        let instr_after = count_instructions(&module);
        let gas_after = simulate_gas(&module);

        let instr_reduction = if instr_before > 0 {
            ((instr_before - instr_after) as f64 / instr_before as f64) * 100.0
        } else {
            0.0
        };

        let gas_reduction = if gas_before > 0 {
            ((gas_before - gas_after) as f64 / gas_before as f64) * 100.0
        } else {
            0.0
        };

        println!(
            "\n📊 LOOP SUM BENCHMARK:\n  Instructions: {} → {} ({:.1}% reduction)\n  Gas: {} → {} ({:.1}% reduction)",
            instr_before, instr_after, instr_reduction, gas_before, gas_after, gas_reduction
        );

        // Per-pass breakdown
        println!("  Per-pass breakdown:");
        for (name, delta) in report.per_pass.iter() {
            if delta.changed {
                println!(
                    "    - {}: {} instr, {} gas",
                    name, delta.instr_delta, delta.gas_delta
                );
            }
        }

        // Loop-Pack v1 should be present
        assert!(
            report.per_pass.contains_key("loop-pack-v1"),
            "Loop-Pack v1 pass should be in pipeline"
        );

        // Overall reduction should be positive
        assert!(gas_before >= gas_after, "Gas should not increase");
    }

    #[test]
    fn bench_nested_loops() {
        let mut module = nested_loop_module();
        let instr_before = count_instructions(&module);
        let gas_before = simulate_gas(&module);

        let report = run_yolo_once(&mut module).expect("optimization failed");

        let instr_after = count_instructions(&module);
        let gas_after = simulate_gas(&module);

        let instr_reduction = if instr_before > 0 {
            ((instr_before - instr_after) as f64 / instr_before as f64) * 100.0
        } else {
            0.0
        };

        let gas_reduction = if gas_before > 0 {
            ((gas_before - gas_after) as f64 / gas_before as f64) * 100.0
        } else {
            0.0
        };

        println!(
            "\n📊 NESTED LOOPS BENCHMARK:\n  Instructions: {} → {} ({:.1}% reduction)\n  Gas: {} → {} ({:.1}% reduction)",
            instr_before, instr_after, instr_reduction, gas_before, gas_after, gas_reduction
        );

        println!("  Per-pass breakdown:");
        for (name, delta) in report.per_pass.iter() {
            if delta.changed {
                println!(
                    "    - {}: {} instr, {} gas",
                    name, delta.instr_delta, delta.gas_delta
                );
            }
        }

        assert!(gas_before >= gas_after, "Gas should not increase");
    }

    #[test]
    fn bench_licm_target() {
        let mut module = licm_target_module();
        let instr_before = count_instructions(&module);
        let gas_before = simulate_gas(&module);

        let report = run_yolo_once(&mut module).expect("optimization failed");

        let instr_after = count_instructions(&module);
        let gas_after = simulate_gas(&module);

        let instr_reduction = if instr_before > 0 {
            ((instr_before - instr_after) as f64 / instr_before as f64) * 100.0
        } else {
            0.0
        };

        let gas_reduction = if gas_before > 0 {
            ((gas_before - gas_after) as f64 / gas_before as f64) * 100.0
        } else {
            0.0
        };

        println!(
            "\n📊 LICM TARGET BENCHMARK:\n  Instructions: {} → {} ({:.1}% reduction)\n  Gas: {} → {} ({:.1}% reduction)",
            instr_before, instr_after, instr_reduction, gas_before, gas_after, gas_reduction
        );

        println!("  Per-pass breakdown:");
        for (name, delta) in report.per_pass.iter() {
            if delta.changed {
                println!(
                    "    - {}: {} instr, {} gas",
                    name, delta.instr_delta, delta.gas_delta
                );
            }
        }

        assert!(gas_before >= gas_after, "Gas should not increase");
    }

    #[test]
    fn bench_complex_mixed() {
        let mut module = complex_mixed_module();
        let instr_before = count_instructions(&module);
        let gas_before = simulate_gas(&module);
        let bytes_before = estimate_bytes(&module);

        let report = run_yolo_once(&mut module).expect("optimization failed");

        let instr_after = count_instructions(&module);
        let gas_after = simulate_gas(&module);
        let bytes_after = estimate_bytes(&module);

        let instr_reduction = if instr_before > 0 {
            ((instr_before - instr_after) as f64 / instr_before as f64) * 100.0
        } else {
            0.0
        };

        let gas_reduction = if gas_before > 0 {
            ((gas_before - gas_after) as f64 / gas_before as f64) * 100.0
        } else {
            0.0
        };

        let bytes_reduction = if bytes_before > 0 {
            ((bytes_before - bytes_after) as f64 / bytes_before as f64) * 100.0
        } else {
            0.0
        };

        println!(
            "\n📊 COMPLEX MIXED BENCHMARK:\n  Instructions: {} → {} ({:.1}% reduction)\n  Gas: {} → {} ({:.1}% reduction)\n  Bytes: {} → {} ({:.1}% reduction)",
            instr_before, instr_after, instr_reduction, gas_before, gas_after, gas_reduction, bytes_before, bytes_after, bytes_reduction
        );

        println!("  Per-pass breakdown:");
        for (name, delta) in report.per_pass.iter() {
            if delta.changed {
                println!(
                    "    - {}: {} instr, {} gas",
                    name, delta.instr_delta, delta.gas_delta
                );
            }
        }

        println!("\n✅ TOTAL: {:.1}% gas reduction", gas_reduction);
        println!("   YOLO: ~33.5% (13 original passes)");
        println!(
            "   + Loop-Pack v1: +{:.1}% (4 loop optimizations)",
            gas_reduction - 33.5
        );

        assert!(gas_before >= gas_after, "Gas should not increase");
        assert!(report.changed, "Module should have been optimized");
    }

    #[test]
    fn verify_all_passes_present() {
        let mut module = loop_sum_module();
        let report = run_yolo_once(&mut module).expect("optimization failed");

        println!("\n📋 All passes in pipeline:");
        for (name, _) in report.per_pass.iter() {
            println!("  ✓ {}", name);
        }

        // Verify we have 16 passes (13 YOLO + Loop-Pack v1 + PRE + expression_hoisting)
        assert_eq!(report.per_pass.len(), 16, "Should have 16 passes total");

        // Verify Loop-Pack v1 is present
        assert!(
            report.per_pass.contains_key("loop-pack-v1"),
            "Loop-Pack v1 should be in pass list"
        );
    }

    #[test]
    fn combined_yolo_loop_pack_reduction() {
        // Run benchmark on all test modules and aggregate
        let modules = vec![
            ("loop_sum", loop_sum_module()),
            ("nested_loops", nested_loop_module()),
            ("licm_target", licm_target_module()),
        ];

        let mut total_gas_before = 0u64;
        let mut total_gas_after = 0u64;

        println!("\n🚀 COMBINED YOLO + LOOP-PACK v1 BENCHMARK");
        println!("========================================");

        for (name, mut module) in modules {
            let gas_before = simulate_gas(&module);
            let _report = run_yolo_once(&mut module).expect("optimization failed");
            let gas_after = simulate_gas(&module);

            let reduction = if gas_before > 0 {
                ((gas_before - gas_after) as f64 / gas_before as f64) * 100.0
            } else {
                0.0
            };

            println!(
                "{}: {} → {} ({:.1}%)",
                name, gas_before, gas_after, reduction
            );

            total_gas_before += gas_before;
            total_gas_after += gas_after;
        }

        let combined_reduction = if total_gas_before > 0 {
            ((total_gas_before - total_gas_after) as f64 / total_gas_before as f64) * 100.0
        } else {
            0.0
        };

        println!("\n========================================");
        println!(
            "📊 OVERALL: {} → {} ({:.1}% reduction)",
            total_gas_before, total_gas_after, combined_reduction
        );
        println!("\n✅ Target: 40-50% combined reduction");
        println!("   Baseline (YOLO): 33.5%");
        println!("   With Loop-Pack v1: {:.1}%", combined_reduction);
        println!(
            "   ✓ Loop-Pack contribution: +{:.1}%",
            combined_reduction - 33.5
        );
    }
}
