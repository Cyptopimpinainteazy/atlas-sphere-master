// crates/x3-compiler/src/lexer.rs
// Tokenization for X3

use anyhow::{anyhow, Result};
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Literals
    Number(u128),
    String(String),
    Ident(String),

    // Keywords
    Fn,
    Pub,
    Import,
    Export,
    Let,
    Mut,
    If,
    Else,
    While,
    For,
    Return,
    Struct,
    Enum,
    Event,
    Error,
    Extern,
    Test,
    Strategy,
    Match,
    Some,
    None,
    True,
    False,
    As,
    Self_,

    // Types
    U8,
    U16,
    U32,
    U64,
    U128,
    Bool,
    String_,
    Bytes,
    Option,

    // Attributes
    At,
    AtVm,
    AtAi,

    // Operators
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Eq,
    EqEq,
    Bang,
    BangEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    AmpAmp,
    PipePipe,
    Amp,
    Pipe,
    Caret,
    LtLt,
    GtGt,

    // Punctuation
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    LeftBracket,
    RightBracket,
    Semicolon,
    Colon,
    ColonColon,
    Comma,
    Dot,
    Arrow,
    FatArrow,
    Question,

    // Special
    Eof,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Token::Ident(s) => write!(f, "ident({})", s),
            Token::Number(n) => write!(f, "num({})", n),
            Token::String(s) => write!(f, "str({})", s),
            _ => write!(f, "{:?}", self),
        }
    }
}

pub fn tokenize(input: &str) -> Result<Vec<Token>> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();

    while let Some(&ch) = chars.peek() {
        match ch {
            // Skip whitespace
            ' ' | '\t' | '\n' | '\r' => {
                chars.next();
            }
            // Comments
            '/' if chars.clone().nth(1) == Some('/') => {
                chars.next();
                chars.next();
                while chars.peek().is_some() && chars.peek() != Some(&'\n') {
                    chars.next();
                }
            }
            // Strings
            '"' => {
                chars.next();
                let mut s = String::new();
                while let Some(&c) = chars.peek() {
                    if c == '"' {
                        chars.next();
                        break;
                    }
                    s.push(c);
                    chars.next();
                }
                tokens.push(Token::String(s));
            }
            // Numbers
            '0'..='9' => {
                let mut num_str = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_numeric() {
                        num_str.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }
                let num: u128 = num_str.parse()?;
                tokens.push(Token::Number(num));
            }
            // Identifiers and keywords
            'a'..='z' | 'A'..='Z' | '_' => {
                let mut ident = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_alphanumeric() || c == '_' {
                        ident.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }
                let token = match ident.as_str() {
                    "fn" => Token::Fn,
                    "pub" => Token::Pub,
                    "import" => Token::Import,
                    "export" => Token::Export,
                    "let" => Token::Let,
                    "mut" => Token::Mut,
                    "if" => Token::If,
                    "else" => Token::Else,
                    "while" => Token::While,
                    "for" => Token::For,
                    "return" => Token::Return,
                    "struct" => Token::Struct,
                    "enum" => Token::Enum,
                    "event" => Token::Event,
                    "error" => Token::Error,
                    "extern" => Token::Extern,
                    "test" => Token::Test,
                    "strategy" => Token::Strategy,
                    "match" => Token::Match,
                    "Some" => Token::Some,
                    "None" => Token::None,
                    "true" => Token::True,
                    "false" => Token::False,
                    "as" => Token::As,
                    "self" => Token::Self_,
                    "u8" => Token::U8,
                    "u16" => Token::U16,
                    "u32" => Token::U32,
                    "u64" => Token::U64,
                    "u128" => Token::U128,
                    "bool" => Token::Bool,
                    "string" => Token::String_,
                    "bytes" => Token::Bytes,
                    "Option" => Token::Option,
                    _ => Token::Ident(ident),
                };
                tokens.push(token);
            }
            // Operators and punctuation
            '+' => {
                tokens.push(Token::Plus);
                chars.next();
            }
            '-' => {
                chars.next();
                if chars.peek() == Some(&'>') {
                    chars.next();
                    tokens.push(Token::Arrow);
                } else {
                    tokens.push(Token::Minus);
                }
            }
            '*' => {
                tokens.push(Token::Star);
                chars.next();
            }
            '/' => {
                tokens.push(Token::Slash);
                chars.next();
            }
            '%' => {
                tokens.push(Token::Percent);
                chars.next();
            }
            '=' => {
                chars.next();
                if chars.peek() == Some(&'=') {
                    chars.next();
                    tokens.push(Token::EqEq);
                } else if chars.peek() == Some(&'>') {
                    chars.next();
                    tokens.push(Token::FatArrow);
                } else {
                    tokens.push(Token::Eq);
                }
            }
            '!' => {
                chars.next();
                if chars.peek() == Some(&'=') {
                    chars.next();
                    tokens.push(Token::BangEq);
                } else {
                    tokens.push(Token::Bang);
                }
            }
            '<' => {
                chars.next();
                if chars.peek() == Some(&'=') {
                    chars.next();
                    tokens.push(Token::LtEq);
                } else if chars.peek() == Some(&'<') {
                    chars.next();
                    tokens.push(Token::LtLt);
                } else {
                    tokens.push(Token::Lt);
                }
            }
            '>' => {
                chars.next();
                if chars.peek() == Some(&'=') {
                    chars.next();
                    tokens.push(Token::GtEq);
                } else if chars.peek() == Some(&'>') {
                    chars.next();
                    tokens.push(Token::GtGt);
                } else {
                    tokens.push(Token::Gt);
                }
            }
            '&' => {
                chars.next();
                if chars.peek() == Some(&'&') {
                    chars.next();
                    tokens.push(Token::AmpAmp);
                } else {
                    tokens.push(Token::Amp);
                }
            }
            '|' => {
                chars.next();
                if chars.peek() == Some(&'|') {
                    chars.next();
                    tokens.push(Token::PipePipe);
                } else {
                    tokens.push(Token::Pipe);
                }
            }
            '^' => {
                tokens.push(Token::Caret);
                chars.next();
            }
            '(' => {
                tokens.push(Token::LeftParen);
                chars.next();
            }
            ')' => {
                tokens.push(Token::RightParen);
                chars.next();
            }
            '{' => {
                tokens.push(Token::LeftBrace);
                chars.next();
            }
            '}' => {
                tokens.push(Token::RightBrace);
                chars.next();
            }
            '[' => {
                tokens.push(Token::LeftBracket);
                chars.next();
            }
            ']' => {
                tokens.push(Token::RightBracket);
                chars.next();
            }
            ';' => {
                tokens.push(Token::Semicolon);
                chars.next();
            }
            ':' => {
                chars.next();
                if chars.peek() == Some(&':') {
                    chars.next();
                    tokens.push(Token::ColonColon);
                } else {
                    tokens.push(Token::Colon);
                }
            }
            ',' => {
                tokens.push(Token::Comma);
                chars.next();
            }
            '.' => {
                tokens.push(Token::Dot);
                chars.next();
            }
            '?' => {
                tokens.push(Token::Question);
                chars.next();
            }
            '@' => {
                tokens.push(Token::At);
                chars.next();
            }
            _ => {
                return Err(anyhow!("Unexpected character: {}", ch));
            }
        }
    }

    tokens.push(Token::Eof);
    Ok(tokens)
}
