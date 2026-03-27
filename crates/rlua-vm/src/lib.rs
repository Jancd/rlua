use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use rlua_core::bytecode::RK_OFFSET;
use rlua_core::function::{Closure, FunctionProto, LuaFunction, NativeFn, UpvalRef};
use rlua_core::gc::{GcRoot, GcRootProvider, MarkSweepGc, RootSource};
use rlua_core::opcode::Opcode;
use rlua_core::table::{LuaTable, TableRef};
use rlua_core::value::LuaValue;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum LuaError {
    Runtime(String),
    Type(String),
    Arithmetic(String),
}

impl std::fmt::Display for LuaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Runtime(msg) => write!(f, "{msg}"),
            Self::Type(msg) => write!(f, "{msg}"),
            Self::Arithmetic(msg) => write!(f, "{msg}"),
        }
    }
}

// ---------------------------------------------------------------------------
// Call frame
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct CallFrame {
    closure: Rc<Closure>,
    pc: usize,
    base: usize,
    #[allow(dead_code)]
    num_results: i32,
    varargs: Vec<LuaValue>,
}

// ---------------------------------------------------------------------------
// VM State
// ---------------------------------------------------------------------------

pub struct VmState {
    stack: Vec<LuaValue>,
    frames: Vec<CallFrame>,
    globals: Rc<RefCell<LuaTable>>,
    output: Vec<String>,
    open_upvalues: HashMap<usize, UpvalRef>,
    /// Shared metatable for all string values.
    string_metatable: Option<TableRef>,
    /// GC instance for allocation tracking.
    gc: MarkSweepGc,
}

impl VmState {
    pub fn new() -> Self {
        Self {
            stack: Vec::with_capacity(256),
            frames: Vec::new(),
            globals: Rc::new(RefCell::new(LuaTable::new())),
            output: Vec::new(),
            open_upvalues: HashMap::new(),
            string_metatable: None,
            gc: MarkSweepGc::new(),
        }
    }

    pub fn globals(&self) -> &Rc<RefCell<LuaTable>> {
        &self.globals
    }

    pub fn register_global(&self, name: &str, func: NativeFn) {
        let val = LuaValue::Function(Rc::new(LuaFunction::Native {
            name: Box::leak(name.to_owned().into_boxed_str()),
            func,
        }));
        self.globals.borrow_mut().rawset(LuaValue::from(name), val);
    }

    pub fn set_string_metatable(&mut self, mt: TableRef) {
        self.string_metatable = Some(mt);
    }

    pub fn get_output(&self) -> &[String] {
        &self.output
    }

    pub fn clear_output(&mut self) {
        self.output.clear();
    }

    fn ensure_stack(&mut self, size: usize) {
        if self.stack.len() < size {
            self.stack.resize(size, LuaValue::Nil);
        }
    }

    fn get_reg(&self, base: usize, idx: u8) -> LuaValue {
        let i = base + idx as usize;
        if let Some(uv) = self.open_upvalues.get(&i) {
            return uv.borrow().clone();
        }
        if i < self.stack.len() {
            self.stack[i].clone()
        } else {
            LuaValue::Nil
        }
    }

    fn set_reg(&mut self, base: usize, idx: u8, val: LuaValue) {
        let i = base + idx as usize;
        if let Some(uv) = self.open_upvalues.get(&i) {
            *uv.borrow_mut() = val.clone();
        }
        self.ensure_stack(i + 1);
        self.stack[i] = val;
    }

    fn rk(&self, base: usize, proto: &FunctionProto, val: u16) -> LuaValue {
        if val >= RK_OFFSET {
            let k_idx = (val - RK_OFFSET) as usize;
            proto.constants[k_idx].clone()
        } else {
            self.get_reg(base, val as u8)
        }
    }

    fn get_or_create_open_upvalue(&mut self, abs_idx: usize) -> UpvalRef {
        if let Some(uv) = self.open_upvalues.get(&abs_idx) {
            return uv.clone();
        }
        let val = if abs_idx < self.stack.len() {
            self.stack[abs_idx].clone()
        } else {
            LuaValue::Nil
        };
        let uv = Rc::new(RefCell::new(val));
        self.open_upvalues.insert(abs_idx, uv.clone());
        uv
    }

    fn close_upvalues(&mut self, from_abs: usize) {
        self.open_upvalues.retain(|&idx, _| idx < from_abs);
    }

    /// Get metamethod for a value (table metatable, or string metatable).
    fn get_metamethod(&self, val: &LuaValue, name: &str) -> Option<LuaValue> {
        match val {
            LuaValue::Table(t) => t.borrow().get_metamethod(name),
            LuaValue::String(_) => {
                let mt = self.string_metatable.as_ref()?;
                let mm = mt.borrow().rawget(&LuaValue::from(name));
                if matches!(mm, LuaValue::Nil) {
                    None
                } else {
                    Some(mm)
                }
            }
            _ => None,
        }
    }

    /// Get the current source location (source:line) from the top call frame.
    fn source_location(&self) -> String {
        if let Some(frame) = self.frames.last() {
            let proto = &frame.closure.proto;
            let pc = if frame.pc > 0 { frame.pc - 1 } else { 0 };
            let line = proto.line_info.get(pc).copied().unwrap_or(0);
            let source = if proto.source_name.is_empty() {
                "?"
            } else {
                &proto.source_name
            };
            if line > 0 {
                format!("{source}:{line}")
            } else {
                source.to_string()
            }
        } else {
            "?".to_string()
        }
    }

    /// Notify GC of an allocation and run collection if threshold exceeded.
    fn notify_alloc(&mut self) {
        if self.gc.notify_alloc() {
            self.run_gc();
        }
    }

    fn run_gc(&mut self) {
        // Collect roots first, then run GC (avoids borrow conflict on self)
        let mut roots = Vec::new();
        self.gc_roots(&mut roots);
        struct RootList(Vec<GcRoot>);
        impl GcRootProvider for RootList {
            fn gc_roots(&self, out: &mut Vec<GcRoot>) {
                out.extend(self.0.iter().cloned());
            }
        }
        let provider = RootList(roots);
        self.gc.collect(&[&provider]);
    }
}

impl Default for VmState {
    fn default() -> Self {
        Self::new()
    }
}

impl GcRootProvider for VmState {
    fn gc_roots(&self, roots: &mut Vec<GcRoot>) {
        for val in &self.stack {
            if !matches!(val, LuaValue::Nil) {
                roots.push(GcRoot {
                    source: RootSource::Stack,
                    value: val.clone(),
                });
            }
        }
        roots.push(GcRoot {
            source: RootSource::Globals,
            value: LuaValue::Table(self.globals.clone()),
        });
        for uv in self.open_upvalues.values() {
            roots.push(GcRoot {
                source: RootSource::OpenUpvalues,
                value: uv.borrow().clone(),
            });
        }
    }
}

// ---------------------------------------------------------------------------
// Metamethod helpers
// ---------------------------------------------------------------------------

const METAMETHOD_DEPTH_LIMIT: usize = 200;

/// Look up a binary metamethod: try left operand first, then right.
fn get_binary_metamethod(
    state: &VmState,
    a: &LuaValue,
    b: &LuaValue,
    name: &str,
) -> Option<LuaValue> {
    state
        .get_metamethod(a, name)
        .or_else(|| state.get_metamethod(b, name))
}

/// Call a metamethod (native only for now) with given args.
fn call_metamethod_native(
    state: &mut VmState,
    mm: &LuaValue,
    args: &[LuaValue],
) -> Result<Vec<LuaValue>, LuaError> {
    call_function(state, mm, args)
}

/// Table __index resolution with depth limit.
fn index_with_metamethod(
    state: &mut VmState,
    table_val: &LuaValue,
    key: &LuaValue,
    depth: usize,
) -> Result<LuaValue, LuaError> {
    if depth > METAMETHOD_DEPTH_LIMIT {
        return Err(LuaError::Runtime(
            "'__index' chain too long; possible loop".to_owned(),
        ));
    }

    match table_val {
        LuaValue::Table(t) => {
            let raw = t.borrow().rawget(key);
            if !matches!(raw, LuaValue::Nil) {
                return Ok(raw);
            }
            // Check __index metamethod
            let mm = t.borrow().get_metamethod("__index");
            match mm {
                Some(mm_val) => match &mm_val {
                    LuaValue::Function(_) => {
                        let results = call_metamethod_native(
                            state,
                            &mm_val,
                            &[table_val.clone(), key.clone()],
                        )?;
                        Ok(results.into_iter().next().unwrap_or(LuaValue::Nil))
                    }
                    LuaValue::Table(_) => index_with_metamethod(state, &mm_val, key, depth + 1),
                    _ => Ok(LuaValue::Nil),
                },
                None => Ok(LuaValue::Nil),
            }
        }
        LuaValue::String(_) => {
            // String indexing: use string metatable
            if let Some(mm) = state.get_metamethod(table_val, "__index") {
                match mm {
                    LuaValue::Table(_) => index_with_metamethod(state, &mm, key, depth + 1),
                    LuaValue::Function(_) => {
                        let results =
                            call_metamethod_native(state, &mm, &[table_val.clone(), key.clone()])?;
                        Ok(results.into_iter().next().unwrap_or(LuaValue::Nil))
                    }
                    _ => Ok(LuaValue::Nil),
                }
            } else {
                Ok(LuaValue::Nil)
            }
        }
        _ => {
            // Check for metamethod on the value
            if let Some(mm) = state.get_metamethod(table_val, "__index") {
                match mm {
                    LuaValue::Table(_) => index_with_metamethod(state, &mm, key, depth + 1),
                    LuaValue::Function(_) => {
                        let results =
                            call_metamethod_native(state, &mm, &[table_val.clone(), key.clone()])?;
                        Ok(results.into_iter().next().unwrap_or(LuaValue::Nil))
                    }
                    _ => Err(LuaError::Type(format!(
                        "attempt to index a {} value",
                        table_val.type_name()
                    ))),
                }
            } else {
                Err(LuaError::Type(format!(
                    "attempt to index a {} value",
                    table_val.type_name()
                )))
            }
        }
    }
}

/// Table __newindex resolution with depth limit.
fn newindex_with_metamethod(
    state: &mut VmState,
    table_val: &LuaValue,
    key: &LuaValue,
    val: &LuaValue,
    depth: usize,
) -> Result<(), LuaError> {
    if depth > METAMETHOD_DEPTH_LIMIT {
        return Err(LuaError::Runtime(
            "'__newindex' chain too long; possible loop".to_owned(),
        ));
    }
    match table_val {
        LuaValue::Table(t) => {
            // If key already exists, raw set directly
            let existing = t.borrow().rawget(key);
            if !matches!(existing, LuaValue::Nil) {
                t.borrow_mut().rawset(key.clone(), val.clone());
                return Ok(());
            }
            // Check __newindex metamethod
            let mm = t.borrow().get_metamethod("__newindex");
            match mm {
                Some(mm_val) => match &mm_val {
                    LuaValue::Function(_) => {
                        call_metamethod_native(
                            state,
                            &mm_val,
                            &[table_val.clone(), key.clone(), val.clone()],
                        )?;
                        Ok(())
                    }
                    LuaValue::Table(_) => {
                        newindex_with_metamethod(state, &mm_val, key, val, depth + 1)
                    }
                    _ => {
                        t.borrow_mut().rawset(key.clone(), val.clone());
                        Ok(())
                    }
                },
                None => {
                    t.borrow_mut().rawset(key.clone(), val.clone());
                    Ok(())
                }
            }
        }
        _ => Err(LuaError::Type(format!(
            "attempt to index a {} value",
            table_val.type_name()
        ))),
    }
}

// ---------------------------------------------------------------------------
// Execution
// ---------------------------------------------------------------------------

pub fn execute(state: &mut VmState, proto: FunctionProto) -> Result<Vec<LuaValue>, LuaError> {
    let proto_rc = Rc::new(proto);
    let closure = Rc::new(Closure::new(proto_rc));
    execute_closure(state, closure, &[])
}

pub fn execute_closure(
    state: &mut VmState,
    closure: Rc<Closure>,
    args: &[LuaValue],
) -> Result<Vec<LuaValue>, LuaError> {
    let base = state.stack.len();
    let proto = &closure.proto;

    let max_stack = proto.max_stack_size as usize;
    state.ensure_stack(base + max_stack.max(proto.num_params as usize + 1));

    for i in 0..proto.num_params as usize {
        let val = args.get(i).cloned().unwrap_or(LuaValue::Nil);
        state.set_reg(base, i as u8, val);
    }

    let varargs = if proto.is_vararg {
        args.get(proto.num_params as usize..)
            .unwrap_or(&[])
            .to_vec()
    } else {
        Vec::new()
    };

    state.frames.push(CallFrame {
        closure: closure.clone(),
        pc: 0,
        base,
        num_results: -1,
        varargs,
    });

    let result = run_loop(state);

    state.close_upvalues(base);
    state.stack.truncate(base);
    state.frames.pop();

    result
}

fn run_loop(state: &mut VmState) -> Result<Vec<LuaValue>, LuaError> {
    loop {
        let frame = state.frames.last().unwrap();
        let base = frame.base;
        let pc = frame.pc;
        let proto = frame.closure.proto.clone();

        if pc >= proto.code.len() {
            return Ok(Vec::new());
        }

        let instr = proto.code[pc];
        let op = instr.opcode();
        let a = instr.a();

        #[cfg(feature = "trace-exec")]
        eprintln!(
            "[trace-exec] pc={pc} op={:?} a={a} b={} c={}",
            op,
            instr.b(),
            instr.c()
        );

        // Advance PC
        state.frames.last_mut().unwrap().pc = pc + 1;

        match op {
            Opcode::Move => {
                let b = instr.b();
                let val = state.get_reg(base, b as u8).clone();
                state.set_reg(base, a, val);
            }

            Opcode::LoadK => {
                let bx = instr.bx() as usize;
                let val = proto.constants[bx].clone();
                state.set_reg(base, a, val);
            }

            Opcode::LoadBool => {
                let b = instr.b();
                let c = instr.c();
                state.set_reg(base, a, LuaValue::Boolean(b != 0));
                if c != 0 {
                    state.frames.last_mut().unwrap().pc += 1;
                }
            }

            Opcode::LoadNil => {
                let b = instr.b() as u8;
                for i in a..=a + b {
                    state.set_reg(base, i, LuaValue::Nil);
                }
            }

            Opcode::GetUpval => {
                let b = instr.b();
                let closure = state.frames.last().unwrap().closure.clone();
                let val = if (b as usize) < closure.upvalues.len() {
                    closure.upvalues[b as usize].borrow().clone()
                } else {
                    LuaValue::Nil
                };
                state.set_reg(base, a, val);
            }

            Opcode::GetGlobal => {
                let bx = instr.bx() as usize;
                let key = proto.constants[bx].clone();
                let val = state.globals.borrow().rawget(&key);
                state.set_reg(base, a, val);
            }

            Opcode::GetTable => {
                let b = instr.b();
                let c = instr.c();
                let table = state.get_reg(base, b as u8).clone();
                let key = state.rk(base, &proto, c);
                let val = index_with_metamethod(state, &table, &key, 0)?;
                state.set_reg(base, a, val);
            }

            Opcode::SetGlobal => {
                let bx = instr.bx() as usize;
                let key = proto.constants[bx].clone();
                let val = state.get_reg(base, a).clone();
                state.globals.borrow_mut().rawset(key, val);
            }

            Opcode::SetUpval => {
                let b = instr.b();
                let val = state.get_reg(base, a).clone();
                let closure = state.frames.last().unwrap().closure.clone();
                if (b as usize) < closure.upvalues.len() {
                    *closure.upvalues[b as usize].borrow_mut() = val;
                }
            }

            Opcode::SetTable => {
                let b = instr.b();
                let c = instr.c();
                let table = state.get_reg(base, a).clone();
                let key = state.rk(base, &proto, b);
                let val = state.rk(base, &proto, c);
                newindex_with_metamethod(state, &table, &key, &val, 0)?;
            }

            Opcode::NewTable => {
                let table = Rc::new(RefCell::new(LuaTable::new()));
                state.set_reg(base, a, LuaValue::Table(table));
                state.notify_alloc();
            }

            Opcode::OpSelf => {
                let b = instr.b();
                let c = instr.c();
                let table = state.get_reg(base, b as u8).clone();
                let key = state.rk(base, &proto, c);
                // R(A+1) = R(B); R(A) = R(B)[RK(C)]
                state.set_reg(base, a + 1, table.clone());
                let val = index_with_metamethod(state, &table, &key, 0)?;
                state.set_reg(base, a, val);
            }

            Opcode::Add | Opcode::Sub | Opcode::Mul | Opcode::Div | Opcode::Mod | Opcode::Pow => {
                let b = instr.b();
                let c = instr.c();
                let lhs = state.rk(base, &proto, b);
                let rhs = state.rk(base, &proto, c);

                // Try numeric operation first
                let ln = lhs.to_number();
                let rn = rhs.to_number();

                if let (Some(ln), Some(rn)) = (ln, rn) {
                    let result = match op {
                        Opcode::Add => ln + rn,
                        Opcode::Sub => ln - rn,
                        Opcode::Mul => ln * rn,
                        Opcode::Div => ln / rn,
                        Opcode::Mod => {
                            let r = ln % rn;
                            if r != 0.0 && (r < 0.0) != (rn < 0.0) {
                                r + rn
                            } else {
                                r
                            }
                        }
                        Opcode::Pow => ln.powf(rn),
                        _ => unreachable!(),
                    };
                    state.set_reg(base, a, LuaValue::Number(result));
                } else {
                    // Try metamethods
                    let mm_name = match op {
                        Opcode::Add => "__add",
                        Opcode::Sub => "__sub",
                        Opcode::Mul => "__mul",
                        Opcode::Div => "__div",
                        Opcode::Mod => "__mod",
                        Opcode::Pow => "__pow",
                        _ => unreachable!(),
                    };
                    if let Some(mm) = get_binary_metamethod(state, &lhs, &rhs, mm_name) {
                        let results = call_metamethod_native(state, &mm, &[lhs, rhs])?;
                        let val = results.into_iter().next().unwrap_or(LuaValue::Nil);
                        state.set_reg(base, a, val);
                    } else {
                        return Err(LuaError::Arithmetic(format!(
                            "attempt to perform arithmetic on a {} value",
                            if ln.is_none() {
                                lhs.type_name()
                            } else {
                                rhs.type_name()
                            }
                        )));
                    }
                }
            }

            Opcode::Unm => {
                let b = instr.b();
                let val = state.get_reg(base, b as u8).clone();
                if let Some(n) = val.to_number() {
                    state.set_reg(base, a, LuaValue::Number(-n));
                } else if let Some(mm) = state.get_metamethod(&val, "__unm") {
                    let results = call_metamethod_native(state, &mm, &[val])?;
                    let r = results.into_iter().next().unwrap_or(LuaValue::Nil);
                    state.set_reg(base, a, r);
                } else {
                    return Err(LuaError::Arithmetic(format!(
                        "attempt to perform arithmetic on a {} value",
                        val.type_name()
                    )));
                }
            }

            Opcode::Not => {
                let b = instr.b();
                let val = state.get_reg(base, b as u8).clone();
                state.set_reg(base, a, LuaValue::Boolean(!val.is_truthy()));
            }

            Opcode::Len => {
                let b = instr.b();
                let val = state.get_reg(base, b as u8).clone();
                // Check __len metamethod first for tables
                if let Some(mm) = state.get_metamethod(&val, "__len") {
                    let results = call_metamethod_native(state, &mm, &[val])?;
                    let r = results.into_iter().next().unwrap_or(LuaValue::Nil);
                    state.set_reg(base, a, r);
                } else {
                    match val {
                        LuaValue::String(s) => {
                            state.set_reg(base, a, LuaValue::Number(s.len() as f64));
                        }
                        LuaValue::Table(t) => {
                            state.set_reg(base, a, LuaValue::Number(t.borrow().len() as f64));
                        }
                        _ => {
                            return Err(LuaError::Type(format!(
                                "attempt to get length of a {} value",
                                val.type_name()
                            )));
                        }
                    }
                }
            }

            Opcode::Concat => {
                let b = instr.b() as u8;
                let c = instr.c() as u8;
                let mut result = String::new();
                let mut need_metamethod = false;
                for i in b..=c {
                    let val = state.get_reg(base, i).clone();
                    match &val {
                        LuaValue::String(s) => result.push_str(s),
                        LuaValue::Number(_) => result.push_str(&val.to_lua_string()),
                        _ => {
                            // Try __concat metamethod
                            if i > b {
                                let prev = LuaValue::from(result.clone());
                                if let Some(mm) =
                                    get_binary_metamethod(state, &prev, &val, "__concat")
                                {
                                    let mut combined = prev;
                                    // Concat remaining with metamethod
                                    for j in i..=c {
                                        let next = state.get_reg(base, j).clone();
                                        let results =
                                            call_metamethod_native(state, &mm, &[combined, next])?;
                                        combined =
                                            results.into_iter().next().unwrap_or(LuaValue::Nil);
                                    }
                                    state.set_reg(base, a, combined);
                                    need_metamethod = true;
                                    break;
                                }
                            }
                            if let Some(mm) = state.get_metamethod(&val, "__concat") {
                                let prev = if result.is_empty() {
                                    val.clone()
                                } else {
                                    LuaValue::from(result.clone())
                                };
                                let results = call_metamethod_native(state, &mm, &[prev, val])?;
                                let r = results.into_iter().next().unwrap_or(LuaValue::Nil);
                                state.set_reg(base, a, r);
                                need_metamethod = true;
                                break;
                            }
                            return Err(LuaError::Type(format!(
                                "attempt to concatenate a {} value",
                                val.type_name()
                            )));
                        }
                    }
                }
                if !need_metamethod {
                    state.notify_alloc(); // String allocation from concatenation
                    state.set_reg(base, a, LuaValue::from(result));
                }
            }

            Opcode::Jmp => {
                let sbx = instr.sbx();
                let new_pc = (state.frames.last().unwrap().pc as i32 + sbx) as usize;
                state.frames.last_mut().unwrap().pc = new_pc;
                // GC safepoint at backward jumps (loop back-edges)
                if sbx < 0 && state.gc.alloc_count() >= state.gc.threshold() {
                    state.run_gc();
                }
            }

            Opcode::Eq => {
                let b = instr.b();
                let c = instr.c();
                let lhs = state.rk(base, &proto, b);
                let rhs = state.rk(base, &proto, c);
                let equal = lua_equal_with_metamethod(state, &lhs, &rhs)?;
                let expected = a != 0;
                if equal == expected {
                    state.frames.last_mut().unwrap().pc += 1;
                }
            }

            Opcode::Lt => {
                let b = instr.b();
                let c = instr.c();
                let lhs = state.rk(base, &proto, b);
                let rhs = state.rk(base, &proto, c);
                let result = lua_less_than_with_metamethod(state, &lhs, &rhs)?;
                let expected = a != 0;
                if result == expected {
                    state.frames.last_mut().unwrap().pc += 1;
                }
            }

            Opcode::Le => {
                let b = instr.b();
                let c = instr.c();
                let lhs = state.rk(base, &proto, b);
                let rhs = state.rk(base, &proto, c);
                let result = lua_less_equal_with_metamethod(state, &lhs, &rhs)?;
                let expected = a != 0;
                if result == expected {
                    state.frames.last_mut().unwrap().pc += 1;
                }
            }

            Opcode::Test => {
                let c = instr.c();
                let val = state.get_reg(base, a).clone();
                if val.is_truthy() != (c != 0) {
                    state.frames.last_mut().unwrap().pc += 1;
                }
            }

            Opcode::TestSet => {
                let b = instr.b();
                let c = instr.c();
                let val = state.get_reg(base, b as u8).clone();
                if val.is_truthy() != (c != 0) {
                    state.frames.last_mut().unwrap().pc += 1;
                } else {
                    state.set_reg(base, a, val);
                }
            }

            Opcode::Call => {
                let b = instr.b();
                let c = instr.c();
                let func_val = state.get_reg(base, a).clone();

                // GC safepoint at function calls
                if state.gc.alloc_count() >= state.gc.threshold() {
                    state.run_gc();
                }

                let num_args = if b == 0 {
                    state.stack.len() - (base + a as usize + 1)
                } else {
                    (b - 1) as usize
                };

                let args: Vec<LuaValue> = (0..num_args)
                    .map(|i| state.get_reg(base, a + 1 + i as u8).clone())
                    .collect();

                let num_results_wanted = if c == 0 { -1i32 } else { (c - 1) as i32 };

                // Special handling for pcall, xpcall, and error (source location)
                let is_pcall = matches!(&func_val, LuaValue::Function(f) if matches!(f.as_ref(), LuaFunction::Native { name: "pcall", .. }));
                let is_xpcall = matches!(&func_val, LuaValue::Function(f) if matches!(f.as_ref(), LuaFunction::Native { name: "xpcall", .. }));
                let is_error = matches!(&func_val, LuaValue::Function(f) if matches!(f.as_ref(), LuaFunction::Native { name: "error", .. }));

                let results = if is_pcall {
                    if args.is_empty() {
                        return Err(LuaError::Runtime(
                            "bad argument #1 to 'pcall' (value expected)".to_owned(),
                        ));
                    }
                    let pcall_func = args[0].clone();
                    let pcall_args = if args.len() > 1 { &args[1..] } else { &[] };
                    match call_function(state, &pcall_func, pcall_args) {
                        Ok(mut res) => {
                            res.insert(0, LuaValue::Boolean(true));
                            res
                        }
                        Err(e) => {
                            let msg = lua_error_to_value(e);
                            vec![LuaValue::Boolean(false), msg]
                        }
                    }
                } else if is_xpcall {
                    if args.len() < 2 {
                        return Err(LuaError::Runtime(
                            "bad argument #1 to 'xpcall' (value expected)".to_owned(),
                        ));
                    }
                    let xpcall_func = args[0].clone();
                    let handler = args[1].clone();
                    let xpcall_args = if args.len() > 2 { &args[2..] } else { &[] };
                    match call_function(state, &xpcall_func, xpcall_args) {
                        Ok(mut res) => {
                            res.insert(0, LuaValue::Boolean(true));
                            res
                        }
                        Err(e) => {
                            let err_val = lua_error_to_value(e);
                            // Call handler with error
                            match call_function(state, &handler, std::slice::from_ref(&err_val)) {
                                Ok(mut res) => {
                                    res.insert(0, LuaValue::Boolean(false));
                                    res
                                }
                                Err(handler_err) => {
                                    let msg = lua_error_to_value(handler_err);
                                    vec![LuaValue::Boolean(false), msg]
                                }
                            }
                        }
                    }
                } else if is_error {
                    // Intercept error() to prepend source location to string messages
                    // Lua 5.1: error(msg, level) — if msg is a string and level != 0,
                    // the source location is prepended at error() call time
                    let msg = args.first().cloned().unwrap_or(LuaValue::Nil);
                    let level = args.get(1).and_then(|v| v.to_number()).unwrap_or(1.0) as i32;

                    let annotated_msg = if level > 0 {
                        if let LuaValue::String(ref s) = msg {
                            let loc = state.source_location();
                            LuaValue::String(std::rc::Rc::new(format!("{loc}: {s}")))
                        } else {
                            msg
                        }
                    } else {
                        msg
                    };

                    return Err(LuaError::Runtime(annotated_msg.to_lua_string()));
                } else {
                    call_function(state, &func_val, &args)?
                };

                let num_results = results.len();
                if num_results_wanted < 0 {
                    for (i, val) in results.into_iter().enumerate() {
                        state.set_reg(base, a + i as u8, val);
                    }
                    state.stack.truncate(base + a as usize + num_results);
                } else {
                    for i in 0..num_results_wanted as usize {
                        let val = results.get(i).cloned().unwrap_or(LuaValue::Nil);
                        state.set_reg(base, a + i as u8, val);
                    }
                }
            }

            Opcode::TailCall => {
                let b = instr.b();
                let func_val = state.get_reg(base, a).clone();

                // GC safepoint at tail calls
                if state.gc.alloc_count() >= state.gc.threshold() {
                    state.run_gc();
                }

                let num_args = if b == 0 {
                    state.stack.len() - (base + a as usize + 1)
                } else {
                    (b - 1) as usize
                };

                let args: Vec<LuaValue> = (0..num_args)
                    .map(|i| state.get_reg(base, a + 1 + i as u8).clone())
                    .collect();

                // Tail call optimization: if calling a Lua closure, reuse current frame
                match &func_val {
                    LuaValue::Function(f) => match f.as_ref() {
                        LuaFunction::Lua(closure) => {
                            let new_proto = &closure.proto;
                            state.close_upvalues(base);

                            // Set up params in current frame's registers
                            for i in 0..new_proto.num_params as usize {
                                let val = args.get(i).cloned().unwrap_or(LuaValue::Nil);
                                state.set_reg(base, i as u8, val);
                            }

                            let varargs = if new_proto.is_vararg {
                                args.get(new_proto.num_params as usize..)
                                    .unwrap_or(&[])
                                    .to_vec()
                            } else {
                                Vec::new()
                            };

                            // Reuse the current call frame
                            let frame = state.frames.last_mut().unwrap();
                            frame.closure = closure.clone();
                            frame.pc = 0;
                            frame.varargs = varargs;
                            // base stays the same — that's the optimization
                            continue;
                        }
                        LuaFunction::Native { name, func } => {
                            // Check for pcall/xpcall interception
                            if *name == "pcall" {
                                if args.is_empty() {
                                    return Err(LuaError::Runtime(
                                        "bad argument #1 to 'pcall' (value expected)".to_owned(),
                                    ));
                                }
                                let pcall_func = args[0].clone();
                                let pcall_args = if args.len() > 1 { &args[1..] } else { &[] };
                                let results = match call_function(state, &pcall_func, pcall_args) {
                                    Ok(mut res) => {
                                        res.insert(0, LuaValue::Boolean(true));
                                        res
                                    }
                                    Err(e) => {
                                        let msg = lua_error_to_value(e);
                                        vec![LuaValue::Boolean(false), msg]
                                    }
                                };
                                return Ok(results);
                            } else if *name == "xpcall" {
                                if args.len() < 2 {
                                    return Err(LuaError::Runtime(
                                        "bad argument #1 to 'xpcall' (value expected)".to_owned(),
                                    ));
                                }
                                let xpcall_func = args[0].clone();
                                let handler = args[1].clone();
                                let xpcall_args = if args.len() > 2 { &args[2..] } else { &[] };
                                let results = match call_function(state, &xpcall_func, xpcall_args)
                                {
                                    Ok(mut res) => {
                                        res.insert(0, LuaValue::Boolean(true));
                                        res
                                    }
                                    Err(e) => {
                                        let err_val = lua_error_to_value(e);
                                        match call_function(
                                            state,
                                            &handler,
                                            std::slice::from_ref(&err_val),
                                        ) {
                                            Ok(mut res) => {
                                                res.insert(0, LuaValue::Boolean(false));
                                                res
                                            }
                                            Err(handler_err) => {
                                                let msg = lua_error_to_value(handler_err);
                                                vec![LuaValue::Boolean(false), msg]
                                            }
                                        }
                                    }
                                };
                                return Ok(results);
                            } else if *name == "error" {
                                // Intercept error() to prepend source location
                                let msg = args.first().cloned().unwrap_or(LuaValue::Nil);
                                let level =
                                    args.get(1).and_then(|v| v.to_number()).unwrap_or(1.0) as i32;
                                let annotated_msg = if level > 0 {
                                    if let LuaValue::String(ref s) = msg {
                                        let loc = state.source_location();
                                        LuaValue::String(std::rc::Rc::new(format!("{loc}: {s}")))
                                    } else {
                                        msg
                                    }
                                } else {
                                    msg
                                };
                                return Err(LuaError::Runtime(annotated_msg.to_lua_string()));
                            }
                            let results = func(&args).map_err(LuaError::Runtime)?;
                            return Ok(results);
                        }
                    },
                    LuaValue::Table(t) => {
                        // Check __call metamethod
                        if let Some(mm) = t.borrow().get_metamethod("__call") {
                            let mut call_args = vec![func_val.clone()];
                            call_args.extend(args);
                            let results = call_function(state, &mm, &call_args)?;
                            return Ok(results);
                        }
                        return Err(LuaError::Type(format!(
                            "attempt to call a {} value",
                            func_val.type_name()
                        )));
                    }
                    _ => {
                        return Err(LuaError::Type(format!(
                            "attempt to call a {} value",
                            func_val.type_name()
                        )));
                    }
                }
            }

            Opcode::Return => {
                let b = instr.b();
                if b == 0 {
                    let top = state.stack.len();
                    let start = base + a as usize;
                    let results: Vec<LuaValue> = if start < top {
                        state.stack[start..top].to_vec()
                    } else {
                        Vec::new()
                    };
                    return Ok(results);
                } else if b == 1 {
                    return Ok(Vec::new());
                } else {
                    let count = (b - 1) as usize;
                    let results: Vec<LuaValue> = (0..count)
                        .map(|i| state.get_reg(base, a + i as u8).clone())
                        .collect();
                    return Ok(results);
                }
            }

            Opcode::ForLoop => {
                let index = to_number(&state.get_reg(base, a))?;
                let limit = to_number(&state.get_reg(base, a + 1))?;
                let step = to_number(&state.get_reg(base, a + 2))?;
                let new_index = index + step;

                let in_range = if step > 0.0 {
                    new_index <= limit
                } else {
                    new_index >= limit
                };

                if in_range {
                    state.set_reg(base, a, LuaValue::Number(new_index));
                    state.set_reg(base, a + 3, LuaValue::Number(new_index));
                    let sbx = instr.sbx();
                    let new_pc = (state.frames.last().unwrap().pc as i32 + sbx) as usize;
                    state.frames.last_mut().unwrap().pc = new_pc;
                }
            }

            Opcode::ForPrep => {
                let index = to_number(&state.get_reg(base, a))?;
                let step = to_number(&state.get_reg(base, a + 2))?;
                state.set_reg(base, a, LuaValue::Number(index - step));
                let sbx = instr.sbx();
                let new_pc = (state.frames.last().unwrap().pc as i32 + sbx) as usize;
                state.frames.last_mut().unwrap().pc = new_pc;
            }

            Opcode::TForLoop => {
                let c = instr.c() as usize;
                let func = state.get_reg(base, a).clone();
                let s = state.get_reg(base, a + 1).clone();
                let var = state.get_reg(base, a + 2).clone();

                let results = call_function(state, &func, &[s, var])?;

                for i in 0..c {
                    let val = results.get(i).cloned().unwrap_or(LuaValue::Nil);
                    state.set_reg(base, a + 3 + i as u8, val);
                }

                let first = state.get_reg(base, a + 3).clone();
                if first != LuaValue::Nil {
                    state.set_reg(base, a + 2, first);
                } else {
                    state.frames.last_mut().unwrap().pc += 1;
                }
            }

            Opcode::SetList => {
                let b = instr.b();
                let c = instr.c();
                let table = state.get_reg(base, a).clone();
                if let LuaValue::Table(t) = table {
                    let count = if b == 0 {
                        state.stack.len() - (base + a as usize + 1)
                    } else {
                        b as usize
                    };
                    let offset = (c as usize - 1) * 50;
                    for i in 1..=count {
                        let val = state.get_reg(base, a + i as u8).clone();
                        let key = LuaValue::Number((offset + i) as f64);
                        t.borrow_mut().rawset(key, val);
                    }
                }
            }

            Opcode::Close => {
                state.close_upvalues(base + a as usize);
            }

            Opcode::Closure => {
                let bx = instr.bx() as usize;
                let child_proto = Rc::new(proto.prototypes[bx].clone());
                let mut closure = Closure::new(child_proto.clone());

                for uv_desc in &child_proto.upvalue_descs {
                    let uv_ref = if uv_desc.in_stack {
                        let abs_idx = base + uv_desc.index as usize;
                        state.get_or_create_open_upvalue(abs_idx)
                    } else {
                        let parent_closure = state.frames.last().unwrap().closure.clone();
                        if (uv_desc.index as usize) < parent_closure.upvalues.len() {
                            parent_closure.upvalues[uv_desc.index as usize].clone()
                        } else {
                            Rc::new(RefCell::new(LuaValue::Nil))
                        }
                    };
                    closure.upvalues.push(uv_ref);
                }

                let func = LuaFunction::Lua(Rc::new(closure));
                state.set_reg(base, a, LuaValue::Function(Rc::new(func)));
                state.notify_alloc();
            }

            Opcode::Vararg => {
                let b = instr.b();
                let varargs = state.frames.last().unwrap().varargs.clone();
                let num_varargs = varargs.len();

                let count = if b == 0 {
                    num_varargs
                } else {
                    (b - 1) as usize
                };

                for (i, val) in varargs.iter().enumerate().take(count) {
                    state.set_reg(base, a + i as u8, val.clone());
                }
                for i in num_varargs..count {
                    state.set_reg(base, a + i as u8, LuaValue::Nil);
                }

                if b == 0 {
                    let new_top = base + a as usize + count;
                    state.stack.resize(new_top, LuaValue::Nil);
                }
            }

            Opcode::Nop => {}

            Opcode::Halt => {
                return Ok(Vec::new());
            }
        }
    }
}

fn call_function(
    state: &mut VmState,
    func_val: &LuaValue,
    args: &[LuaValue],
) -> Result<Vec<LuaValue>, LuaError> {
    match func_val {
        LuaValue::Function(f) => match f.as_ref() {
            LuaFunction::Native { func, .. } => func(args).map_err(LuaError::Runtime),
            LuaFunction::Lua(closure) => execute_closure(state, closure.clone(), args),
        },
        LuaValue::Table(t) => {
            // __call metamethod
            if let Some(mm) = t.borrow().get_metamethod("__call") {
                let mut call_args = vec![func_val.clone()];
                call_args.extend_from_slice(args);
                call_function(state, &mm, &call_args)
            } else {
                Err(LuaError::Type(format!(
                    "attempt to call a {} value",
                    func_val.type_name()
                )))
            }
        }
        _ => Err(LuaError::Type(format!(
            "attempt to call a {} value",
            func_val.type_name()
        ))),
    }
}

fn lua_error_to_value(e: LuaError) -> LuaValue {
    match e {
        LuaError::Runtime(s) | LuaError::Type(s) | LuaError::Arithmetic(s) => LuaValue::from(s),
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn to_number(val: &LuaValue) -> Result<f64, LuaError> {
    val.to_number().ok_or_else(|| {
        LuaError::Arithmetic(format!(
            "attempt to perform arithmetic on a {} value",
            val.type_name()
        ))
    })
}

fn lua_equal_with_metamethod(
    state: &mut VmState,
    a: &LuaValue,
    b: &LuaValue,
) -> Result<bool, LuaError> {
    // Raw equality first
    if a == b {
        return Ok(true);
    }
    // Only check __eq for two tables or two userdata of same type
    match (a, b) {
        (LuaValue::Table(ta), LuaValue::Table(tb)) => {
            if Rc::ptr_eq(ta, tb) {
                return Ok(true);
            }
            // Both must have the same __eq metamethod
            let mm_a = ta.borrow().get_metamethod("__eq");
            if let Some(mm) = mm_a {
                let results = call_metamethod_native(state, &mm, &[a.clone(), b.clone()])?;
                let r = results.first().map(|v| v.is_truthy()).unwrap_or(false);
                return Ok(r);
            }
            Ok(false)
        }
        _ => Ok(false),
    }
}

fn lua_less_than_with_metamethod(
    state: &mut VmState,
    a: &LuaValue,
    b: &LuaValue,
) -> Result<bool, LuaError> {
    match (a, b) {
        (LuaValue::Number(a), LuaValue::Number(b)) => Ok(a < b),
        (LuaValue::String(a), LuaValue::String(b)) => Ok(a.as_str() < b.as_str()),
        _ => {
            if let Some(mm) = get_binary_metamethod(state, a, b, "__lt") {
                let results = call_metamethod_native(state, &mm, &[a.clone(), b.clone()])?;
                Ok(results.first().map(|v| v.is_truthy()).unwrap_or(false))
            } else {
                Err(LuaError::Type(format!(
                    "attempt to compare {} with {}",
                    a.type_name(),
                    b.type_name()
                )))
            }
        }
    }
}

fn lua_less_equal_with_metamethod(
    state: &mut VmState,
    a: &LuaValue,
    b: &LuaValue,
) -> Result<bool, LuaError> {
    match (a, b) {
        (LuaValue::Number(a), LuaValue::Number(b)) => Ok(a <= b),
        (LuaValue::String(a), LuaValue::String(b)) => Ok(a.as_str() <= b.as_str()),
        _ => {
            if let Some(mm) = get_binary_metamethod(state, a, b, "__le") {
                let results = call_metamethod_native(state, &mm, &[a.clone(), b.clone()])?;
                Ok(results.first().map(|v| v.is_truthy()).unwrap_or(false))
            } else if let Some(mm) = get_binary_metamethod(state, b, a, "__lt") {
                // Fallback: a <= b iff not (b < a)
                let results = call_metamethod_native(state, &mm, &[b.clone(), a.clone()])?;
                Ok(!results.first().map(|v| v.is_truthy()).unwrap_or(false))
            } else {
                Err(LuaError::Type(format!(
                    "attempt to compare {} with {}",
                    a.type_name(),
                    b.type_name()
                )))
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rlua_core::bytecode::Instruction;
    use rlua_core::function::FunctionProto;
    use rlua_core::opcode::Opcode;

    fn make_proto(code: Vec<Instruction>, constants: Vec<LuaValue>) -> FunctionProto {
        FunctionProto {
            code,
            constants,
            max_stack_size: 10,
            ..FunctionProto::new()
        }
    }

    #[test]
    fn execute_return_number() {
        let proto = make_proto(
            vec![
                Instruction::encode_abx(Opcode::LoadK, 0, 0),
                Instruction::encode_abc(Opcode::Return, 0, 2, 0),
            ],
            vec![LuaValue::Number(42.0)],
        );
        let mut state = VmState::new();
        let results = execute(&mut state, proto).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], LuaValue::Number(42.0));
    }

    #[test]
    fn execute_arithmetic() {
        let proto = make_proto(
            vec![
                Instruction::encode_abx(Opcode::LoadK, 0, 0),
                Instruction::encode_abx(Opcode::LoadK, 1, 1),
                Instruction::encode_abc(Opcode::Add, 2, 0, 1),
                Instruction::encode_abc(Opcode::Return, 2, 2, 0),
            ],
            vec![LuaValue::Number(10.0), LuaValue::Number(3.0)],
        );
        let mut state = VmState::new();
        let results = execute(&mut state, proto).unwrap();
        assert_eq!(results[0], LuaValue::Number(13.0));
    }

    #[test]
    fn execute_global_set_get() {
        let proto = make_proto(
            vec![
                Instruction::encode_abx(Opcode::LoadK, 0, 1),
                Instruction::encode_abx(Opcode::SetGlobal, 0, 0),
                Instruction::encode_abx(Opcode::GetGlobal, 1, 0),
                Instruction::encode_abc(Opcode::Return, 1, 2, 0),
            ],
            vec![LuaValue::from("x"), LuaValue::Number(99.0)],
        );
        let mut state = VmState::new();
        let results = execute(&mut state, proto).unwrap();
        assert_eq!(results[0], LuaValue::Number(99.0));
    }

    #[test]
    fn execute_comparison() {
        let proto = make_proto(
            vec![
                Instruction::encode_abc(
                    Opcode::Lt,
                    1,
                    Instruction::rk_constant(0),
                    Instruction::rk_constant(1),
                ),
                Instruction::encode_asbx(Opcode::Jmp, 0, 2),
                Instruction::encode_abc(Opcode::LoadBool, 0, 1, 1),
                Instruction::encode_abc(Opcode::LoadBool, 0, 0, 0),
                Instruction::encode_abc(Opcode::Return, 0, 2, 0),
            ],
            vec![LuaValue::Number(1.0), LuaValue::Number(2.0)],
        );
        let mut state = VmState::new();
        let results = execute(&mut state, proto).unwrap();
        assert_eq!(results[0], LuaValue::Boolean(true));
    }

    #[test]
    fn execute_for_loop() {
        let proto = make_proto(
            vec![
                Instruction::encode_abx(Opcode::LoadK, 0, 0),
                Instruction::encode_abx(Opcode::LoadK, 1, 1),
                Instruction::encode_abx(Opcode::LoadK, 2, 2),
                Instruction::encode_abx(Opcode::LoadK, 3, 1),
                Instruction::encode_asbx(Opcode::ForPrep, 1, 1),
                Instruction::encode_abc(Opcode::Add, 0, 0, 4),
                Instruction::encode_asbx(Opcode::ForLoop, 1, -2),
                Instruction::encode_abc(Opcode::Return, 0, 2, 0),
            ],
            vec![
                LuaValue::Number(0.0),
                LuaValue::Number(1.0),
                LuaValue::Number(5.0),
            ],
        );
        let mut state = VmState::new();
        let results = execute(&mut state, proto).unwrap();
        assert_eq!(results[0], LuaValue::Number(15.0));
    }

    #[test]
    fn execute_native_call() {
        fn my_add(args: &[LuaValue]) -> Result<Vec<LuaValue>, String> {
            let a = args.first().and_then(|v| v.to_number()).unwrap_or(0.0);
            let b = args.get(1).and_then(|v| v.to_number()).unwrap_or(0.0);
            Ok(vec![LuaValue::Number(a + b)])
        }

        let proto = make_proto(
            vec![
                Instruction::encode_abx(Opcode::GetGlobal, 0, 0),
                Instruction::encode_abx(Opcode::LoadK, 1, 1),
                Instruction::encode_abx(Opcode::LoadK, 2, 2),
                Instruction::encode_abc(Opcode::Call, 0, 3, 2),
                Instruction::encode_abc(Opcode::Return, 0, 2, 0),
            ],
            vec![
                LuaValue::from("add"),
                LuaValue::Number(10.0),
                LuaValue::Number(20.0),
            ],
        );

        let mut state = VmState::new();
        state.register_global("add", my_add);
        let results = execute(&mut state, proto).unwrap();
        assert_eq!(results[0], LuaValue::Number(30.0));
    }

    #[test]
    fn execute_table_ops() {
        let proto = make_proto(
            vec![
                Instruction::encode_abc(Opcode::NewTable, 0, 0, 0),
                Instruction::encode_abc(
                    Opcode::SetTable,
                    0,
                    Instruction::rk_constant(0),
                    Instruction::rk_constant(1),
                ),
                Instruction::encode_abc(Opcode::GetTable, 1, 0, Instruction::rk_constant(0)),
                Instruction::encode_abc(Opcode::Return, 1, 2, 0),
            ],
            vec![LuaValue::Number(1.0), LuaValue::Number(42.0)],
        );
        let mut state = VmState::new();
        let results = execute(&mut state, proto).unwrap();
        assert_eq!(results[0], LuaValue::Number(42.0));
    }

    #[test]
    fn execute_concat() {
        let proto = make_proto(
            vec![
                Instruction::encode_abx(Opcode::LoadK, 0, 0),
                Instruction::encode_abx(Opcode::LoadK, 1, 1),
                Instruction::encode_abc(Opcode::Concat, 2, 0, 1),
                Instruction::encode_abc(Opcode::Return, 2, 2, 0),
            ],
            vec![LuaValue::from("hello"), LuaValue::from(" world")],
        );
        let mut state = VmState::new();
        let results = execute(&mut state, proto).unwrap();
        assert_eq!(results[0], LuaValue::from("hello world"));
    }
}
