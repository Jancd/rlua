mod compiler;

pub use compiler::CompileError;

use rlua_core::FunctionProto;

pub fn compile_source(source: &str) -> Result<FunctionProto, CompileError> {
    compile_named(source, "")
}

pub fn compile_named(source: &str, name: &str) -> Result<FunctionProto, CompileError> {
    let block = rlua_parser::parse(source).map_err(CompileError::Parse)?;
    let mut c = compiler::Compiler::new(name);
    c.compile_main(&block)
}
