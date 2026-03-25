pub mod ast;
pub mod lexer;
pub mod parser;
pub mod token;

pub use ast::{BinOp, Block, Expr, FuncName, FunctionCall, Stmt, TableField, UnOp};
pub use lexer::LexError;
pub use parser::Parser;
pub use token::{Token, TokenKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub message: String,
    pub line: u32,
    pub column: u32,
}

impl From<LexError> for ParseError {
    fn from(e: LexError) -> Self {
        Self {
            message: e.message,
            line: e.line,
            column: e.column,
        }
    }
}

/// Parse Lua source into an AST block.
pub fn parse(source: &str) -> Result<Block, ParseError> {
    let mut parser = Parser::new(source)?;
    parser.parse_block()
}
