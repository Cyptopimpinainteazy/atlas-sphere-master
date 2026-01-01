//! Completion provider for Atlas-specific constructs.

use crate::document::DocumentStore;
use lsp_types::{
    CompletionItem, CompletionItemKind, Documentation, InsertTextFormat, MarkupContent, MarkupKind,
    Position, Url,
};
use std::sync::Arc;

/// Provides completion items for Atlas development.
pub struct CompletionProvider {
    documents: Arc<DocumentStore>,
}

impl CompletionProvider {
    pub fn new(documents: Arc<DocumentStore>) -> Self {
        Self { documents }
    }

    /// Generate completions at a position.
    pub async fn complete(&self, uri: &Url, position: Position) -> Vec<CompletionItem> {
        let mut items = Vec::new();

        // Get document context
        let doc = match self.documents.get(uri) {
            Some(d) => d,
            None => return items,
        };

        let line = match doc.line(position.line as usize) {
            Some(l) => l.to_string(),
            None => return items,
        };

        let col = position.character as usize;
        let prefix = if col <= line.len() {
            &line[..col]
        } else {
            &line
        };

        // Determine completion context
        if self.is_comit_file(uri) {
            items.extend(self.comit_completions(prefix));
        } else if self.is_rust_file(uri) {
            items.extend(self.rust_completions(prefix, &doc.text()));
        }

        // Always add Atlas-specific completions
        items.extend(self.atlas_completions(prefix));

        items
    }

    /// Resolve additional completion item details.
    pub async fn resolve(&self, mut item: CompletionItem) -> CompletionItem {
        // Add documentation if not present
        if item.documentation.is_none() {
            if let Some(docs) = self.get_documentation(&item.label) {
                item.documentation = Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: docs,
                }));
            }
        }
        item
    }

    fn is_comit_file(&self, uri: &Url) -> bool {
        uri.path().ends_with(".comit") || uri.path().ends_with(".atlas")
    }

    fn is_rust_file(&self, uri: &Url) -> bool {
        uri.path().ends_with(".rs")
    }

    /// Completions for .comit files.
    fn comit_completions(&self, prefix: &str) -> Vec<CompletionItem> {
        let mut items = Vec::new();

        // Transaction templates
        if prefix.trim().is_empty() || prefix.contains("comit") {
            items.push(CompletionItem {
                label: "comit".to_string(),
                kind: Some(CompletionItemKind::SNIPPET),
                detail: Some("Comit transaction definition".to_string()),
                insert_text: Some(
                    r#"comit "${1:name}" {
    evm {
        contract: "${2:0x...}",
        method: "${3:transfer}",
        args: [${4}],
        gas_limit: ${5:100000}
    }
    svm {
        program: "${6:...}",
        instruction: "${7:0}",
        accounts: [${8}],
        compute_units: ${9:200000}
    }
}"#
                    .to_string(),
                ),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..Default::default()
            });

            items.push(CompletionItem {
                label: "evm_only".to_string(),
                kind: Some(CompletionItemKind::SNIPPET),
                detail: Some("EVM-only Comit transaction".to_string()),
                insert_text: Some(
                    r#"comit "${1:name}" {
    evm {
        contract: "${2:0x...}",
        method: "${3:transfer}",
        args: [$4],
        gas_limit: ${5:100000}
    }
}"#
                    .to_string(),
                ),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..Default::default()
            });

            items.push(CompletionItem {
                label: "svm_only".to_string(),
                kind: Some(CompletionItemKind::SNIPPET),
                detail: Some("SVM-only Comit transaction".to_string()),
                insert_text: Some(
                    r#"comit "${1:name}" {
    svm {
        program: "${2:...}",
        instruction: "${3:0}",
        accounts: [$4],
        compute_units: ${5:200000}
    }
}"#
                    .to_string(),
                ),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..Default::default()
            });
        }

        // EVM properties
        if prefix.contains("evm") || prefix.ends_with(".") {
            items.extend(vec![
                self.simple_completion(
                    "contract",
                    CompletionItemKind::PROPERTY,
                    "EVM contract address",
                ),
                self.simple_completion(
                    "method",
                    CompletionItemKind::PROPERTY,
                    "Contract method name",
                ),
                self.simple_completion("args", CompletionItemKind::PROPERTY, "Method arguments"),
                self.simple_completion(
                    "gas_limit",
                    CompletionItemKind::PROPERTY,
                    "Gas limit for execution",
                ),
                self.simple_completion("value", CompletionItemKind::PROPERTY, "ETH value to send"),
            ]);
        }

        // SVM properties
        if prefix.contains("svm") || prefix.ends_with(".") {
            items.extend(vec![
                self.simple_completion(
                    "program",
                    CompletionItemKind::PROPERTY,
                    "Solana program ID",
                ),
                self.simple_completion(
                    "instruction",
                    CompletionItemKind::PROPERTY,
                    "Instruction index",
                ),
                self.simple_completion("accounts", CompletionItemKind::PROPERTY, "Account list"),
                self.simple_completion(
                    "compute_units",
                    CompletionItemKind::PROPERTY,
                    "Compute unit budget",
                ),
                self.simple_completion("data", CompletionItemKind::PROPERTY, "Instruction data"),
            ]);
        }

        items
    }

    /// Completions for Rust files (pallet development).
    fn rust_completions(&self, prefix: &str, content: &str) -> Vec<CompletionItem> {
        let mut items = Vec::new();

        // Pallet macros
        if prefix.contains("#[") || prefix.ends_with("#") {
            items.extend(vec![
                self.macro_completion("pallet::call", "Define dispatchable functions"),
                self.macro_completion("pallet::storage", "Define storage items"),
                self.macro_completion("pallet::event", "Define pallet events"),
                self.macro_completion("pallet::error", "Define pallet errors"),
                self.macro_completion("pallet::config", "Define pallet configuration trait"),
                self.macro_completion("pallet::hooks", "Define pallet hooks"),
                self.macro_completion("pallet::weight", "Specify call weight"),
                self.macro_completion("pallet::genesis_config", "Define genesis configuration"),
            ]);
        }

        // Frame imports
        if prefix.contains("use ") || prefix.contains("frame") {
            items.extend(vec![
                self.simple_completion(
                    "frame_support",
                    CompletionItemKind::MODULE,
                    "FRAME support utilities",
                ),
                self.simple_completion(
                    "frame_system",
                    CompletionItemKind::MODULE,
                    "FRAME system pallet",
                ),
                self.simple_completion(
                    "sp_runtime",
                    CompletionItemKind::MODULE,
                    "Substrate runtime primitives",
                ),
                self.simple_completion(
                    "sp_std",
                    CompletionItemKind::MODULE,
                    "Substrate std replacement",
                ),
            ]);
        }

        // Atlas-specific imports
        if content.contains("atlas") || prefix.contains("atlas") {
            items.extend(vec![
                self.simple_completion(
                    "pallet_atlas_kernel",
                    CompletionItemKind::MODULE,
                    "Atlas Kernel pallet",
                ),
                self.simple_completion(
                    "ComitPayload",
                    CompletionItemKind::STRUCT,
                    "Comit transaction payload",
                ),
                self.simple_completion(
                    "EvmPayload",
                    CompletionItemKind::STRUCT,
                    "EVM execution payload",
                ),
                self.simple_completion(
                    "SvmPayload",
                    CompletionItemKind::STRUCT,
                    "SVM execution payload",
                ),
            ]);
        }

        items
    }

    /// General Atlas completions.
    fn atlas_completions(&self, prefix: &str) -> Vec<CompletionItem> {
        let mut items = Vec::new();

        // Type completions
        items.extend(vec![
            self.type_completion("AccountId", "32-byte account identifier"),
            self.type_completion("Balance", "Native token balance"),
            self.type_completion("BlockNumber", "Block number type"),
            self.type_completion("Hash", "256-bit hash type"),
            self.type_completion("AssetId", "Asset identifier"),
        ]);

        // Common values
        if prefix.contains("0x") {
            items.push(CompletionItem {
                label: "zero_address".to_string(),
                kind: Some(CompletionItemKind::CONSTANT),
                detail: Some("Zero address".to_string()),
                insert_text: Some("0x0000000000000000000000000000000000000000".to_string()),
                ..Default::default()
            });
        }

        items
    }

    fn simple_completion(
        &self,
        label: &str,
        kind: CompletionItemKind,
        detail: &str,
    ) -> CompletionItem {
        CompletionItem {
            label: label.to_string(),
            kind: Some(kind),
            detail: Some(detail.to_string()),
            ..Default::default()
        }
    }

    fn macro_completion(&self, label: &str, detail: &str) -> CompletionItem {
        CompletionItem {
            label: label.to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some(detail.to_string()),
            insert_text: Some(format!("#[{}]", label)),
            ..Default::default()
        }
    }

    fn type_completion(&self, label: &str, detail: &str) -> CompletionItem {
        CompletionItem {
            label: label.to_string(),
            kind: Some(CompletionItemKind::TYPE_PARAMETER),
            detail: Some(detail.to_string()),
            ..Default::default()
        }
    }

    fn get_documentation(&self, label: &str) -> Option<String> {
        match label {
            "comit" => Some(
                "# Comit Transaction\n\n\
                A Comit is an atomic cross-VM transaction that executes on both EVM and SVM.\n\n\
                ## Fields\n\
                - `evm`: EVM execution payload (optional)\n\
                - `svm`: SVM execution payload (optional)\n\n\
                At least one of `evm` or `svm` must be specified."
                    .to_string(),
            ),
            "pallet::call" => Some(
                "# Pallet Call\n\n\
                Defines dispatchable functions (extrinsics) for the pallet.\n\n\
                ```rust\n\
                #[pallet::call]\n\
                impl<T: Config> Pallet<T> {\n\
                    #[pallet::weight(10_000)]\n\
                    pub fn my_function(origin: OriginFor<T>) -> DispatchResult {\n\
                        Ok(())\n\
                    }\n\
                }\n\
                ```"
                .to_string(),
            ),
            _ => None,
        }
    }
}
