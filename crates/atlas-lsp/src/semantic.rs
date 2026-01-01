//! Semantic tokens provider for syntax highlighting.

use lsp_types::{
    Position, Range, SemanticToken, SemanticTokenModifier, SemanticTokenType, SemanticTokens,
    SemanticTokensLegend, Url,
};
use std::sync::Arc;

use crate::document::DocumentStore;

/// Semantic token types used by the LSP.
pub const TOKEN_TYPES: &[SemanticTokenType] = &[
    SemanticTokenType::NAMESPACE, // 0 - modules, crates
    SemanticTokenType::TYPE,      // 1 - types, structs
    SemanticTokenType::CLASS,     // 2 - classes, pallets
    SemanticTokenType::ENUM,      // 3 - enums
    SemanticTokenType::INTERFACE, // 4 - traits
    SemanticTokenType::STRUCT,    // 5 - structs
    SemanticTokenType::PARAMETER, // 6 - parameters
    SemanticTokenType::VARIABLE,  // 7 - variables
    SemanticTokenType::PROPERTY,  // 8 - properties
    SemanticTokenType::FUNCTION,  // 9 - functions
    SemanticTokenType::MACRO,     // 10 - macros
    SemanticTokenType::KEYWORD,   // 11 - keywords
    SemanticTokenType::MODIFIER,  // 12 - modifiers
    SemanticTokenType::COMMENT,   // 13 - comments
    SemanticTokenType::STRING,    // 14 - strings
    SemanticTokenType::NUMBER,    // 15 - numbers
    SemanticTokenType::REGEXP,    // 16 - regex
    SemanticTokenType::OPERATOR,  // 17 - operators
];

/// Semantic token modifiers used by the LSP.
pub const TOKEN_MODIFIERS: &[SemanticTokenModifier] = &[
    SemanticTokenModifier::DECLARATION,     // 0
    SemanticTokenModifier::DEFINITION,      // 1
    SemanticTokenModifier::READONLY,        // 2
    SemanticTokenModifier::STATIC,          // 3
    SemanticTokenModifier::DEPRECATED,      // 4
    SemanticTokenModifier::ABSTRACT,        // 5
    SemanticTokenModifier::ASYNC,           // 6
    SemanticTokenModifier::MODIFICATION,    // 7
    SemanticTokenModifier::DOCUMENTATION,   // 8
    SemanticTokenModifier::DEFAULT_LIBRARY, // 9
];

/// Get the legend for semantic tokens.
pub fn legend() -> SemanticTokensLegend {
    SemanticTokensLegend {
        token_types: TOKEN_TYPES.to_vec(),
        token_modifiers: TOKEN_MODIFIERS.to_vec(),
    }
}

/// Provides semantic tokens for syntax highlighting.
pub struct SemanticTokensProvider {
    documents: Arc<DocumentStore>,
}

impl SemanticTokensProvider {
    pub fn new(documents: Arc<DocumentStore>) -> Self {
        Self { documents }
    }

    /// Get semantic tokens for an entire document.
    pub async fn full(&self, uri: &Url) -> Option<SemanticTokens> {
        let doc = self.documents.get(uri)?;
        let text = doc.content.to_string();

        let tokens = if uri.path().ends_with(".comit") || uri.path().ends_with(".atlas") {
            self.tokenize_comit(&text)
        } else if uri.path().ends_with(".rs") {
            self.tokenize_rust(&text)
        } else {
            vec![]
        };

        Some(SemanticTokens {
            result_id: None,
            data: tokens,
        })
    }

    /// Get semantic tokens for a range.
    pub async fn range(&self, uri: &Url, range: Range) -> Option<SemanticTokens> {
        let doc = self.documents.get(uri)?;
        let start_offset = doc.position_to_offset(range.start)?;
        let end_offset = doc.position_to_offset(range.end)?;

        let text = doc.content.to_string();
        let slice = &text[start_offset..end_offset.min(text.len())];

        let tokens = if uri.path().ends_with(".comit") || uri.path().ends_with(".atlas") {
            self.tokenize_comit_with_offset(slice, range.start)
        } else if uri.path().ends_with(".rs") {
            self.tokenize_rust_with_offset(slice, range.start)
        } else {
            vec![]
        };

        Some(SemanticTokens {
            result_id: None,
            data: tokens,
        })
    }

    fn tokenize_comit(&self, text: &str) -> Vec<SemanticToken> {
        self.tokenize_comit_with_offset(text, Position::new(0, 0))
    }

    fn tokenize_comit_with_offset(&self, text: &str, start: Position) -> Vec<SemanticToken> {
        let mut tokens = Vec::new();
        let mut prev_line = start.line;
        let mut prev_char = start.character;

        // Comit keywords
        let keywords = ["comit", "evm", "svm"];
        let properties = [
            "contract",
            "method",
            "args",
            "gas_limit",
            "value",
            "program",
            "instruction",
            "accounts",
            "compute_units",
            "data",
        ];

        for (line_num, line) in text.lines().enumerate() {
            let line_idx = start.line + line_num as u32;

            // Find keywords
            for keyword in &keywords {
                for (idx, _) in line.match_indices(keyword) {
                    let col = idx as u32;
                    tokens.push(SemanticToken {
                        delta_line: line_idx - prev_line,
                        delta_start: if line_idx == prev_line {
                            col - prev_char
                        } else {
                            col
                        },
                        length: keyword.len() as u32,
                        token_type: 11, // Keyword
                        token_modifiers_bitset: 0,
                    });
                    prev_line = line_idx;
                    prev_char = col;
                }
            }

            // Find properties
            for prop in &properties {
                // Match property: or "property":
                let pattern = format!("\"{}\":", prop);
                for (idx, _) in line.match_indices(&pattern) {
                    let col = idx as u32 + 1; // Skip the opening quote
                    tokens.push(SemanticToken {
                        delta_line: line_idx - prev_line,
                        delta_start: if line_idx == prev_line {
                            col - prev_char
                        } else {
                            col
                        },
                        length: prop.len() as u32,
                        token_type: 8, // Property
                        token_modifiers_bitset: 0,
                    });
                    prev_line = line_idx;
                    prev_char = col;
                }
            }

            // Find strings (0x addresses, quoted strings)
            for (idx, _) in line.match_indices("0x") {
                // Find end of hex string
                let rest = &line[idx + 2..];
                let hex_len = rest.chars().take_while(|c| c.is_ascii_hexdigit()).count();
                if hex_len >= 8 {
                    // Looks like an address
                    let col = idx as u32;
                    tokens.push(SemanticToken {
                        delta_line: line_idx - prev_line,
                        delta_start: if line_idx == prev_line {
                            col - prev_char
                        } else {
                            col
                        },
                        length: (2 + hex_len) as u32,
                        token_type: 14, // String
                        token_modifiers_bitset: 0,
                    });
                    prev_line = line_idx;
                    prev_char = col;
                }
            }

            // Find numbers
            let mut chars = line.char_indices().peekable();
            while let Some((idx, c)) = chars.next() {
                if c.is_ascii_digit() {
                    let start_idx = idx;
                    let mut end_idx = idx;
                    while let Some(&(i, ch)) = chars.peek() {
                        if ch.is_ascii_digit() || ch == '_' {
                            end_idx = i;
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    let num_len = end_idx - start_idx + 1;
                    if num_len > 0 {
                        let col = start_idx as u32;
                        tokens.push(SemanticToken {
                            delta_line: line_idx - prev_line,
                            delta_start: if line_idx == prev_line {
                                col - prev_char
                            } else {
                                col
                            },
                            length: num_len as u32,
                            token_type: 15, // Number
                            token_modifiers_bitset: 0,
                        });
                        prev_line = line_idx;
                        prev_char = col;
                    }
                }
            }

            // Find comments
            if let Some(idx) = line.find("//") {
                let col = idx as u32;
                let comment_len = (line.len() - idx) as u32;
                tokens.push(SemanticToken {
                    delta_line: line_idx - prev_line,
                    delta_start: if line_idx == prev_line {
                        col - prev_char
                    } else {
                        col
                    },
                    length: comment_len,
                    token_type: 13, // Comment
                    token_modifiers_bitset: 0,
                });
                prev_line = line_idx;
                prev_char = col;
            }
        }

        // Sort tokens by position
        tokens.sort_by(|a, b| {
            let a_line = a.delta_line;
            let b_line = b.delta_line;
            if a_line != b_line {
                a_line.cmp(&b_line)
            } else {
                a.delta_start.cmp(&b.delta_start)
            }
        });

        // Convert to relative positions
        self.convert_to_delta(tokens)
    }

    fn tokenize_rust(&self, text: &str) -> Vec<SemanticToken> {
        self.tokenize_rust_with_offset(text, Position::new(0, 0))
    }

    fn tokenize_rust_with_offset(&self, text: &str, start: Position) -> Vec<SemanticToken> {
        let mut tokens = Vec::new();
        let mut prev_line = start.line;
        let mut prev_char = start.character;

        // Rust/Substrate keywords
        let keywords = [
            "fn", "let", "mut", "pub", "struct", "enum", "impl", "trait", "type", "const",
            "static", "use", "mod", "crate", "self", "super", "where", "for", "loop", "while",
            "if", "else", "match", "return", "async", "await", "move", "ref", "dyn", "unsafe",
        ];

        // Substrate/FRAME macros
        let macros = [
            "pallet",
            "config",
            "storage",
            "call",
            "event",
            "error",
            "hooks",
            "genesis_config",
            "genesis_build",
            "weight",
            "transactional",
            "require_transactional",
        ];

        // Atlas-specific types
        let atlas_types = [
            "ComitPayload",
            "EvmPayload",
            "SvmPayload",
            "ExecutionReceipt",
            "AtlasKernel",
            "CanonicalLedger",
            "AuthorizedAccounts",
        ];

        for (line_num, line) in text.lines().enumerate() {
            let line_idx = start.line + line_num as u32;

            // Find keywords
            for keyword in &keywords {
                // Match whole word
                let pattern = format!(r"\b{}\b", keyword);
                if let Ok(re) = regex::Regex::new(&pattern) {
                    for mat in re.find_iter(line) {
                        let col = mat.start() as u32;
                        tokens.push(SemanticToken {
                            delta_line: line_idx - prev_line,
                            delta_start: if line_idx == prev_line {
                                col.saturating_sub(prev_char)
                            } else {
                                col
                            },
                            length: keyword.len() as u32,
                            token_type: 11, // Keyword
                            token_modifiers_bitset: 0,
                        });
                        prev_line = line_idx;
                        prev_char = col;
                    }
                }
            }

            // Find macros (attributes like #[pallet::config])
            if line.contains("#[pallet::") {
                for macro_name in &macros {
                    let pattern = format!("pallet::{}", macro_name);
                    if let Some(idx) = line.find(&pattern) {
                        let col = idx as u32;
                        tokens.push(SemanticToken {
                            delta_line: line_idx - prev_line,
                            delta_start: if line_idx == prev_line {
                                col.saturating_sub(prev_char)
                            } else {
                                col
                            },
                            length: pattern.len() as u32,
                            token_type: 10, // Macro
                            token_modifiers_bitset: 0,
                        });
                        prev_line = line_idx;
                        prev_char = col;
                    }
                }
            }

            // Find Atlas types
            for atlas_type in &atlas_types {
                let pattern = format!(r"\b{}\b", atlas_type);
                if let Ok(re) = regex::Regex::new(&pattern) {
                    for mat in re.find_iter(line) {
                        let col = mat.start() as u32;
                        tokens.push(SemanticToken {
                            delta_line: line_idx - prev_line,
                            delta_start: if line_idx == prev_line {
                                col.saturating_sub(prev_char)
                            } else {
                                col
                            },
                            length: atlas_type.len() as u32,
                            token_type: 1, // Type
                            token_modifiers_bitset: 0,
                        });
                        prev_line = line_idx;
                        prev_char = col;
                    }
                }
            }

            // Find function definitions
            if let Ok(re) = regex::Regex::new(r"fn\s+(\w+)") {
                for caps in re.captures_iter(line) {
                    if let Some(name) = caps.get(1) {
                        let col = name.start() as u32;
                        tokens.push(SemanticToken {
                            delta_line: line_idx - prev_line,
                            delta_start: if line_idx == prev_line {
                                col.saturating_sub(prev_char)
                            } else {
                                col
                            },
                            length: name.len() as u32,
                            token_type: 9,             // Function
                            token_modifiers_bitset: 1, // Declaration
                        });
                        prev_line = line_idx;
                        prev_char = col;
                    }
                }
            }

            // Find comments
            if let Some(idx) = line.find("//") {
                let col = idx as u32;
                let comment_len = (line.len() - idx) as u32;
                tokens.push(SemanticToken {
                    delta_line: line_idx - prev_line,
                    delta_start: if line_idx == prev_line {
                        col.saturating_sub(prev_char)
                    } else {
                        col
                    },
                    length: comment_len,
                    token_type: 13, // Comment
                    token_modifiers_bitset: 0,
                });
                prev_line = line_idx;
                prev_char = col;
            }
        }

        // Convert to relative positions
        self.convert_to_delta(tokens)
    }

    fn convert_to_delta(&self, mut tokens: Vec<SemanticToken>) -> Vec<SemanticToken> {
        // Sort by absolute position first
        tokens.sort_by(|a, b| {
            if a.delta_line != b.delta_line {
                a.delta_line.cmp(&b.delta_line)
            } else {
                a.delta_start.cmp(&b.delta_start)
            }
        });

        // Convert to relative deltas
        let mut result = Vec::with_capacity(tokens.len());
        let mut prev_line = 0u32;
        let mut prev_start = 0u32;

        for token in tokens {
            let delta_line = token.delta_line - prev_line;
            let delta_start = if delta_line == 0 {
                token.delta_start.saturating_sub(prev_start)
            } else {
                token.delta_start
            };

            result.push(SemanticToken {
                delta_line,
                delta_start,
                length: token.length,
                token_type: token.token_type,
                token_modifiers_bitset: token.token_modifiers_bitset,
            });

            prev_line = token.delta_line;
            prev_start = token.delta_start;
        }

        result
    }
}
