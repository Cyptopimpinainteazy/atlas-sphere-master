//! Diagnostics provider for Atlas files.

use crate::document::DocumentStore;
use lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range, Url};
use regex::Regex;
use std::sync::Arc;

/// Provides diagnostics for Atlas files.
pub struct DiagnosticsProvider {
    documents: Arc<DocumentStore>,
}

impl DiagnosticsProvider {
    pub fn new(documents: Arc<DocumentStore>) -> Self {
        Self { documents }
    }

    /// Generate diagnostics for a document.
    pub async fn diagnose(&self, uri: &Url) -> Vec<Diagnostic> {
        let doc = match self.documents.get(uri) {
            Some(d) => d,
            None => return vec![],
        };

        let content = doc.text();
        let mut diagnostics = Vec::new();

        if uri.path().ends_with(".comit") || uri.path().ends_with(".atlas") {
            diagnostics.extend(self.diagnose_comit(&content));
        } else if uri.path().ends_with(".rs") {
            diagnostics.extend(self.diagnose_rust(&content));
        }

        diagnostics
    }

    /// Diagnose .comit files.
    fn diagnose_comit(&self, content: &str) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Check for balanced braces
        let open_braces = content.matches('{').count();
        let close_braces = content.matches('}').count();
        if open_braces != close_braces {
            diagnostics.push(Diagnostic {
                range: Range {
                    start: Position {
                        line: 0,
                        character: 0,
                    },
                    end: Position {
                        line: 0,
                        character: 0,
                    },
                },
                severity: Some(DiagnosticSeverity::ERROR),
                code: Some(lsp_types::NumberOrString::String("E001".to_string())),
                source: Some("atlas-lsp".to_string()),
                message: format!(
                    "Unbalanced braces: {} open, {} close",
                    open_braces, close_braces
                ),
                ..Default::default()
            });
        }

        // Check for comit blocks
        let comit_re = Regex::new(r#"comit\s+"[^"]+"\s*\{"#).unwrap();
        let has_comit = comit_re.is_match(content);

        if !has_comit && !content.trim().is_empty() {
            diagnostics.push(Diagnostic {
                range: Range {
                    start: Position {
                        line: 0,
                        character: 0,
                    },
                    end: Position {
                        line: 0,
                        character: 0,
                    },
                },
                severity: Some(DiagnosticSeverity::WARNING),
                code: Some(lsp_types::NumberOrString::String("W001".to_string())),
                source: Some("atlas-lsp".to_string()),
                message: "No comit transaction defined. Expected: comit \"name\" { ... }"
                    .to_string(),
                ..Default::default()
            });
        }

        // Check for EVM/SVM blocks in comits
        for (line_num, line) in content.lines().enumerate() {
            // Check for invalid gas limits
            if let Some(caps) = Regex::new(r"gas_limit:\s*(\d+)").unwrap().captures(line) {
                if let Ok(gas) = caps[1].parse::<u64>() {
                    if gas > 30_000_000 {
                        diagnostics.push(Diagnostic {
                            range: Range {
                                start: Position {
                                    line: line_num as u32,
                                    character: 0,
                                },
                                end: Position {
                                    line: line_num as u32,
                                    character: line.len() as u32,
                                },
                            },
                            severity: Some(DiagnosticSeverity::WARNING),
                            code: Some(lsp_types::NumberOrString::String("W002".to_string())),
                            source: Some("atlas-lsp".to_string()),
                            message: format!("Gas limit {} exceeds typical block gas limit", gas),
                            ..Default::default()
                        });
                    }
                }
            }

            // Check for invalid compute units
            if let Some(caps) = Regex::new(r"compute_units:\s*(\d+)")
                .unwrap()
                .captures(line)
            {
                if let Ok(cu) = caps[1].parse::<u64>() {
                    if cu > 1_400_000 {
                        diagnostics.push(Diagnostic {
                            range: Range {
                                start: Position {
                                    line: line_num as u32,
                                    character: 0,
                                },
                                end: Position {
                                    line: line_num as u32,
                                    character: line.len() as u32,
                                },
                            },
                            severity: Some(DiagnosticSeverity::WARNING),
                            code: Some(lsp_types::NumberOrString::String("W003".to_string())),
                            source: Some("atlas-lsp".to_string()),
                            message: format!(
                                "Compute units {} exceeds max per transaction (1.4M)",
                                cu
                            ),
                            ..Default::default()
                        });
                    }
                }
            }

            // Check for invalid addresses
            let addr_re = Regex::new(r#"contract:\s*"(0x[a-fA-F0-9]*)""#).unwrap();
            if let Some(caps) = addr_re.captures(line) {
                let addr = &caps[1];
                if addr.len() != 42 {
                    diagnostics.push(Diagnostic {
                        range: Range {
                            start: Position {
                                line: line_num as u32,
                                character: 0,
                            },
                            end: Position {
                                line: line_num as u32,
                                character: line.len() as u32,
                            },
                        },
                        severity: Some(DiagnosticSeverity::ERROR),
                        code: Some(lsp_types::NumberOrString::String("E002".to_string())),
                        source: Some("atlas-lsp".to_string()),
                        message: format!(
                            "Invalid EVM address: expected 42 characters (0x + 40 hex), got {}",
                            addr.len()
                        ),
                        ..Default::default()
                    });
                }
            }
        }

        diagnostics
    }

    /// Diagnose Rust pallet files.
    fn diagnose_rust(&self, content: &str) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        for (line_num, line) in content.lines().enumerate() {
            // Check for common pallet issues

            // Missing weight annotation
            if line.contains("pub fn ") && line.contains("origin") {
                // Look back for weight annotation
                let has_weight = if line_num > 0 {
                    let prev_lines: Vec<_> = content.lines().take(line_num).collect();
                    prev_lines.iter().rev().take(5).any(|l| {
                        l.contains("#[pallet::weight") || l.contains("#[pallet::call_index")
                    })
                } else {
                    false
                };

                if !has_weight && content.contains("#[pallet::call]") {
                    diagnostics.push(Diagnostic {
                        range: Range {
                            start: Position {
                                line: line_num as u32,
                                character: 0,
                            },
                            end: Position {
                                line: line_num as u32,
                                character: line.len() as u32,
                            },
                        },
                        severity: Some(DiagnosticSeverity::WARNING),
                        code: Some(lsp_types::NumberOrString::String("W010".to_string())),
                        source: Some("atlas-lsp".to_string()),
                        message:
                            "Dispatchable function may be missing #[pallet::weight(...)] annotation"
                                .to_string(),
                        ..Default::default()
                    });
                }
            }

            // Check for ensure! without frame_support import
            if line.contains("ensure!(") && !content.contains("use frame_support") {
                diagnostics.push(Diagnostic {
                    range: Range {
                        start: Position {
                            line: line_num as u32,
                            character: 0,
                        },
                        end: Position {
                            line: line_num as u32,
                            character: line.len() as u32,
                        },
                    },
                    severity: Some(DiagnosticSeverity::HINT),
                    code: Some(lsp_types::NumberOrString::String("H001".to_string())),
                    source: Some("atlas-lsp".to_string()),
                    message: "ensure! macro requires `use frame_support::ensure;`".to_string(),
                    ..Default::default()
                });
            }

            // Check for deprecated patterns
            if line.contains("decl_storage!") {
                diagnostics.push(Diagnostic {
                    range: Range {
                        start: Position { line: line_num as u32, character: 0 },
                        end: Position { line: line_num as u32, character: line.len() as u32 },
                    },
                    severity: Some(DiagnosticSeverity::WARNING),
                    code: Some(lsp_types::NumberOrString::String("D001".to_string())),
                    source: Some("atlas-lsp".to_string()),
                    message: "decl_storage! is deprecated. Use #[pallet::storage] attribute macro instead.".to_string(),
                    ..Default::default()
                });
            }

            if line.contains("decl_module!") {
                diagnostics.push(Diagnostic {
                    range: Range {
                        start: Position {
                            line: line_num as u32,
                            character: 0,
                        },
                        end: Position {
                            line: line_num as u32,
                            character: line.len() as u32,
                        },
                    },
                    severity: Some(DiagnosticSeverity::WARNING),
                    code: Some(lsp_types::NumberOrString::String("D002".to_string())),
                    source: Some("atlas-lsp".to_string()),
                    message:
                        "decl_module! is deprecated. Use #[pallet::call] attribute macro instead."
                            .to_string(),
                    ..Default::default()
                });
            }
        }

        diagnostics
    }
}
