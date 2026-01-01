pub mod cursor;
pub mod error;
pub mod grammar;
pub mod parser;
pub mod tokens;
pub mod validator;

pub use error::{ParseError, ParseResult};
pub use parser::Parser;
pub use tokens::TokenStream;
pub use validator::StructuralValidator;

use x3_ast::Module;

/// Convenient entrypoint: parse source into a module.
pub fn parse_program(source: &str) -> ParseResult<Module> {
    let mut parser = Parser::from_source(source);
    let module = parser.parse_module()?;
    StructuralValidator::validate(&module)?;
    Ok(module)
}
