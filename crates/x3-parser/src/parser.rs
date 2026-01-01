use crate::cursor::Cursor;
use crate::error::{ParseError, ParseResult};
use crate::grammar::{binary_binding_power, symbol_to_binary_op, PREFIX_BP};
use crate::tokens::MAX_NESTING_DEPTH;
use x3_ast::{
    Agent, AssignExpression, AtomicBlock, BinaryExpression, Block, BreakStatement, CallExpression,
    Const, ContinueStatement, EmitStatement, Expression, FieldAccessExpression, ForLoopKind,
    ForStatement, Function, GlobalLet, Identifier, IfStatement, Item, LetStatement,
    LiteralExpression, LoopStatement, Module, Param, RangeExpression, Statement, TypeAnnotation,
    UnaryExpression, UnaryOp, WhileStatement,
};
use x3_common::{Keyword, Literal, Span, Symbol, Token, TokenKind};
use x3_lexer::Lexer;

/// Convenient entrypoint for parsing a module.
pub struct Parser {
    cursor: Cursor,
    nesting_depth: usize,
}

impl Parser {
    /// Create a parser from the raw source text.
    pub fn from_source(source: &str) -> Self {
        Self {
            cursor: Cursor::new(Lexer::lex_all(source)),
            nesting_depth: 0,
        }
    }

    pub fn parse_module(&mut self) -> ParseResult<Module> {
        let start_span = self.peek().span;
        let mut items = Vec::new();
        while !self.peek_kind_is_eof() {
            items.push(self.parse_item()?);
        }
        let span = start_span.merge(self.peek().span);
        Ok(Module { items, span })
    }

    fn parse_item(&mut self) -> ParseResult<Item> {
        match &self.peek().kind {
            TokenKind::Keyword(Keyword::Fn) => self.parse_function_item(),
            TokenKind::Keyword(Keyword::Agent) => self.parse_agent_item(),
            TokenKind::Keyword(Keyword::Const) => self.parse_const_item(),
            TokenKind::Keyword(Keyword::Let) => self.parse_global_let(),
            TokenKind::Eof => Err(ParseError::new("unexpected end of input", self.peek().span)),
            token => Err(ParseError::new(
                format!("unexpected token {:?} at top-level", token),
                self.peek().span,
            )),
        }
    }

    fn parse_function_item(&mut self) -> ParseResult<Item> {
        let fn_token = self.expect_keyword(Keyword::Fn)?;
        let name = self.expect_identifier()?;
        let _ = self.expect_symbol(Symbol::LeftParen)?;
        let params = self.parse_parameter_list()?;
        let _ = self.expect_symbol(Symbol::RightParen)?;
        let ret_ty = if self.accept_symbol(Symbol::Arrow) {
            Some(self.parse_type_annotation()?)
        } else {
            None
        };
        let body = self.parse_block()?;
        let span = fn_token.span.merge(body.span);
        Ok(Item::Function(Function {
            name,
            params,
            ret_ty,
            body,
            span,
        }))
    }

    fn parse_global_let(&mut self) -> ParseResult<Item> {
        let let_token = self.expect_keyword(Keyword::Let)?;
        let mutable = self.accept_keyword(Keyword::Mut);
        let name = self.expect_identifier()?;
        let ty = if self.accept_symbol(Symbol::Colon) {
            Some(self.parse_type_annotation()?)
        } else {
            None
        };
        self.expect_symbol(Symbol::Equals)?;
        let initializer = self.parse_expression_bp(0)?;
        let _ = self.expect_symbol(Symbol::Semicolon)?;
        let span = let_token.span.merge(initializer.span());
        Ok(Item::GlobalLet(GlobalLet {
            name,
            mutable,
            ty,
            initializer,
            span,
        }))
    }

    fn parse_const_item(&mut self) -> ParseResult<Item> {
        self.expect_keyword(Keyword::Const)?;
        let name = self.expect_identifier()?;
        self.expect_symbol(Symbol::Colon)?;
        let ty = self.parse_type_annotation()?;
        self.expect_symbol(Symbol::Equals)?;
        let value = self.parse_expression_bp(0)?;
        self.expect_symbol(Symbol::Semicolon)?;
        Ok(Item::Const(Const {
            name,
            ty,
            value: Box::new(value),
            span: Span::new(0, 0), // TODO: proper span
        }))
    }

    fn parse_agent_item(&mut self) -> ParseResult<Item> {
        self.expect_keyword(Keyword::Agent)?;
        let name = self.expect_identifier()?;
        self.expect_symbol(Symbol::LeftBrace)?;
        let mut items = Vec::new();
        while !self.peek_symbol(Symbol::RightBrace) {
            items.push(self.parse_item()?);
        }
        self.expect_symbol(Symbol::RightBrace)?;
        Ok(Item::Agent(Agent {
            name,
            items,
            span: Span::new(0, 0), // TODO: proper span
        }))
    }

    fn parse_parameter_list(&mut self) -> ParseResult<Vec<Param>> {
        let mut params = Vec::new();
        if self.peek_symbol(Symbol::RightParen) {
            return Ok(params);
        }
        loop {
            let mutable = self.accept_keyword(Keyword::Mut);
            let name = self.expect_identifier()?;
            let ty = if self.accept_symbol(Symbol::Colon) {
                Some(self.parse_type_annotation()?)
            } else {
                None
            };
            let span = name.span;
            params.push(Param {
                name,
                mutable,
                ty,
                span,
            });
            if !self.accept_symbol(Symbol::Comma) {
                break;
            }
        }
        Ok(params)
    }

    fn parse_type_annotation(&mut self) -> ParseResult<TypeAnnotation> {
        let identifier = self.expect_identifier()?;
        let span = identifier.span;
        Ok(TypeAnnotation {
            name: identifier,
            span,
        })
    }

    fn parse_block(&mut self) -> ParseResult<Block> {
        let left = self.expect_symbol(Symbol::LeftBrace)?;
        let mut statements = Vec::new();
        while !self.accept_symbol(Symbol::RightBrace) {
            if matches!(self.peek().kind, TokenKind::Eof) {
                return Err(ParseError::new("missing closing brace", self.peek().span));
            }
            statements.push(self.parse_statement()?);
        }
        let span = left.span.merge(self.previous_span());
        Ok(Block { statements, span })
    }

    fn parse_statement(&mut self) -> ParseResult<Statement> {
        match &self.peek().kind {
            TokenKind::Keyword(Keyword::Let) => self.parse_let_statement().map(Statement::Let),
            TokenKind::Keyword(Keyword::If) => self.parse_if_statement().map(Statement::If),
            TokenKind::Keyword(Keyword::While) => {
                self.parse_while_statement().map(Statement::While)
            }
            TokenKind::Keyword(Keyword::Loop) => self.parse_loop_statement().map(Statement::Loop),
            TokenKind::Keyword(Keyword::For) => self.parse_for_statement().map(Statement::For),
            TokenKind::Keyword(Keyword::Atomic) => self.parse_atomic_block().map(Statement::Atomic),
            TokenKind::Keyword(Keyword::Emit) => self.parse_emit_statement().map(Statement::Emit),
            TokenKind::Keyword(Keyword::Break) => {
                self.parse_break_statement().map(Statement::Break)
            }
            TokenKind::Keyword(Keyword::Continue) => {
                self.parse_continue_statement().map(Statement::Continue)
            }
            TokenKind::Keyword(Keyword::Return) => self.parse_return_statement(),
            _ => {
                let expr = self.parse_expression_bp(0)?;
                self.expect_symbol(Symbol::Semicolon)?;
                Ok(Statement::Expr(expr))
            }
        }
    }

    fn parse_let_statement(&mut self) -> ParseResult<LetStatement> {
        let let_token = self.expect_keyword(Keyword::Let)?;
        let mutable = self.accept_keyword(Keyword::Mut);
        let name = self.expect_identifier()?;
        let ty = if self.accept_symbol(Symbol::Colon) {
            Some(self.parse_type_annotation()?)
        } else {
            None
        };
        self.expect_symbol(Symbol::Equals)?;
        let initializer = self.parse_expression_bp(0)?;
        self.expect_symbol(Symbol::Semicolon)?;
        let span = let_token.span.merge(initializer.span());
        Ok(LetStatement {
            name,
            mutable,
            ty,
            initializer,
            span,
        })
    }

    fn parse_if_statement(&mut self) -> ParseResult<IfStatement> {
        let if_token = self.expect_keyword(Keyword::If)?;
        let condition = if self.accept_symbol(Symbol::LeftParen) {
            let expr = self.parse_expression_bp(0)?;
            self.expect_symbol(Symbol::RightParen)?;
            expr
        } else {
            self.parse_expression_bp(0)?
        };
        let then_block = self.parse_block()?;
        let else_block = if self.accept_keyword(Keyword::Else) {
            Some(self.parse_block()?)
        } else {
            None
        };
        let span = if_token.span.merge(then_block.span);
        Ok(IfStatement {
            condition,
            then_block,
            else_block,
            span,
        })
    }

    fn parse_while_statement(&mut self) -> ParseResult<WhileStatement> {
        let while_token = self.expect_keyword(Keyword::While)?;
        let condition = if self.accept_symbol(Symbol::LeftParen) {
            let expr = self.parse_expression_bp(0)?;
            self.expect_symbol(Symbol::RightParen)?;
            expr
        } else {
            self.parse_expression_bp(0)?
        };
        let body = self.parse_block()?;
        let span = while_token.span.merge(body.span);
        Ok(WhileStatement {
            condition,
            body,
            span,
        })
    }

    fn parse_return_statement(&mut self) -> ParseResult<Statement> {
        let return_token = self.expect_keyword(Keyword::Return)?;
        let value = if self.peek_symbol(Symbol::Semicolon) {
            None
        } else {
            Some(self.parse_expression_bp(0)?)
        };
        self.expect_symbol(Symbol::Semicolon)?;
        let span = if let Some(expr) = &value {
            return_token.span.merge(expr.span())
        } else {
            return_token.span
        };
        Ok(Statement::Return(value, span))
    }

    fn parse_loop_statement(&mut self) -> ParseResult<LoopStatement> {
        let loop_token = self.expect_keyword(Keyword::Loop)?;
        let body = self.parse_block()?;
        let span = loop_token.span.merge(body.span);
        Ok(LoopStatement { body, span })
    }

    fn parse_for_statement(&mut self) -> ParseResult<ForStatement> {
        let for_token = self.expect_keyword(Keyword::For)?;
        self.expect_symbol(Symbol::LeftParen)?;

        // Check if this is a range-based for loop: for (let var in range)
        if self.peek_keyword(Keyword::Let) {
            // Look ahead to see if there's an 'in' keyword after the let statement
            let mut lookahead = self.cursor.clone();
            // Skip 'let'
            lookahead.bump();
            // Skip optional 'mut'
            if lookahead.peek_keyword(Keyword::Mut) {
                lookahead.bump();
            }
            // Skip identifier
            if let TokenKind::Identifier(_) = lookahead.peek().kind {
                lookahead.bump();
                // Skip optional type annotation
                if lookahead.peek_symbol(Symbol::Colon) {
                    lookahead.bump();
                    // Skip type (simplified - just skip until = or in)
                    while !lookahead.peek_symbol(Symbol::Equals)
                        && !lookahead.peek_keyword(Keyword::In)
                        && !lookahead.is_eof()
                    {
                        lookahead.bump();
                    }
                }
                // Check for 'in' keyword
                if lookahead.peek_keyword(Keyword::In) {
                    // This is a range-based for loop
                    let _ = self.expect_keyword(Keyword::Let);
                    let mutable = self.accept_keyword(Keyword::Mut);
                    if mutable {
                        return Err(ParseError::new(
                            "range-based for loop variables cannot be mutable",
                            self.peek().span,
                        ));
                    }
                    let variable = self.expect_identifier()?;
                    // Skip optional type annotation (not allowed in range for)
                    if self.peek_symbol(Symbol::Colon) {
                        return Err(ParseError::new(
                            "range-based for loop variables cannot have explicit types",
                            self.peek().span,
                        ));
                    }
                    self.expect_keyword(Keyword::In)?;
                    let range_expr = self.parse_expression_bp(0)?;
                    let range = match range_expr {
                        Expression::Range(r) => r,
                        _ => {
                            return Err(ParseError::new(
                                "expected range expression after 'in'",
                                range_expr.span(),
                            ))
                        }
                    };
                    self.expect_symbol(Symbol::RightParen)?;
                    let body = self.parse_block()?;
                    let span = for_token.span.merge(body.span);
                    return Ok(ForStatement {
                        kind: ForLoopKind::Range { variable, range },
                        body,
                        span,
                    });
                }
            }
        }

        // C-style for loop: for (init; cond; update)
        // Initializer: let_stmt | expr_stmt | ";"
        let init = if self.accept_symbol(Symbol::Semicolon) {
            None
        } else if self.peek_keyword(Keyword::Let) {
            // Parse let statement without semicolon for for loop initializer
            let let_token = self.expect_keyword(Keyword::Let)?;
            let mutable = self.accept_keyword(Keyword::Mut);
            let name = self.expect_identifier()?;
            let ty = if self.accept_symbol(Symbol::Colon) {
                Some(self.parse_type_annotation()?)
            } else {
                None
            };
            self.expect_symbol(Symbol::Equals)?;
            let initializer = self.parse_expression_bp(0)?;
            let span = let_token.span.merge(initializer.span());
            self.expect_symbol(Symbol::Semicolon)?;
            Some(Box::new(Statement::Let(LetStatement {
                name,
                mutable,
                ty,
                initializer,
                span,
            })))
        } else {
            let expr = self.parse_expression_bp(0)?;
            self.expect_symbol(Symbol::Semicolon)?;
            Some(Box::new(Statement::Expr(expr)))
        };

        // Condition (optional)
        let condition = if self.accept_symbol(Symbol::Semicolon) {
            None
        } else {
            let expr = self.parse_expression_bp(0)?;
            self.expect_symbol(Symbol::Semicolon)?;
            Some(expr)
        };

        // Update (optional)
        let update = if self.peek_symbol(Symbol::RightParen) {
            None
        } else {
            Some(self.parse_expression_bp(0)?)
        };
        self.expect_symbol(Symbol::RightParen)?;

        let body = self.parse_block()?;
        let span = for_token.span.merge(body.span);
        Ok(ForStatement {
            kind: ForLoopKind::CStyle {
                init,
                condition,
                update,
            },
            body,
            span,
        })
    }

    fn parse_atomic_block(&mut self) -> ParseResult<AtomicBlock> {
        let atomic_token = self.expect_keyword(Keyword::Atomic)?;
        // atomic(expr) { ... } or atomic { ... }
        let metadata = if self.accept_symbol(Symbol::LeftParen) {
            let expr = self.parse_expression_bp(0)?;
            self.expect_symbol(Symbol::RightParen)?;
            Some(expr)
        } else {
            None
        };
        let body = self.parse_block()?;
        let span = atomic_token.span.merge(body.span);
        Ok(AtomicBlock {
            metadata,
            body,
            span,
        })
    }

    fn parse_emit_statement(&mut self) -> ParseResult<EmitStatement> {
        let emit_token = self.expect_keyword(Keyword::Emit)?;
        let value = self.parse_expression_bp(0)?;
        self.expect_symbol(Symbol::Semicolon)?;
        let span = emit_token.span.merge(self.previous_span());
        Ok(EmitStatement { value, span })
    }

    fn parse_break_statement(&mut self) -> ParseResult<BreakStatement> {
        let break_token = self.expect_keyword(Keyword::Break)?;
        self.expect_symbol(Symbol::Semicolon)?;
        Ok(BreakStatement {
            span: break_token.span,
        })
    }

    fn parse_continue_statement(&mut self) -> ParseResult<ContinueStatement> {
        let continue_token = self.expect_keyword(Keyword::Continue)?;
        self.expect_symbol(Symbol::Semicolon)?;
        Ok(ContinueStatement {
            span: continue_token.span,
        })
    }

    fn parse_expression_bp(&mut self, min_bp: u8) -> ParseResult<Expression> {
        // Track nesting depth for bounded memory
        self.nesting_depth += 1;
        if self.nesting_depth > MAX_NESTING_DEPTH {
            let span = self.peek().span;
            self.nesting_depth -= 1;
            return Err(ParseError::new(
                format!("maximum nesting depth ({}) exceeded", MAX_NESTING_DEPTH),
                span,
            ));
        }

        let result = self.parse_expression_bp_inner(min_bp);
        self.nesting_depth -= 1;
        result
    }

    fn parse_expression_bp_inner(&mut self, min_bp: u8) -> ParseResult<Expression> {
        let mut lhs = self.parse_prefix_expression()?;
        loop {
            let symbol = match self.peek().kind.clone() {
                TokenKind::Symbol(symbol) => symbol,
                _ => break,
            };

            if symbol == Symbol::Equals {
                return self.parse_assignment(lhs);
            }

            let bp = match binary_binding_power(symbol) {
                Some(bp) => bp,
                None => break,
            };

            if bp.left < min_bp {
                break;
            }

            let op_symbol = symbol;
            self.bump();
            let rhs = self.parse_expression_bp(bp.right)?;
            let span = lhs.span().merge(rhs.span());
            let op = symbol_to_binary_op(op_symbol).unwrap_or(x3_ast::BinaryOp::Equal);
            let binary = BinaryExpression {
                op,
                left: Box::new(lhs),
                right: Box::new(rhs),
                span,
            };
            lhs = Expression::Binary(binary);
        }
        Ok(lhs)
    }

    fn parse_assignment(&mut self, lhs: Expression) -> ParseResult<Expression> {
        let equals = self.expect_symbol(Symbol::Equals)?;
        match lhs {
            Expression::Identifier(ident) => {
                let rhs = self.parse_expression_bp(1)?;
                let span = ident.span.merge(rhs.span());
                Ok(Expression::Assign(AssignExpression {
                    target: ident,
                    value: Box::new(rhs),
                    span,
                }))
            }
            _ => Err(ParseError::new(
                "assignment target must be an identifier",
                equals.span,
            )),
        }
    }

    fn parse_prefix_expression(&mut self) -> ParseResult<Expression> {
        match self.peek().kind.clone() {
            TokenKind::Symbol(Symbol::Minus) => {
                let token = self.bump();
                let rhs = self.parse_expression_bp(PREFIX_BP)?;
                let span = token.span.merge(rhs.span());
                Ok(Expression::Unary(UnaryExpression {
                    op: UnaryOp::Negate,
                    expr: Box::new(rhs),
                    span,
                }))
            }
            TokenKind::Symbol(Symbol::Bang) => {
                let token = self.bump();
                let rhs = self.parse_expression_bp(PREFIX_BP)?;
                let span = token.span.merge(rhs.span());
                Ok(Expression::Unary(UnaryExpression {
                    op: UnaryOp::Not,
                    expr: Box::new(rhs),
                    span,
                }))
            }
            TokenKind::Literal(literal) => {
                let token = self.bump();
                let lit = LiteralExpression {
                    literal: literal.clone(),
                    span: token.span,
                };
                let expr = Expression::Literal(lit);
                Ok(self.finish_postfix(expr)?)
            }
            TokenKind::Keyword(Keyword::True) => {
                let token = self.bump();
                let lit = LiteralExpression {
                    literal: Literal::Bool(true),
                    span: token.span,
                };
                Ok(self.finish_postfix(Expression::Literal(lit))?)
            }
            TokenKind::Keyword(Keyword::False) => {
                let token = self.bump();
                let lit = LiteralExpression {
                    literal: Literal::Bool(false),
                    span: token.span,
                };
                Ok(self.finish_postfix(Expression::Literal(lit))?)
            }
            TokenKind::Identifier(_) => {
                let identifier = self.expect_identifier()?;
                Ok(self.finish_postfix(Expression::Identifier(identifier))?)
            }
            TokenKind::Symbol(Symbol::LeftParen) => {
                let _ = self.expect_symbol(Symbol::LeftParen)?;
                let expr = self.parse_expression_bp(0)?;
                self.expect_symbol(Symbol::RightParen)?;
                Ok(self.finish_postfix(expr)?)
            }
            other => Err(ParseError::new(
                format!("unexpected token {:?} in expression", other),
                self.peek().span,
            )),
        }
    }

    fn finish_postfix(&mut self, mut base: Expression) -> ParseResult<Expression> {
        loop {
            if self.peek_symbol(Symbol::LeftParen) {
                self.expect_symbol(Symbol::LeftParen)?;
                let mut args = Vec::new();
                if !self.peek_symbol(Symbol::RightParen) {
                    loop {
                        args.push(self.parse_expression_bp(0)?);
                        if !self.accept_symbol(Symbol::Comma) {
                            break;
                        }
                    }
                }
                let right_paren = self.expect_symbol(Symbol::RightParen)?;
                let span = base.span().merge(right_paren.span);
                base = Expression::Call(CallExpression {
                    callee: Box::new(base),
                    args,
                    span,
                });
            } else if self.peek_symbol(Symbol::Dot) {
                // Check if this is a range operator (..) by looking at the next token
                if self.cursor.peek_n(1).and_then(|t| t.kind.symbol()) == Some(Symbol::Dot) {
                    // Range operator ..
                    self.expect_symbol(Symbol::Dot)?;
                    self.expect_symbol(Symbol::Dot)?;
                    let end = self.parse_expression_bp(0)?;
                    let span = base.span().merge(end.span());
                    base = Expression::Range(RangeExpression {
                        start: Box::new(base),
                        end: Box::new(end),
                        span,
                    });
                } else {
                    // Field access
                    self.expect_symbol(Symbol::Dot)?;
                    let field = self.expect_identifier()?;
                    let span = base.span().merge(field.span);
                    base = Expression::FieldAccess(FieldAccessExpression {
                        object: Box::new(base),
                        field,
                        span,
                    });
                }
            } else {
                break;
            }
        }
        Ok(base)
    }

    fn expect_identifier(&mut self) -> ParseResult<Identifier> {
        match self.bump().kind {
            TokenKind::Identifier(name) => Ok(Identifier {
                name,
                span: self.previous_span(),
            }),
            other => Err(ParseError::new(
                format!("expected identifier but found {:?}", other),
                self.peek().span,
            )),
        }
    }

    fn peek(&self) -> &Token {
        self.cursor.peek()
    }

    fn peek_kind_is_eof(&self) -> bool {
        self.cursor.is_eof()
    }

    fn previous_span(&self) -> Span {
        self.cursor.previous_span()
    }

    fn peek_symbol(&self, symbol: Symbol) -> bool {
        self.cursor.peek_symbol(symbol)
    }

    fn accept_symbol(&mut self, symbol: Symbol) -> bool {
        self.cursor.match_symbol(symbol)
    }

    fn expect_symbol(&mut self, symbol: Symbol) -> ParseResult<Token> {
        self.cursor.consume_symbol(symbol).ok_or_else(|| {
            ParseError::new(format!("expected symbol {:?}", symbol), self.peek().span)
        })
    }

    fn expect_keyword(&mut self, keyword: Keyword) -> ParseResult<Token> {
        self.cursor.consume_keyword(keyword).ok_or_else(|| {
            ParseError::new(format!("expected keyword {:?}", keyword), self.peek().span)
        })
    }

    fn accept_keyword(&mut self, keyword: Keyword) -> bool {
        self.cursor.match_keyword(keyword)
    }

    fn bump(&mut self) -> Token {
        self.cursor.bump()
    }

    fn peek_keyword(&self, keyword: Keyword) -> bool {
        self.cursor.peek_keyword(keyword)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use x3_ast::{BinaryOp, Expression};

    #[test]
    fn parse_function_and_return() {
        let source = r#"
            fn add(x: i32, y: i32) -> i32 {
                let result = x + y;
                return result;
            }
        "#;
        let mut parser = Parser::from_source(source);
        let module = parser.parse_module().expect("should parse module");
        assert_eq!(module.items.len(), 1);
        match &module.items[0] {
            Item::Function(func) => {
                assert_eq!(func.params.len(), 2);
            }
            _ => panic!("expected function"),
        }
    }

    #[test]
    fn parse_function_with_let_and_return() {
        let source = r#"
            fn measure() -> i32 {
                let total = 1 + 2 * 3;
                return total;
            }
        "#;
        let mut parser = Parser::from_source(source);
        let module = parser.parse_module().expect("should parse module");
        let function = match &module.items[0] {
            Item::Function(f) => f,
            _ => panic!("expected function"),
        };
        assert_eq!(function.body.statements.len(), 2);
        match &function.body.statements[0] {
            Statement::Let(stmt) => {
                assert_eq!(stmt.name.name, "total");
                match &stmt.initializer {
                    Expression::Binary(bin) => {
                        assert_eq!(bin.op, BinaryOp::Add);
                    }
                    _ => panic!("expected binary initializer"),
                }
            }
            _ => panic!("expected let statement"),
        }
        match &function.body.statements[1] {
            Statement::Return(Some(expr), _) => match expr {
                Expression::Identifier(id) => assert_eq!(id.name, "total"),
                _ => panic!("expected identifier return"),
            },
            _ => panic!("expected return statement"),
        }
    }

    #[test]
    fn parse_call_expression_with_binary_argument() {
        let source = r#"
            fn emit_log() {
                log(sum(1, 2 + 3));
            }
        "#;
        let mut parser = Parser::from_source(source);
        let module = parser.parse_module().expect("should parse module");
        let function = match &module.items[0] {
            Item::Function(f) => f,
            _ => panic!("expected function"),
        };
        match &function.body.statements[0] {
            Statement::Expr(Expression::Call(call)) => {
                match &*call.callee {
                    Expression::Identifier(id) => assert_eq!(id.name, "log"),
                    _ => panic!("expected identifier callee"),
                }
                assert_eq!(call.args.len(), 1);
                match &call.args[0] {
                    Expression::Call(inner) => match &inner.args[1] {
                        Expression::Binary(bin) => assert_eq!(bin.op, BinaryOp::Add),
                        _ => panic!("expected binary second argument"),
                    },
                    _ => panic!("expected nested call"),
                }
            }
            _ => panic!("expected expression statement"),
        }
    }

    #[test]
    fn parse_minimal_fixture() {
        let source = r#"let x = 1;
fn main() {
    return x;
}"#;
        let mut parser = Parser::from_source(source);
        let module = parser.parse_module().expect("should parse module");
        assert_eq!(module.items.len(), 2);
    }

    #[test]
    fn test_range_parsing() {
        let source = "fn test() { let x = 0..10; }";
        let mut parser = Parser::from_source(source);
        let module = parser.parse_module();
        assert!(module.is_ok(), "Failed: {:?}", module.err());
    }
}
