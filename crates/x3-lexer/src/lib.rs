use std::iter::Peekable;
use std::str::CharIndices;

use x3_common::{Keyword, Literal, Span, Symbol, Token, TokenKind};

/// A lexing iterator that produces X3 tokens from a string slice.
pub struct Lexer<'a> {
    src: &'a str,
    chars: Peekable<CharIndices<'a>>,
    start: usize,
    current: usize,
    eof_emitted: bool,
}

impl<'a> Lexer<'a> {
    pub fn new(src: &'a str) -> Self {
        Self {
            src,
            chars: src.char_indices().peekable(),
            start: 0,
            current: 0,
            eof_emitted: false,
        }
    }

    fn peek_char(&mut self) -> Option<char> {
        self.chars.peek().map(|(_, ch)| *ch)
    }

    fn peek_nth(&self, n: usize) -> Option<char> {
        self.chars.clone().nth(n).map(|(_, ch)| ch)
    }

    fn advance(&mut self) -> Option<char> {
        self.chars.next().map(|(idx, ch)| {
            self.current = idx + ch.len_utf8();
            ch
        })
    }

    fn current_span(&self) -> Span {
        Span::new(self.start, self.current)
    }

    fn make_token(&self, kind: TokenKind) -> Token {
        Token::new(kind, self.current_span())
    }

    fn skip_whitespace(&mut self) {
        loop {
            match self.peek_char() {
                Some(c) if c.is_whitespace() => {
                    self.advance();
                }
                Some('/') if self.peek_nth(1) == Some('/') => {
                    self.advance();
                    self.advance();
                    while let Some(ch) = self.peek_char() {
                        if ch == '\n' {
                            break;
                        }
                        self.advance();
                    }
                }
                _ => break,
            }
        }
    }

    fn lex_identifier(&mut self) -> Token {
        let mut identifier = String::new();
        while let Some(ch) = self.peek_char() {
            if is_identifier_continue(ch) {
                identifier.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        let kind = if let Some(keyword) = Keyword::from_str(&identifier) {
            TokenKind::Keyword(keyword)
        } else {
            TokenKind::Identifier(identifier)
        };

        self.make_token(kind)
    }

    fn lex_number(&mut self) -> Token {
        let mut number = String::new();
        while let Some(ch) = self.peek_char() {
            if ch.is_ascii_digit() {
                number.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        // Check for float: must be a dot followed by a digit (not another dot for range ..)
        let is_float = if self.peek_char() == Some('.')
            && self
                .peek_nth(1)
                .map(|c| c.is_ascii_digit())
                .unwrap_or(false)
        {
            number.push('.');
            self.advance();
            while let Some(ch) = self.peek_char() {
                if ch.is_ascii_digit() {
                    number.push(ch);
                    self.advance();
                } else {
                    break;
                }
            }
            true
        } else {
            false
        };

        let literal = if is_float {
            number
                .parse::<f64>()
                .map(Literal::Float)
                .unwrap_or(Literal::Float(0.0))
        } else {
            number
                .parse::<i64>()
                .map(Literal::Integer)
                .unwrap_or(Literal::Integer(0))
        };

        self.make_token(TokenKind::Literal(literal))
    }

    fn lex_string(&mut self) -> Token {
        self.advance();
        let mut contents = String::new();
        while let Some(ch) = self.peek_char() {
            if ch == '"' {
                self.advance();
                break;
            }
            contents.push(ch);
            self.advance();
        }
        self.make_token(TokenKind::Literal(Literal::String(contents)))
    }

    fn emit_symbol(&self, symbol: Symbol) -> Token {
        self.make_token(TokenKind::Symbol(symbol))
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            self.skip_whitespace();
            self.start = self.current;
            if self.peek_char().is_none() {
                if self.eof_emitted {
                    return None;
                }
                self.eof_emitted = true;
                return Some(Token::new(
                    TokenKind::Eof,
                    Span::new(self.current, self.current),
                ));
            }

            match self.peek_char().unwrap() {
                '"' => return Some(self.lex_string()),
                c if c.is_ascii_digit() => return Some(self.lex_number()),
                c if is_identifier_start(c) => return Some(self.lex_identifier()),
                '+' => {
                    self.advance();
                    return Some(self.emit_symbol(Symbol::Plus));
                }
                '-' => {
                    self.advance();
                    if self.peek_char() == Some('>') {
                        self.advance();
                        return Some(self.emit_symbol(Symbol::Arrow));
                    }
                    return Some(self.emit_symbol(Symbol::Minus));
                }
                '*' => {
                    self.advance();
                    return Some(self.emit_symbol(Symbol::Star));
                }
                '%' => {
                    self.advance();
                    return Some(self.emit_symbol(Symbol::Percent));
                }
                '=' => {
                    self.advance();
                    if self.peek_char() == Some('=') {
                        self.advance();
                        return Some(self.emit_symbol(Symbol::DoubleEquals));
                    }
                    if self.peek_char() == Some('>') {
                        self.advance();
                        return Some(self.emit_symbol(Symbol::FatArrow));
                    }
                    return Some(self.emit_symbol(Symbol::Equals));
                }
                '!' => {
                    self.advance();
                    if self.peek_char() == Some('=') {
                        self.advance();
                        return Some(self.emit_symbol(Symbol::BangEquals));
                    }
                    return Some(self.emit_symbol(Symbol::Bang));
                }
                '<' => {
                    self.advance();
                    if self.peek_char() == Some('=') {
                        self.advance();
                        return Some(self.emit_symbol(Symbol::LessEqual));
                    }
                    return Some(self.emit_symbol(Symbol::Less));
                }
                '>' => {
                    self.advance();
                    if self.peek_char() == Some('=') {
                        self.advance();
                        return Some(self.emit_symbol(Symbol::GreaterEqual));
                    }
                    return Some(self.emit_symbol(Symbol::Greater));
                }
                '&' => {
                    self.advance();
                    if self.peek_char() == Some('&') {
                        self.advance();
                        return Some(self.emit_symbol(Symbol::And));
                    }
                    return Some(self.emit_symbol(Symbol::Amp));
                }
                '|' => {
                    self.advance();
                    if self.peek_char() == Some('|') {
                        self.advance();
                        return Some(self.emit_symbol(Symbol::Or));
                    }
                    return Some(self.emit_symbol(Symbol::Pipe));
                }
                ':' => {
                    self.advance();
                    return Some(self.emit_symbol(Symbol::Colon));
                }
                ';' => {
                    self.advance();
                    return Some(self.emit_symbol(Symbol::Semicolon));
                }
                ',' => {
                    self.advance();
                    return Some(self.emit_symbol(Symbol::Comma));
                }
                '.' => {
                    self.advance();
                    return Some(self.emit_symbol(Symbol::Dot));
                }
                '(' => {
                    self.advance();
                    return Some(self.emit_symbol(Symbol::LeftParen));
                }
                ')' => {
                    self.advance();
                    return Some(self.emit_symbol(Symbol::RightParen));
                }
                '{' => {
                    self.advance();
                    return Some(self.emit_symbol(Symbol::LeftBrace));
                }
                '}' => {
                    self.advance();
                    return Some(self.emit_symbol(Symbol::RightBrace));
                }
                '[' => {
                    self.advance();
                    return Some(self.emit_symbol(Symbol::LeftBracket));
                }
                ']' => {
                    self.advance();
                    return Some(self.emit_symbol(Symbol::RightBracket));
                }
                '/' => {
                    self.advance();
                    return Some(self.emit_symbol(Symbol::Slash));
                }
                '^' => {
                    self.advance();
                    return Some(self.emit_symbol(Symbol::Caret));
                }
                ch => {
                    self.advance();
                    continue;
                }
            }
        }
    }
}

fn is_identifier_start(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphabetic()
}

fn is_identifier_continue(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphanumeric()
}

impl<'a> Lexer<'a> {
    /// Convenience helper to lex an entire string into tokens.
    pub fn lex_all(src: &'a str) -> Vec<Token> {
        Lexer::new(src).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use x3_common::{Keyword, Symbol};

    #[test]
    fn test_lex_fixture() {
        let source = r#"let x = 1;
fn main() {
    return x;
}"#;
        let tokens = Lexer::lex_all(source);
        for token in &tokens {
            println!("{:?}", token);
        }
        // Check that "return" is tokenized as Keyword::Return
        let return_token = tokens
            .iter()
            .find(|t| matches!(t.kind, TokenKind::Keyword(Keyword::Return)))
            .unwrap();
        assert_eq!(return_token.kind, TokenKind::Keyword(Keyword::Return));
    }

    #[test]
    fn test_lex_range() {
        let source = "0..10";
        let tokens = Lexer::lex_all(source);
        for token in &tokens {
            println!("{:?}", token);
        }
        // Should be: Integer(0), Dot, Dot, Integer(10), Eof
        assert_eq!(tokens.len(), 5);
        assert!(matches!(
            tokens[0].kind,
            TokenKind::Literal(Literal::Integer(0))
        ));
        assert!(matches!(tokens[1].kind, TokenKind::Symbol(Symbol::Dot)));
        assert!(matches!(tokens[2].kind, TokenKind::Symbol(Symbol::Dot)));
        assert!(matches!(
            tokens[3].kind,
            TokenKind::Literal(Literal::Integer(10))
        ));
    }
}
