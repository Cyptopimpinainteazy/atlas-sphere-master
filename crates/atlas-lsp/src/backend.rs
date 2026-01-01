//! LSP Backend implementation.

use crate::completion::CompletionProvider;
use crate::diagnostics::DiagnosticsProvider;
use crate::document::DocumentStore;
use crate::hover::HoverProvider;
use crate::semantic::SemanticTokensProvider;
use lsp_types::*;
use std::sync::Arc;
use tower_lsp::jsonrpc::Result;
use tower_lsp::{Client, LanguageServer};
use tracing::{debug, info};

/// LSP Backend for Atlas Sphere.
pub struct Backend {
    client: Client,
    documents: Arc<DocumentStore>,
    completion: CompletionProvider,
    diagnostics: DiagnosticsProvider,
    hover: HoverProvider,
    semantic: SemanticTokensProvider,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        let documents = Arc::new(DocumentStore::new());
        Self {
            client,
            documents: documents.clone(),
            completion: CompletionProvider::new(documents.clone()),
            diagnostics: DiagnosticsProvider::new(documents.clone()),
            hover: HoverProvider::new(documents.clone()),
            semantic: SemanticTokensProvider::new(documents),
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _params: InitializeParams) -> Result<InitializeResult> {
        info!("Atlas LSP initialized");

        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                // Text document sync
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        open_close: Some(true),
                        change: Some(TextDocumentSyncKind::INCREMENTAL),
                        save: Some(TextDocumentSyncSaveOptions::SaveOptions(SaveOptions {
                            include_text: Some(true),
                        })),
                        ..Default::default()
                    },
                )),

                // Completion
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![
                        ".".to_string(),
                        ":".to_string(),
                        "<".to_string(),
                        "\"".to_string(),
                    ]),
                    resolve_provider: Some(true),
                    ..Default::default()
                }),

                // Hover
                hover_provider: Some(HoverProviderCapability::Simple(true)),

                // Signature help
                signature_help_provider: Some(SignatureHelpOptions {
                    trigger_characters: Some(vec!["(".to_string(), ",".to_string()]),
                    retrigger_characters: None,
                    work_done_progress_options: Default::default(),
                }),

                // Go to definition
                definition_provider: Some(OneOf::Left(true)),

                // References
                references_provider: Some(OneOf::Left(true)),

                // Document symbols
                document_symbol_provider: Some(OneOf::Left(true)),

                // Workspace symbols
                workspace_symbol_provider: Some(OneOf::Left(true)),

                // Code actions
                code_action_provider: Some(CodeActionProviderCapability::Simple(true)),

                // Formatting
                document_formatting_provider: Some(OneOf::Left(true)),

                // Semantic tokens
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensOptions(
                        SemanticTokensOptions {
                            legend: SemanticTokensLegend {
                                token_types: vec![
                                    SemanticTokenType::NAMESPACE,
                                    SemanticTokenType::TYPE,
                                    SemanticTokenType::CLASS,
                                    SemanticTokenType::ENUM,
                                    SemanticTokenType::INTERFACE,
                                    SemanticTokenType::STRUCT,
                                    SemanticTokenType::TYPE_PARAMETER,
                                    SemanticTokenType::PARAMETER,
                                    SemanticTokenType::VARIABLE,
                                    SemanticTokenType::PROPERTY,
                                    SemanticTokenType::ENUM_MEMBER,
                                    SemanticTokenType::FUNCTION,
                                    SemanticTokenType::METHOD,
                                    SemanticTokenType::MACRO,
                                    SemanticTokenType::KEYWORD,
                                    SemanticTokenType::MODIFIER,
                                    SemanticTokenType::COMMENT,
                                    SemanticTokenType::STRING,
                                    SemanticTokenType::NUMBER,
                                    SemanticTokenType::OPERATOR,
                                ],
                                token_modifiers: vec![
                                    SemanticTokenModifier::DECLARATION,
                                    SemanticTokenModifier::DEFINITION,
                                    SemanticTokenModifier::READONLY,
                                    SemanticTokenModifier::STATIC,
                                    SemanticTokenModifier::DEPRECATED,
                                    SemanticTokenModifier::ASYNC,
                                ],
                            },
                            full: Some(SemanticTokensFullOptions::Bool(true)),
                            range: Some(true),
                            ..Default::default()
                        },
                    ),
                ),

                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "atlas-lsp".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    async fn initialized(&self, _params: InitializedParams) {
        info!("Atlas LSP server initialized");
        self.client
            .log_message(MessageType::INFO, "Atlas LSP ready")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        info!("Atlas LSP shutting down");
        Ok(())
    }

    // ========================================================================
    // Document Synchronization
    // ========================================================================

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        debug!("Document opened: {}", uri);

        self.documents.open(
            uri.clone(),
            params.text_document.text,
            params.text_document.language_id,
        );

        // Run diagnostics
        let diagnostics = self.diagnostics.diagnose(&uri).await;
        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        debug!("Document changed: {}", uri);

        for change in params.content_changes {
            if let Some(range) = change.range {
                self.documents.apply_change(&uri, range, &change.text);
            } else {
                // Full document sync
                self.documents.set_content(&uri, &change.text);
            }
        }

        // Run diagnostics
        let diagnostics = self.diagnostics.diagnose(&uri).await;
        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        debug!("Document saved: {}", uri);

        if let Some(text) = params.text {
            self.documents.set_content(&uri, &text);
        }

        // Run full diagnostics on save
        let diagnostics = self.diagnostics.diagnose(&uri).await;
        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        debug!("Document closed: {}", uri);
        self.documents.close(&uri);

        // Clear diagnostics
        self.client.publish_diagnostics(uri, vec![], None).await;
    }

    // ========================================================================
    // Language Features
    // ========================================================================

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = &params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;

        debug!("Completion requested at {:?}", position);

        let items = self.completion.complete(uri, position).await;
        Ok(Some(CompletionResponse::Array(items)))
    }

    async fn completion_resolve(&self, item: CompletionItem) -> Result<CompletionItem> {
        Ok(self.completion.resolve(item).await)
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        debug!("Hover requested at {:?}", position);

        Ok(self.hover.hover(uri, position).await)
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        debug!("Go to definition at {:?}", position);

        // Get the word at position
        let doc = match self.documents.get(uri) {
            Some(d) => d,
            None => return Ok(None),
        };

        let word = match doc.word_at(position) {
            Some(w) => w,
            None => return Ok(None),
        };

        // Search for definition in current document
        let text = doc.text();
        drop(doc); // Release lock before searching

        // Look for common definition patterns
        let patterns = [
            format!("fn {}(", word),     // Function definition
            format!("struct {} ", word), // Struct definition
            format!("enum {} ", word),   // Enum definition
            format!("type {} ", word),   // Type alias
            format!("const {}", word),   // Constant
            format!("let {} ", word),    // Variable binding
            format!("let {}: ", word),   // Typed variable binding
            format!("asset {} ", word),  // Atlas asset definition
            format!("comit {} ", word),  // Atlas comit definition
        ];

        for pattern in &patterns {
            if let Some(idx) = text.find(pattern) {
                // Convert byte offset to position
                let doc = self.documents.get(uri).unwrap();
                let pos = doc.offset_to_position(idx);

                let location = Location {
                    uri: uri.clone(),
                    range: Range {
                        start: pos,
                        end: Position {
                            line: pos.line,
                            character: pos.character + word.len() as u32,
                        },
                    },
                };
                return Ok(Some(GotoDefinitionResponse::Scalar(location)));
            }
        }

        Ok(None)
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let uri = &params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;

        debug!("Find references at {:?}", position);

        // Get the word at position
        let doc = match self.documents.get(uri) {
            Some(d) => d,
            None => return Ok(None),
        };

        let word = match doc.word_at(position) {
            Some(w) => w,
            None => return Ok(None),
        };

        let text = doc.text();
        drop(doc);

        // Find all occurrences of the word
        let mut locations = Vec::new();
        let mut search_start = 0;

        while let Some(idx) = text[search_start..].find(&word) {
            let absolute_idx = search_start + idx;

            // Check word boundaries
            let before_ok = absolute_idx == 0
                || !text
                    .chars()
                    .nth(absolute_idx - 1)
                    .map(|c| c.is_alphanumeric() || c == '_')
                    .unwrap_or(false);
            let after_ok = absolute_idx + word.len() >= text.len()
                || !text
                    .chars()
                    .nth(absolute_idx + word.len())
                    .map(|c| c.is_alphanumeric() || c == '_')
                    .unwrap_or(false);

            if before_ok && after_ok {
                let doc = self.documents.get(uri).unwrap();
                let pos = doc.offset_to_position(absolute_idx);

                locations.push(Location {
                    uri: uri.clone(),
                    range: Range {
                        start: pos,
                        end: Position {
                            line: pos.line,
                            character: pos.character + word.len() as u32,
                        },
                    },
                });
            }

            search_start = absolute_idx + 1;
        }

        if locations.is_empty() {
            Ok(None)
        } else {
            Ok(Some(locations))
        }
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let uri = &params.text_document.uri;

        debug!("Document symbols for {}", uri);

        let doc = match self.documents.get(uri) {
            Some(d) => d,
            None => return Ok(None),
        };

        let text = doc.text();
        let mut symbols = Vec::new();

        // Parse document for symbol definitions
        for (line_idx, line) in text.lines().enumerate() {
            let trimmed = line.trim();

            // Function definitions
            if let Some(name) = extract_symbol_name(trimmed, "fn ", "(") {
                symbols.push(create_symbol_info(
                    name,
                    SymbolKind::FUNCTION,
                    uri.clone(),
                    line_idx as u32,
                ));
            }
            // Struct definitions
            else if let Some(name) = extract_symbol_name(trimmed, "struct ", " ") {
                symbols.push(create_symbol_info(
                    name,
                    SymbolKind::STRUCT,
                    uri.clone(),
                    line_idx as u32,
                ));
            }
            // Enum definitions
            else if let Some(name) = extract_symbol_name(trimmed, "enum ", " ") {
                symbols.push(create_symbol_info(
                    name,
                    SymbolKind::ENUM,
                    uri.clone(),
                    line_idx as u32,
                ));
            }
            // Type aliases
            else if let Some(name) = extract_symbol_name(trimmed, "type ", " ") {
                symbols.push(create_symbol_info(
                    name,
                    SymbolKind::TYPE_PARAMETER,
                    uri.clone(),
                    line_idx as u32,
                ));
            }
            // Constants
            else if let Some(name) = extract_symbol_name(trimmed, "const ", ":") {
                symbols.push(create_symbol_info(
                    name,
                    SymbolKind::CONSTANT,
                    uri.clone(),
                    line_idx as u32,
                ));
            }
            // Atlas-specific: asset definitions
            else if let Some(name) = extract_symbol_name(trimmed, "asset ", " ") {
                symbols.push(create_symbol_info(
                    name,
                    SymbolKind::CLASS,
                    uri.clone(),
                    line_idx as u32,
                ));
            }
            // Atlas-specific: comit definitions
            else if let Some(name) = extract_symbol_name(trimmed, "comit ", " ") {
                symbols.push(create_symbol_info(
                    name,
                    SymbolKind::MODULE,
                    uri.clone(),
                    line_idx as u32,
                ));
            }
        }

        if symbols.is_empty() {
            Ok(None)
        } else {
            Ok(Some(DocumentSymbolResponse::Flat(symbols)))
        }
    }

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let uri = &params.text_document.uri;
        let range = params.range;

        debug!("Code action at {:?}", range);

        let mut actions = Vec::new();

        // Check diagnostics for quick fixes
        for diagnostic in &params.context.diagnostics {
            if let Some(fix) = generate_quick_fix(uri, diagnostic) {
                actions.push(CodeActionOrCommand::CodeAction(fix));
            }
        }

        // Add refactoring actions if text is selected
        if range.start != range.end {
            if let Some(doc) = self.documents.get(uri) {
                let selected_text = doc.text_range(range);

                // Extract function refactoring
                if !selected_text.is_empty() && selected_text.len() < 1000 {
                    actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                        title: "Extract to function".to_string(),
                        kind: Some(CodeActionKind::REFACTOR_EXTRACT),
                        diagnostics: None,
                        edit: None, // Would be computed on resolve
                        command: None,
                        is_preferred: None,
                        disabled: None,
                        data: None,
                    }));
                }
            }
        }

        Ok(Some(actions))
    }

    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        let uri = &params.text_document.uri;

        debug!("Format document {}", uri);

        let doc = match self.documents.get(uri) {
            Some(d) => d,
            None => return Ok(None),
        };

        let text = doc.text();
        let line_count = doc.line_count();
        drop(doc);

        // Basic formatting: normalize indentation and spacing
        let formatted = format_atlas_code(&text);

        if formatted == text {
            return Ok(None);
        }

        // Return a single edit that replaces the entire document
        Ok(Some(vec![TextEdit {
            range: Range {
                start: Position {
                    line: 0,
                    character: 0,
                },
                end: Position {
                    line: line_count as u32,
                    character: 0,
                },
            },
            new_text: formatted,
        }]))
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let uri = &params.text_document.uri;

        debug!("Semantic tokens for {}", uri);

        match self.semantic.full(uri).await {
            Some(tokens) => Ok(Some(SemanticTokensResult::Tokens(tokens))),
            None => Ok(None),
        }
    }

    async fn semantic_tokens_range(
        &self,
        params: SemanticTokensRangeParams,
    ) -> Result<Option<SemanticTokensRangeResult>> {
        let uri = &params.text_document.uri;
        let range = params.range;

        debug!("Semantic tokens range {:?}", range);

        match self.semantic.range(uri, range).await {
            Some(tokens) => Ok(Some(SemanticTokensRangeResult::Tokens(tokens))),
            None => Ok(None),
        }
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Extract a symbol name from a line given a prefix and terminator.
fn extract_symbol_name(line: &str, prefix: &str, terminator: &str) -> Option<String> {
    if !line.starts_with(prefix) {
        return None;
    }

    let after_prefix = &line[prefix.len()..];
    let end = if terminator.is_empty() {
        after_prefix.len()
    } else if let Some(idx) = after_prefix.find(terminator) {
        idx
    } else if let Some(idx) = after_prefix.find('{') {
        idx
    } else {
        after_prefix.len()
    };

    let name = after_prefix[..end].trim();
    if name.is_empty()
        || !name
            .chars()
            .next()
            .map(|c| c.is_alphabetic() || c == '_')
            .unwrap_or(false)
    {
        return None;
    }

    // Extract just the identifier (stop at any non-identifier char)
    let name: String = name
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();

    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

/// Create a SymbolInformation struct.
fn create_symbol_info(name: String, kind: SymbolKind, uri: Url, line: u32) -> SymbolInformation {
    #[allow(deprecated)] // container_name is deprecated but still required
    SymbolInformation {
        name,
        kind,
        tags: None,
        deprecated: None,
        location: Location {
            uri,
            range: Range {
                start: Position { line, character: 0 },
                end: Position { line, character: 0 },
            },
        },
        container_name: None,
    }
}

/// Generate a quick fix code action from a diagnostic.
fn generate_quick_fix(uri: &Url, diagnostic: &Diagnostic) -> Option<CodeAction> {
    let message = &diagnostic.message;

    // Missing semicolon
    if message.contains("missing semicolon") || message.contains("expected `;`") {
        return Some(CodeAction {
            title: "Add semicolon".to_string(),
            kind: Some(CodeActionKind::QUICKFIX),
            diagnostics: Some(vec![diagnostic.clone()]),
            edit: Some(WorkspaceEdit {
                changes: Some(std::collections::HashMap::from([(
                    uri.clone(),
                    vec![TextEdit {
                        range: Range {
                            start: diagnostic.range.end,
                            end: diagnostic.range.end,
                        },
                        new_text: ";".to_string(),
                    }],
                )])),
                document_changes: None,
                change_annotations: None,
            }),
            command: None,
            is_preferred: Some(true),
            disabled: None,
            data: None,
        });
    }

    // Unused import
    if message.contains("unused import") {
        return Some(CodeAction {
            title: "Remove unused import".to_string(),
            kind: Some(CodeActionKind::QUICKFIX),
            diagnostics: Some(vec![diagnostic.clone()]),
            edit: Some(WorkspaceEdit {
                changes: Some(std::collections::HashMap::from([(
                    uri.clone(),
                    vec![TextEdit {
                        range: Range {
                            start: Position {
                                line: diagnostic.range.start.line,
                                character: 0,
                            },
                            end: Position {
                                line: diagnostic.range.start.line + 1,
                                character: 0,
                            },
                        },
                        new_text: String::new(),
                    }],
                )])),
                document_changes: None,
                change_annotations: None,
            }),
            command: None,
            is_preferred: Some(true),
            disabled: None,
            data: None,
        });
    }

    None
}

/// Basic code formatter for Atlas DSL.
fn format_atlas_code(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut indent_level: i32 = 0;
    let indent_str = "    "; // 4 spaces

    for line in text.lines() {
        let trimmed = line.trim();

        // Skip empty lines but preserve them
        if trimmed.is_empty() {
            result.push('\n');
            continue;
        }

        // Decrease indent before closing braces
        if trimmed.starts_with('}') || trimmed.starts_with(']') || trimmed.starts_with(')') {
            indent_level = (indent_level - 1).max(0);
        }

        // Write indented line
        for _ in 0..indent_level {
            result.push_str(indent_str);
        }
        result.push_str(trimmed);
        result.push('\n');

        // Increase indent after opening braces
        let opens = trimmed.chars().filter(|&c| c == '{' || c == '[').count() as i32;
        let closes = trimmed.chars().filter(|&c| c == '}' || c == ']').count() as i32;
        indent_level = (indent_level + opens - closes).max(0);

        // Handle trailing opens (e.g., "fn foo() {")
        if trimmed.ends_with('{') && !trimmed.starts_with('}') {
            // Already handled above
        }
    }

    // Remove trailing newline if original didn't have one
    if !text.ends_with('\n') && result.ends_with('\n') {
        result.pop();
    }

    result
}
