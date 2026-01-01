//! Document storage and management.

use dashmap::DashMap;
use lsp_types::{Position, Range, Url};
use ropey::Rope;

/// Stored document information.
pub struct Document {
    pub content: Rope,
    pub language_id: String,
    pub version: i32,
}

impl Document {
    pub fn new(content: String, language_id: String) -> Self {
        Self {
            content: Rope::from_str(&content),
            language_id,
            version: 0,
        }
    }

    /// Get the text at a given line.
    pub fn line(&self, line: usize) -> Option<&str> {
        if line < self.content.len_lines() {
            Some(self.content.line(line).as_str().unwrap_or(""))
        } else {
            None
        }
    }

    /// Get the word at a position.
    pub fn word_at(&self, position: Position) -> Option<String> {
        let line = self.line(position.line as usize)?;
        let col = position.character as usize;

        if col > line.len() {
            return None;
        }

        // Find word boundaries
        let start = line[..col]
            .rfind(|c: char| !c.is_alphanumeric() && c != '_')
            .map(|i| i + 1)
            .unwrap_or(0);

        let end = line[col..]
            .find(|c: char| !c.is_alphanumeric() && c != '_')
            .map(|i| col + i)
            .unwrap_or(line.len());

        if start < end {
            Some(line[start..end].to_string())
        } else {
            None
        }
    }

    /// Get text in a range.
    pub fn text_range(&self, range: Range) -> String {
        let start_idx = self.position_to_offset(range.start);
        let end_idx = self.position_to_offset(range.end);

        if let (Some(start), Some(end)) = (start_idx, end_idx) {
            self.content.slice(start..end).to_string()
        } else {
            String::new()
        }
    }

    /// Convert position to byte offset.
    pub fn position_to_offset(&self, position: Position) -> Option<usize> {
        let line = position.line as usize;
        if line >= self.content.len_lines() {
            return None;
        }

        let line_start = self.content.line_to_char(line);
        let col = position.character as usize;
        let line_len = self.content.line(line).len_chars();

        if col > line_len {
            Some(line_start + line_len)
        } else {
            Some(line_start + col)
        }
    }

    /// Convert byte offset to position.
    pub fn offset_to_position(&self, offset: usize) -> Position {
        let line = self.content.char_to_line(offset);
        let line_start = self.content.line_to_char(line);
        let character = offset - line_start;

        Position {
            line: line as u32,
            character: character as u32,
        }
    }

    /// Apply an incremental change.
    pub fn apply_change(&mut self, range: Range, text: &str) {
        let start = self.position_to_offset(range.start).unwrap_or(0);
        let end = self.position_to_offset(range.end).unwrap_or(start);

        self.content.remove(start..end);
        self.content.insert(start, text);
        self.version += 1;
    }

    /// Get full content as string.
    pub fn text(&self) -> String {
        self.content.to_string()
    }

    /// Get line count.
    pub fn line_count(&self) -> usize {
        self.content.len_lines()
    }
}

/// Document store for managing open documents.
pub struct DocumentStore {
    documents: DashMap<Url, Document>,
}

impl DocumentStore {
    pub fn new() -> Self {
        Self {
            documents: DashMap::new(),
        }
    }

    /// Open a document.
    pub fn open(&self, uri: Url, content: String, language_id: String) {
        self.documents
            .insert(uri, Document::new(content, language_id));
    }

    /// Close a document.
    pub fn close(&self, uri: &Url) {
        self.documents.remove(uri);
    }

    /// Get a document.
    pub fn get(&self, uri: &Url) -> Option<dashmap::mapref::one::Ref<'_, Url, Document>> {
        self.documents.get(uri)
    }

    /// Set document content (full sync).
    pub fn set_content(&self, uri: &Url, content: &str) {
        if let Some(mut doc) = self.documents.get_mut(uri) {
            doc.content = Rope::from_str(content);
            doc.version += 1;
        }
    }

    /// Apply incremental change.
    pub fn apply_change(&self, uri: &Url, range: Range, text: &str) {
        if let Some(mut doc) = self.documents.get_mut(uri) {
            doc.apply_change(range, text);
        }
    }

    /// Check if a document is open.
    pub fn is_open(&self, uri: &Url) -> bool {
        self.documents.contains_key(uri)
    }

    /// Get all open document URIs.
    pub fn all_uris(&self) -> Vec<Url> {
        self.documents.iter().map(|r| r.key().clone()).collect()
    }
}

impl Default for DocumentStore {
    fn default() -> Self {
        Self::new()
    }
}
