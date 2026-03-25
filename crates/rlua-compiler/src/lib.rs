mod compiler;

pub use compiler::CompileError;

use rlua_core::FunctionProto;

pub fn compile_source(source: &str) -> Result<FunctionProto, CompileError> {
    let block = rlua_parser::parse(source).map_err(CompileError::Parse)?;
    let mut c = compiler::Compiler::new(source);
    c.compile_main(&block)
}
