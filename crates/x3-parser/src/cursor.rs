use x3_common::{Keyword, Span, Symbol, Token, TokenKind};

/// Token stream cursor with helpers for peeking and consuming tokens.
#[derive(Clone)]
pub struct Cursor {
    tokens: Vec<Token>,
    position: usize,
}

impl Cursor {
    pub fn new(mut tokens: Vec<Token>) -> Self {
        if tokens.is_empty() {
            tokens.push(Token::new(TokenKind::Eof, Span::default()));
        } else if !matches!(tokens.last().unwrap().kind, TokenKind::Eof) {
            tokens.push(Token::new(TokenKind::Eof, Span::default()));
        }
        Self {
            tokens,
            position: 0,
        }
    }

    pub fn peek(&self) -> &Token {
        self.tokens
            .get(self.position)
            .unwrap_or_else(|| self.tokens.last().unwrap())
    }

    /// Peek n tokens ahead (0 = current, 1 = next, 2 = after next, etc.)
    pub fn peek_n(&self, n: usize) -> Option<&Token> {
        self.tokens.get(self.position + n)
    }

    pub fn lookahead(&self, offset: usize) -> &Token {
        self.tokens
            .get(self.position + offset)
            .unwrap_or_else(|| self.tokens.last().unwrap())
    }

    pub fn bump(&mut self) -> Token {
        let token = self.tokens[self.position].clone();
        if self.position + 1 < self.tokens.len() {
            self.position += 1;
        }
        token
    }

    pub fn previous_span(&self) -> Span {
        if self.position == 0 {
            self.peek().span
        } else {
            self.tokens[self.position - 1].span
        }
    }

    pub fn is_eof(&self) -> bool {
        matches!(self.peek().kind, TokenKind::Eof)
    }

    pub fn peek_symbol(&self, symbol: Symbol) -> bool {
        matches!(self.peek().kind, TokenKind::Symbol(sym) if sym == symbol)
    }

    pub fn match_symbol(&mut self, symbol: Symbol) -> bool {
        if self.peek_symbol(symbol) {
            self.bump();
            true
        } else {
            false
        }
    }

    pub fn consume_symbol(&mut self, symbol: Symbol) -> Option<Token> {
        if self.peek_symbol(symbol) {
            Some(self.bump())
        } else {
            None
        }
    }

    pub fn peek_keyword(&self, keyword: Keyword) -> bool {
        matches!(self.peek().kind, TokenKind::Keyword(kw) if kw == keyword)
    }

    pub fn match_keyword(&mut self, keyword: Keyword) -> bool {
        if self.peek_keyword(keyword) {
            self.bump();
            true
        } else {
            false
        }
    }

    pub fn consume_keyword(&mut self, keyword: Keyword) -> Option<Token> {
        if self.peek_keyword(keyword) {
            Some(self.bump())
        } else {
            None
        }
    }
}
