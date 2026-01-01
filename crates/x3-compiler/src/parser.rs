// crates/x3-compiler/src/parser.rs
// Parse tokens into AST

use crate::ast::*;
use crate::lexer::Token;
use anyhow::{anyhow, Result};

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, pos: 0 }
    }

    fn current(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or(&Token::Eof)
    }

    fn peek(&self, n: usize) -> &Token {
        self.tokens.get(self.pos + n).unwrap_or(&Token::Eof)
    }

    fn advance(&mut self) -> Token {
        let t = self.current().clone();
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
        t
    }

    fn expect(&mut self, expected: Token) -> Result<()> {
        if std::mem::discriminant(self.current()) == std::mem::discriminant(&expected) {
            self.advance();
            Ok(())
        } else {
            Err(anyhow!("Expected {:?}, got {:?}", expected, self.current()))
        }
    }

    fn parse_program(&mut self) -> Result<Program> {
        let mut modules = Vec::new();
        while self.current() != &Token::Eof {
            modules.push(self.parse_module()?);
        }
        Ok(Program { modules })
    }

    fn parse_module(&mut self) -> Result<Module> {
        let mut imports = Vec::new();
        let mut items = Vec::new();

        while let Token::Import = self.current() {
            self.advance();
            imports.push(self.parse_import()?);
        }

        while self.current() != &Token::Eof {
            items.push(self.parse_item()?);
        }

        Ok(Module {
            name: "main".to_string(),
            imports,
            items,
        })
    }

    fn parse_import(&mut self) -> Result<Import> {
        let mut path = String::new();
        if let Token::Ident(s) = self.current() {
            path = s.clone();
            self.advance();
        }

        let mut items = Vec::new();
        if self.current() == &Token::ColonColon {
            self.advance();
            if self.current() == &Token::Star {
                self.advance();
                items.push("*".to_string());
            }
        }

        self.expect(Token::Semicolon)?;
        Ok(Import { path, items })
    }

    fn parse_item(&mut self) -> Result<Item> {
        // Skip attributes (@vm.hint, @ai.hint, etc.)
        while self.current() == &Token::At {
            self.skip_attribute()?;
        }

        // Skip export/pub visibility modifiers
        while matches!(self.current(), Token::Export | Token::Pub) {
            self.advance();
        }

        match self.current() {
            Token::Fn => {
                self.advance();
                Ok(Item::Function(self.parse_function()?))
            }
            Token::Struct => {
                self.advance();
                Ok(Item::Struct(self.parse_struct()?))
            }
            Token::Enum => {
                self.advance();
                Ok(Item::Enum(self.parse_enum()?))
            }
            Token::Event => {
                self.advance();
                Ok(Item::Event(self.parse_event()?))
            }
            Token::Error => {
                self.advance();
                Ok(Item::Error(self.parse_error()?))
            }
            Token::Strategy => {
                self.advance();
                Ok(Item::Strategy(self.parse_strategy()?))
            }
            _ => Err(anyhow!("Unexpected token in item: {:?}", self.current())),
        }
    }

    fn skip_attribute(&mut self) -> Result<()> {
        self.expect(Token::At)?;
        // Skip attribute name (e.g., vm, ai, etc.)
        if let Token::Ident(_) = self.current() {
            self.advance();
        }
        // Skip attribute dot access (e.g., vm.hint)
        if self.current() == &Token::Dot {
            self.advance();
            if let Token::Ident(_) = self.current() {
                self.advance();
            }
        }
        // Skip attribute arguments
        if self.current() == &Token::LeftParen {
            self.advance();
            let mut paren_depth = 1;
            while paren_depth > 0 && self.current() != &Token::Eof {
                match self.current() {
                    Token::LeftParen => paren_depth += 1,
                    Token::RightParen => paren_depth -= 1,
                    _ => {}
                }
                self.advance();
            }
        }
        Ok(())
    }

    fn parse_function(&mut self) -> Result<Function> {
        let is_pub = if self.current() == &Token::Pub {
            self.advance();
            true
        } else {
            false
        };

        let name = if let Token::Ident(s) = self.current() {
            s.clone()
        } else {
            return Err(anyhow!("Expected function name"));
        };
        self.advance();

        self.expect(Token::LeftParen)?;
        let mut params = Vec::new();
        while self.current() != &Token::RightParen {
            if let Token::Ident(pname) = self.current() {
                let pname = pname.clone();
                self.advance();
                self.expect(Token::Colon)?;
                let ptype = self.parse_type()?;
                params.push((pname, ptype));

                if self.current() == &Token::Comma {
                    self.advance();
                }
            }
        }
        self.expect(Token::RightParen)?;

        let return_type = if self.current() == &Token::Arrow {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };

        self.expect(Token::LeftBrace)?;
        let mut body = Vec::new();
        while self.current() != &Token::RightBrace {
            body.push(self.parse_statement()?);
        }
        self.expect(Token::RightBrace)?;

        Ok(Function {
            name,
            is_pub,
            is_extern: false,
            params,
            return_type,
            body,
            attributes: Vec::new(),
        })
    }

    fn parse_struct(&mut self) -> Result<StructDef> {
        let name = if let Token::Ident(s) = self.current() {
            s.clone()
        } else {
            return Err(anyhow!("Expected struct name"));
        };
        self.advance();

        self.expect(Token::LeftBrace)?;
        let mut fields = Vec::new();
        while self.current() != &Token::RightBrace {
            if let Token::Ident(fname) = self.current() {
                let fname = fname.clone();
                self.advance();
                self.expect(Token::Colon)?;
                let ftype = self.parse_type()?;
                fields.push((fname, ftype));

                if self.current() == &Token::Comma {
                    self.advance();
                }
            }
        }
        self.expect(Token::RightBrace)?;

        Ok(StructDef { name, fields })
    }

    fn parse_enum(&mut self) -> Result<EnumDef> {
        let name = if let Token::Ident(s) = self.current() {
            s.clone()
        } else {
            return Err(anyhow!("Expected enum name"));
        };
        self.advance();

        self.expect(Token::LeftBrace)?;
        let mut variants = Vec::new();
        while self.current() != &Token::RightBrace {
            if let Token::Ident(vname) = self.current() {
                let vname = vname.clone();
                self.advance();
                let mut vtypes = Vec::new();
                if self.current() == &Token::LeftParen {
                    self.advance();
                    while self.current() != &Token::RightParen {
                        vtypes.push(self.parse_type()?);
                        if self.current() == &Token::Comma {
                            self.advance();
                        }
                    }
                    self.expect(Token::RightParen)?;
                }
                variants.push((vname, vtypes));

                if self.current() == &Token::Comma {
                    self.advance();
                }
            }
        }
        self.expect(Token::RightBrace)?;

        Ok(EnumDef { name, variants })
    }

    fn parse_event(&mut self) -> Result<EventDef> {
        let name = if let Token::Ident(s) = self.current() {
            s.clone()
        } else {
            return Err(anyhow!("Expected event name"));
        };
        self.advance();

        self.expect(Token::LeftParen)?;
        let mut fields = Vec::new();
        while self.current() != &Token::RightParen {
            if let Token::Ident(fname) = self.current() {
                let fname = fname.clone();
                self.advance();
                self.expect(Token::Colon)?;
                let ftype = self.parse_type()?;
                fields.push((fname, ftype));

                if self.current() == &Token::Comma {
                    self.advance();
                }
            }
        }
        self.expect(Token::RightParen)?;
        self.expect(Token::Semicolon)?;

        Ok(EventDef { name, fields })
    }

    fn parse_error(&mut self) -> Result<ErrorDef> {
        let name = if let Token::Ident(s) = self.current() {
            s.clone()
        } else {
            return Err(anyhow!("Expected error name"));
        };
        self.advance();

        self.expect(Token::LeftParen)?;
        let message = if let Token::String(s) = self.current() {
            s.clone()
        } else {
            String::new()
        };
        self.advance();
        self.expect(Token::RightParen)?;
        self.expect(Token::Semicolon)?;

        Ok(ErrorDef { name, message })
    }

    fn parse_strategy(&mut self) -> Result<Strategy> {
        let name = if let Token::Ident(s) = self.current() {
            s.clone()
        } else {
            return Err(anyhow!("Expected strategy name"));
        };
        self.advance();

        self.expect(Token::LeftBrace)?;
        let mut fields = Vec::new();
        let mut methods = Vec::new();

        while self.current() != &Token::RightBrace {
            if self.current() == &Token::Fn {
                self.advance();
                methods.push(self.parse_function()?);
            } else if let Token::Ident(fname) = self.current() {
                let fname = fname.clone();
                self.advance();
                self.expect(Token::Colon)?;
                let ftype = self.parse_type()?;
                self.expect(Token::Eq)?;
                let value = self.parse_expr()?;
                self.expect(Token::Semicolon)?;
                fields.push((fname, ftype, value));
            }
        }
        self.expect(Token::RightBrace)?;

        Ok(Strategy {
            name,
            fields,
            methods,
        })
    }

    fn parse_type(&mut self) -> Result<Type> {
        match self.current() {
            Token::U8 => {
                self.advance();
                Ok(Type::U8)
            }
            Token::U16 => {
                self.advance();
                Ok(Type::U16)
            }
            Token::U32 => {
                self.advance();
                Ok(Type::U32)
            }
            Token::U64 => {
                self.advance();
                Ok(Type::U64)
            }
            Token::U128 => {
                self.advance();
                Ok(Type::U128)
            }
            Token::Bool => {
                self.advance();
                Ok(Type::Bool)
            }
            Token::String_ => {
                self.advance();
                Ok(Type::String)
            }
            Token::Bytes => {
                self.advance();
                if let Token::Number(n) = self.current() {
                    match n {
                        20 => {
                            self.advance();
                            Ok(Type::Bytes20)
                        }
                        32 => {
                            self.advance();
                            Ok(Type::Bytes32)
                        }
                        _ => Ok(Type::Bytes),
                    }
                } else {
                    Ok(Type::Bytes)
                }
            }
            Token::Option => {
                self.advance();
                self.expect(Token::Lt)?;
                let inner = self.parse_type()?;
                self.expect(Token::Gt)?;
                Ok(Type::Option(Box::new(inner)))
            }
            Token::Ident(name) => {
                let name = name.clone();
                self.advance();
                Ok(Type::Struct(name))
            }
            Token::LeftBracket => {
                self.advance();
                let inner = self.parse_type()?;
                self.expect(Token::RightBracket)?;
                Ok(Type::Array(Box::new(inner)))
            }
            _ => Err(anyhow!("Unexpected token in type: {:?}", self.current())),
        }
    }

    fn parse_statement(&mut self) -> Result<Statement> {
        match self.current() {
            Token::Let => {
                self.advance();
                let name = if let Token::Ident(s) = self.current() {
                    s.clone()
                } else {
                    return Err(anyhow!("Expected variable name"));
                };
                self.advance();

                let ty = if self.current() == &Token::Colon {
                    self.advance();
                    Some(self.parse_type()?)
                } else {
                    None
                };

                self.expect(Token::Eq)?;
                let expr = self.parse_expr()?;
                self.expect(Token::Semicolon)?;

                Ok(Statement::Let(name, ty, expr))
            }
            Token::If => {
                self.advance();
                let cond = self.parse_expr()?;
                self.expect(Token::LeftBrace)?;
                let mut body = Vec::new();
                while self.current() != &Token::RightBrace {
                    body.push(self.parse_statement()?);
                }
                self.expect(Token::RightBrace)?;

                let else_body = if self.current() == &Token::Else {
                    self.advance();
                    self.expect(Token::LeftBrace)?;
                    let mut eb = Vec::new();
                    while self.current() != &Token::RightBrace {
                        eb.push(self.parse_statement()?);
                    }
                    self.expect(Token::RightBrace)?;
                    Some(eb)
                } else {
                    None
                };

                Ok(Statement::If(cond, body, else_body))
            }
            Token::Return => {
                self.advance();
                let expr = if self.current() == &Token::Semicolon {
                    None
                } else {
                    Some(self.parse_expr()?)
                };
                self.expect(Token::Semicolon)?;
                Ok(Statement::Return(expr))
            }
            _ => {
                let expr = self.parse_expr()?;
                // Semicolon is optional for some statements
                if self.current() == &Token::Semicolon {
                    self.advance();
                }
                Ok(Statement::Expr(expr))
            }
        }
    }

    fn parse_expr(&mut self) -> Result<Expr> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<Expr> {
        let mut left = self.parse_and()?;
        while self.current() == &Token::PipePipe {
            self.advance();
            let right = self.parse_and()?;
            left = Expr::Binary(BinOp::Or, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr> {
        let mut left = self.parse_comparison()?;
        while self.current() == &Token::AmpAmp {
            self.advance();
            let right = self.parse_comparison()?;
            left = Expr::Binary(BinOp::And, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<Expr> {
        let mut left = self.parse_additive()?;
        while let Some(op) = self.match_comparison_op() {
            self.advance();
            let right = self.parse_additive()?;
            left = Expr::Binary(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn match_comparison_op(&self) -> Option<BinOp> {
        match self.current() {
            Token::EqEq => Some(BinOp::Eq),
            Token::BangEq => Some(BinOp::Ne),
            Token::Lt => Some(BinOp::Lt),
            Token::Gt => Some(BinOp::Gt),
            Token::LtEq => Some(BinOp::Le),
            Token::GtEq => Some(BinOp::Ge),
            _ => None,
        }
    }

    fn parse_additive(&mut self) -> Result<Expr> {
        let mut left = self.parse_multiplicative()?;
        while let Some(op) = self.match_additive_op() {
            self.advance();
            let right = self.parse_multiplicative()?;
            left = Expr::Binary(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn match_additive_op(&self) -> Option<BinOp> {
        match self.current() {
            Token::Plus => Some(BinOp::Add),
            Token::Minus => Some(BinOp::Sub),
            _ => None,
        }
    }

    fn parse_multiplicative(&mut self) -> Result<Expr> {
        let mut left = self.parse_unary()?;
        while let Some(op) = self.match_multiplicative_op() {
            self.advance();
            let right = self.parse_unary()?;
            left = Expr::Binary(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn match_multiplicative_op(&self) -> Option<BinOp> {
        match self.current() {
            Token::Star => Some(BinOp::Mul),
            Token::Slash => Some(BinOp::Div),
            Token::Percent => Some(BinOp::Mod),
            _ => None,
        }
    }

    fn parse_unary(&mut self) -> Result<Expr> {
        match self.current() {
            Token::Bang => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(Expr::Unary(UnOp::Not, Box::new(expr)))
            }
            Token::Minus => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(Expr::Unary(UnOp::Neg, Box::new(expr)))
            }
            _ => self.parse_postfix(),
        }
    }

    fn parse_postfix(&mut self) -> Result<Expr> {
        let mut expr = self.parse_primary()?;

        loop {
            match self.current() {
                Token::LeftParen => {
                    self.advance();
                    let mut args = Vec::new();
                    while self.current() != &Token::RightParen {
                        args.push(self.parse_expr()?);
                        if self.current() == &Token::Comma {
                            self.advance();
                        }
                    }
                    self.expect(Token::RightParen)?;
                    expr = match expr {
                        Expr::Ident(name) => Expr::Call(name, args),
                        _ => return Err(anyhow!("Invalid function call")),
                    };
                }
                Token::Dot => {
                    self.advance();
                    if let Token::Ident(name) = self.current() {
                        let name = name.clone();
                        self.advance();
                        expr = Expr::FieldAccess(Box::new(expr), name);
                    }
                }
                Token::LeftBracket => {
                    self.advance();
                    let idx = self.parse_expr()?;
                    self.expect(Token::RightBracket)?;
                    expr = Expr::Index(Box::new(expr), Box::new(idx));
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr> {
        match self.current() {
            Token::Number(n) => {
                let n = *n;
                self.advance();
                Ok(Expr::Literal(Literal::Number(n)))
            }
            Token::String(s) => {
                let s = s.clone();
                self.advance();
                Ok(Expr::Literal(Literal::String(s)))
            }
            Token::True => {
                self.advance();
                Ok(Expr::Literal(Literal::Bool(true)))
            }
            Token::False => {
                self.advance();
                Ok(Expr::Literal(Literal::Bool(false)))
            }
            Token::None => {
                self.advance();
                Ok(Expr::Literal(Literal::None))
            }
            Token::Ident(name) => {
                let name = name.clone();
                self.advance();
                Ok(Expr::Ident(name))
            }
            Token::LeftParen => {
                self.advance();
                let expr = self.parse_expr()?;
                self.expect(Token::RightParen)?;
                Ok(expr)
            }
            Token::LeftBracket => {
                self.advance();
                let mut elements = Vec::new();
                while self.current() != &Token::RightBracket {
                    elements.push(self.parse_expr()?);
                    if self.current() == &Token::Comma {
                        self.advance();
                    }
                }
                self.expect(Token::RightBracket)?;
                Ok(Expr::Array(elements))
            }
            _ => Err(anyhow!(
                "Unexpected token in expression: {:?}",
                self.current()
            )),
        }
    }
}

pub fn parse(tokens: Vec<Token>) -> Result<Program> {
    let mut parser = Parser::new(tokens);
    parser.parse_program()
}
