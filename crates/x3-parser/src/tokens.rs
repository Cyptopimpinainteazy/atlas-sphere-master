//! Deterministic token stream wrapper with position tracking.
//!
//! This module re-exports the cursor as a `TokenStream` for clarity and adds
//! additional helpers for diagnostics and bounded nesting checks.

use x3_common::{Keyword, Span, Symbol, Token, TokenKind};

/// Maximum expression nesting depth before the parser errors out.
pub const MAX_NESTING_DEPTH: usize = 256;

/// Token stream wrapper that tracks position and supports backtracking.
pub struct TokenStream {
    tokens: Vec<Token>,
    position: usize,
    /// Nesting depth tracker for bounded memory use.
    nesting_depth: usize,
}

impl TokenStream {
    /// Create a new `TokenStream` from a vector of tokens.
    /// Ensures there is always an EOF sentinel at the end.
    pub fn new(mut tokens: Vec<Token>) -> Self {
        if tokens.is_empty() {
            tokens.push(Token::new(TokenKind::Eof, Span::default()));
        } else if !matches!(tokens.last().unwrap().kind, TokenKind::Eof) {
            tokens.push(Token::new(TokenKind::Eof, Span::default()));
        }
        Self {
            tokens,
            position: 0,
            nesting_depth: 0,
        }
    }

    /// Current position in the token stream.
    #[inline]
    pub fn position(&self) -> usize {
        self.position
    }

    /// Peek at the current token without advancing.
    #[inline]
    pub fn peek(&self) -> &Token {
        self.tokens
            .get(self.position)
            .unwrap_or_else(|| self.tokens.last().unwrap())
    }

    /// Look ahead by `offset` tokens.
    #[inline]
    pub fn lookahead(&self, offset: usize) -> &Token {
        self.tokens
            .get(self.position + offset)
            .unwrap_or_else(|| self.tokens.last().unwrap())
    }

    /// Advance the cursor and return the current token.
    pub fn bump(&mut self) -> Token {
        let token = self.tokens[self.position].clone();
        if self.position + 1 < self.tokens.len() {
            self.position += 1;
        }
        token
    }

    /// Get the span of the previous token (useful after bumping).
    pub fn previous_span(&self) -> Span {
        if self.position == 0 {
            self.peek().span
        } else {
            self.tokens[self.position - 1].span
        }
    }

    /// Check if the current token is EOF.
    #[inline]
    pub fn is_eof(&self) -> bool {
        matches!(self.peek().kind, TokenKind::Eof)
    }

    // ────────────────────────────────────────────────────────────────────────
    // Symbol helpers
    // ────────────────────────────────────────────────────────────────────────

    /// Check if the current token is a specific symbol.
    #[inline]
    pub fn peek_symbol(&self, symbol: Symbol) -> bool {
        matches!(self.peek().kind, TokenKind::Symbol(sym) if sym == symbol)
    }

    /// If the current token matches `symbol`, consume it and return `true`.
    pub fn match_symbol(&mut self, symbol: Symbol) -> bool {
        if self.peek_symbol(symbol) {
            self.bump();
            true
        } else {
            false
        }
    }

    /// Consume the current token if it matches `symbol`, returning it.
    pub fn consume_symbol(&mut self, symbol: Symbol) -> Option<Token> {
        if self.peek_symbol(symbol) {
            Some(self.bump())
        } else {
            None
        }
    }

    // ────────────────────────────────────────────────────────────────────────
    // Keyword helpers
    // ────────────────────────────────────────────────────────────────────────

    /// Check if the current token is a specific keyword.
    #[inline]
    pub fn peek_keyword(&self, keyword: Keyword) -> bool {
        matches!(self.peek().kind, TokenKind::Keyword(kw) if kw == keyword)
    }

    /// If the current token matches `keyword`, consume it and return `true`.
    pub fn match_keyword(&mut self, keyword: Keyword) -> bool {
        if self.peek_keyword(keyword) {
            self.bump();
            true
        } else {
            false
        }
    }

    /// Consume the current token if it matches `keyword`, returning it.
    pub fn consume_keyword(&mut self, keyword: Keyword) -> Option<Token> {
        if self.peek_keyword(keyword) {
            Some(self.bump())
        } else {
            None
        }
    }

    // ────────────────────────────────────────────────────────────────────────
    // Nesting depth tracking
    // ────────────────────────────────────────────────────────────────────────

    /// Increment nesting depth. Returns `Err` if limit exceeded.
    pub fn enter_nesting(&mut self) -> Result<(), ()> {
        self.nesting_depth += 1;
        if self.nesting_depth > MAX_NESTING_DEPTH {
            Err(())
        } else {
            Ok(())
        }
    }

    /// Decrement nesting depth.
    pub fn exit_nesting(&mut self) {
        self.nesting_depth = self.nesting_depth.saturating_sub(1);
    }

    /// Current nesting depth.
    #[inline]
    pub fn nesting_depth(&self) -> usize {
        self.nesting_depth
    }
}
