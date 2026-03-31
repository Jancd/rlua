use std::cell::RefCell;
use std::rc::Rc;

use crate::bytecode::Instruction;
use crate::value::{LuaValue, ThreadRef};

/// Description of a local variable (debug info).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalVar {
    pub name: String,
    pub start_pc: u32,
    pub end_pc: u32,
}

/// Description of how an upvalue is captured.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UpvalueDesc {
    /// If true, the upvalue captures a local in the immediately enclosing function's stack.
    /// If false, it captures an upvalue of the enclosing function.
    pub in_stack: bool,
    /// Index into the enclosing function's locals (if in_stack) or upvalues (if not).
    pub index: u8,
}

/// A compiled function prototype (analogous to Lua's `Proto` struct).
#[derive(Debug, Clone)]
pub struct FunctionProto {
    pub source_name: String,
    pub line_defined: u32,
    pub last_line_defined: u32,
    pub num_upvalues: u8,
    pub num_params: u8,
    pub is_vararg: bool,
    pub max_stack_size: u8,
    pub code: Vec<Instruction>,
    pub constants: Vec<LuaValue>,
    pub prototypes: Vec<FunctionProto>,
    pub upvalue_descs: Vec<UpvalueDesc>,
    pub line_info: Vec<u32>,
    pub local_vars: Vec<LocalVar>,
    pub upvalue_names: Vec<String>,
}

impl FunctionProto {
    pub fn new() -> Self {
        Self {
            source_name: String::new(),
            line_defined: 0,
            last_line_defined: 0,
            num_upvalues: 0,
            num_params: 0,
            is_vararg: true, // main chunk is always vararg
            max_stack_size: 2,
            code: Vec::new(),
            constants: Vec::new(),
            prototypes: Vec::new(),
            upvalue_descs: Vec::new(),
            line_info: Vec::new(),
            local_vars: Vec::new(),
            upvalue_names: Vec::new(),
        }
    }
}

impl Default for FunctionProto {
    fn default() -> Self {
        Self::new()
    }
}

/// A shared reference to a captured upvalue.
pub type UpvalRef = Rc<RefCell<LuaValue>>;

#[derive(Debug, Clone, PartialEq)]
pub enum CallOutcome {
    Return(Vec<LuaValue>),
    Yield(Vec<LuaValue>),
}

pub trait NativeVmContext {
    fn call_function(
        &mut self,
        func: &LuaValue,
        args: &[LuaValue],
    ) -> Result<Vec<LuaValue>, String>;
    fn source_location(&self) -> String;
    fn create_coroutine(&mut self, func: &LuaValue) -> Result<LuaValue, String>;
    fn resume_coroutine(
        &mut self,
        thread: &LuaValue,
        args: &[LuaValue],
    ) -> Result<Vec<LuaValue>, String>;
    fn running_coroutine(&self) -> Option<LuaValue>;
    fn coroutine_status(&self, thread: &LuaValue) -> Result<&'static str, String>;
    fn yield_current(&mut self, args: &[LuaValue]) -> Result<CallOutcome, String>;
}

/// A runtime closure: a function prototype plus captured upvalues.
#[derive(Debug, Clone)]
pub struct Closure {
    pub proto: Rc<FunctionProto>,
    pub upvalues: Vec<UpvalRef>,
}

impl Closure {
    pub fn new(proto: Rc<FunctionProto>) -> Self {
        Self {
            upvalues: Vec::with_capacity(proto.num_upvalues as usize),
            proto,
        }
    }
}

/// Native function signature: takes a VM-aware context, returns results or error.
pub type NativeFn =
    fn(ctx: &mut dyn NativeVmContext, args: &[LuaValue]) -> Result<CallOutcome, String>;

/// A Lua function can be either a Lua closure or a native Rust function.
#[derive(Clone)]
pub enum LuaFunction {
    Lua(Rc<Closure>),
    Native { name: &'static str, func: NativeFn },
    WrappedCoroutine { thread: ThreadRef },
}

impl std::fmt::Debug for LuaFunction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Lua(c) => write!(f, "function: {:p}", Rc::as_ptr(&c.proto)),
            Self::Native { name, .. } => write!(f, "function: {name}"),
            Self::WrappedCoroutine { thread } => {
                write!(f, "function: coroutine.wrap({:p})", Rc::as_ptr(thread))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn function_proto_default() {
        let proto = FunctionProto::new();
        assert!(proto.is_vararg);
        assert_eq!(proto.num_params, 0);
        assert!(proto.code.is_empty());
        assert!(proto.constants.is_empty());
    }

    #[test]
    fn closure_creation() {
        let proto = Rc::new(FunctionProto::new());
        let closure = Closure::new(proto.clone());
        assert!(closure.upvalues.is_empty());
        assert!(Rc::ptr_eq(&closure.proto, &proto));
    }
}
