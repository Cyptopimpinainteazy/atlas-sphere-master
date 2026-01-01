use serde::{Deserialize, Serialize};
use x3_common::{Literal, Span};

/// The root node of a parsed module.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Module {
    pub items: Vec<Item>,
    pub span: Span,
}

/// Top-level declarations.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Item {
    Function(Function),
    GlobalLet(GlobalLet),
    Const(Const),
    Agent(Agent),
}

/// A free-standing binding, potentially mutable.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GlobalLet {
    pub name: Identifier,
    pub mutable: bool,
    pub ty: Option<TypeAnnotation>,
    pub initializer: Expression,
    pub span: Span,
}

/// A constant declaration.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Const {
    pub name: Identifier,
    pub ty: TypeAnnotation,
    pub value: Box<Expression>,
    pub span: Span,
}

/// An agent definition.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Agent {
    pub name: Identifier,
    pub items: Vec<Item>,
    pub span: Span,
}

/// A function definition.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Function {
    pub name: Identifier,
    pub params: Vec<Param>,
    pub ret_ty: Option<TypeAnnotation>,
    pub body: Block,
    pub span: Span,
}

/// Parameter metadata.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Param {
    pub name: Identifier,
    pub mutable: bool,
    pub ty: Option<TypeAnnotation>,
    pub span: Span,
}

/// Type annotation for bindings and signatures.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TypeAnnotation {
    pub name: Identifier,
    pub span: Span,
}

/// A block of statements.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Block {
    pub statements: Vec<Statement>,
    pub span: Span,
}

/// Statements that can appear inside blocks.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Statement {
    Let(LetStatement),
    Expr(Expression),
    Return(Option<Expression>, Span),
    If(IfStatement),
    While(WhileStatement),
    Loop(LoopStatement),
    For(ForStatement),
    Atomic(AtomicBlock),
    Emit(EmitStatement),
    Break(BreakStatement),
    Continue(ContinueStatement),
}

impl Statement {
    pub fn span(&self) -> Span {
        match self {
            Statement::Let(stmt) => stmt.span,
            Statement::Expr(expr) => expr.span(),
            Statement::Return(_, span) => *span,
            Statement::If(stmt) => stmt.span,
            Statement::While(stmt) => stmt.span,
            Statement::Loop(stmt) => stmt.span,
            Statement::For(stmt) => stmt.span,
            Statement::Atomic(stmt) => stmt.span,
            Statement::Emit(stmt) => stmt.span,
            Statement::Break(stmt) => stmt.span,
            Statement::Continue(stmt) => stmt.span,
        }
    }
}

/// Let binding inside a block.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LetStatement {
    pub name: Identifier,
    pub mutable: bool,
    pub ty: Option<TypeAnnotation>,
    pub initializer: Expression,
    pub span: Span,
}

/// If-statement structure.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IfStatement {
    pub condition: Expression,
    pub then_block: Block,
    pub else_block: Option<Block>,
    pub span: Span,
}

/// While-loop structure.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WhileStatement {
    pub condition: Expression,
    pub body: Block,
    pub span: Span,
}

/// Infinite loop structure (`loop { ... }`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LoopStatement {
    pub body: Block,
    pub span: Span,
}

/// For-loop structure (`for (init; cond; update) { ... }` or `for (let var in range) { ... }`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ForStatement {
    pub kind: ForLoopKind,
    pub body: Block,
    pub span: Span,
}

/// Kind of for loop.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ForLoopKind {
    /// C-style for loop: `for (init; cond; update) { ... }`
    CStyle {
        /// Optional initializer (let statement or expression).
        init: Option<Box<Statement>>,
        /// Optional loop condition.
        condition: Option<Expression>,
        /// Optional update expression.
        update: Option<Expression>,
    },
    /// Range-based for loop: `for (let var in range) { ... }`
    Range {
        /// Loop variable name.
        variable: Identifier,
        /// Range expression to iterate over.
        range: RangeExpression,
    },
}

/// Atomic block with optional metadata expression.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AtomicBlock {
    /// Optional metadata guard expression inside `atomic(expr)`.
    pub metadata: Option<Expression>,
    pub body: Block,
    pub span: Span,
}

/// Emit statement (`emit expr;`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EmitStatement {
    pub value: Expression,
    pub span: Span,
}

/// Break statement for exiting loops.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BreakStatement {
    pub span: Span,
}

/// Continue statement for skipping to next iteration.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContinueStatement {
    pub span: Span,
}

/// Expressions.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Expression {
    Literal(LiteralExpression),
    Identifier(Identifier),
    Binary(BinaryExpression),
    Unary(UnaryExpression),
    Call(CallExpression),
    Assign(AssignExpression),
    FieldAccess(FieldAccessExpression),
    Range(RangeExpression),
}

impl Expression {
    pub fn span(&self) -> Span {
        match self {
            Expression::Literal(lit) => lit.span,
            Expression::Identifier(id) => id.span,
            Expression::Binary(expr) => expr.span,
            Expression::Unary(expr) => expr.span,
            Expression::Call(expr) => expr.span,
            Expression::Assign(expr) => expr.span,
            Expression::FieldAccess(expr) => expr.span,
            Expression::Range(expr) => expr.span,
        }
    }
}

/// Literal expression with span.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LiteralExpression {
    pub literal: Literal,
    pub span: Span,
}

/// Binary expression.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BinaryExpression {
    pub op: BinaryOp,
    pub left: Box<Expression>,
    pub right: Box<Expression>,
    pub span: Span,
}

/// Unary expression.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UnaryExpression {
    pub op: UnaryOp,
    pub expr: Box<Expression>,
    pub span: Span,
}

/// Function call.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CallExpression {
    pub callee: Box<Expression>,
    pub args: Vec<Expression>,
    pub span: Span,
}

/// Assignment expression.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AssignExpression {
    pub target: Identifier,
    pub value: Box<Expression>,
    pub span: Span,
}

/// Field access expression.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FieldAccessExpression {
    pub object: Box<Expression>,
    pub field: Identifier,
    pub span: Span,
}

/// Range expression (start..end).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RangeExpression {
    pub start: Box<Expression>,
    pub end: Box<Expression>,
    pub span: Span,
}

/// Named identifier with a span.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Identifier {
    pub name: String,
    pub span: Span,
}

/// Supported binary operators.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    LogicalAnd,
    LogicalOr,
}

/// Supported unary operators.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnaryOp {
    Negate,
    Not,
}
