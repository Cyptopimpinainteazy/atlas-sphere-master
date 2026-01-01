// crates/x3-compiler/src/ast.rs
// Abstract Syntax Tree definitions

#[derive(Debug, Clone)]
pub struct Program {
    pub modules: Vec<Module>,
}

#[derive(Debug, Clone)]
pub struct Module {
    pub name: String,
    pub imports: Vec<Import>,
    pub items: Vec<Item>,
}

#[derive(Debug, Clone)]
pub struct Import {
    pub path: String,
    pub items: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum Item {
    Function(Function),
    Struct(StructDef),
    Enum(EnumDef),
    Event(EventDef),
    Error(ErrorDef),
    Strategy(Strategy),
}

#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub is_pub: bool,
    pub is_extern: bool,
    pub params: Vec<(String, Type)>,
    pub return_type: Option<Type>,
    pub body: Vec<Statement>,
    pub attributes: Vec<Attribute>,
}

#[derive(Debug, Clone)]
pub struct Strategy {
    pub name: String,
    pub fields: Vec<(String, Type, Expr)>,
    pub methods: Vec<Function>,
}

#[derive(Debug, Clone)]
pub struct StructDef {
    pub name: String,
    pub fields: Vec<(String, Type)>,
}

#[derive(Debug, Clone)]
pub struct EnumDef {
    pub name: String,
    pub variants: Vec<(String, Vec<Type>)>,
}

#[derive(Debug, Clone)]
pub struct EventDef {
    pub name: String,
    pub fields: Vec<(String, Type)>,
}

#[derive(Debug, Clone)]
pub struct ErrorDef {
    pub name: String,
    pub message: String,
}

#[derive(Debug, Clone)]
pub enum Type {
    U8,
    U16,
    U32,
    U64,
    U128,
    Bool,
    String,
    Bytes,
    Bytes20,
    Bytes32,
    Address,
    Option(Box<Type>),
    Array(Box<Type>),
    Struct(String),
    Enum(String),
}

#[derive(Debug, Clone)]
pub enum Statement {
    Let(String, Option<Type>, Expr),
    LetMut(String, Option<Type>, Expr),
    Assign(String, Expr),
    If(Expr, Vec<Statement>, Option<Vec<Statement>>),
    While(Expr, Vec<Statement>),
    For(String, Expr, Vec<Statement>),
    Return(Option<Expr>),
    Expr(Expr),
}

#[derive(Debug, Clone)]
pub enum Expr {
    Literal(Literal),
    Ident(String),
    Binary(BinOp, Box<Expr>, Box<Expr>),
    Unary(UnOp, Box<Expr>),
    Call(String, Vec<Expr>),
    MethodCall(Box<Expr>, String, Vec<Expr>),
    FieldAccess(Box<Expr>, String),
    Index(Box<Expr>, Box<Expr>),
    Array(Vec<Expr>),
    Struct(String, Vec<(String, Expr)>),
    Match(Box<Expr>, Vec<(Pattern, Expr)>),
    If(Box<Expr>, Box<Expr>, Option<Box<Expr>>),
    Block(Vec<Statement>),
    Cast(Box<Expr>, Type),
}

#[derive(Debug, Clone)]
pub enum Literal {
    Number(u128),
    String(String),
    Bool(bool),
    None,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    And,
    Or,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnOp {
    Not,
    Neg,
    BitNot,
}

#[derive(Debug, Clone)]
pub enum Pattern {
    Literal(Literal),
    Ident(String),
    Enum(String, String, Vec<Pattern>),
}

#[derive(Debug, Clone)]
pub struct Attribute {
    pub name: String,
    pub args: Vec<String>,
}
