use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use rlua_core::bytecode::RK_OFFSET;
use rlua_core::function::{Closure, FunctionProto, LuaFunction, NativeFn, UpvalRef};
use rlua_core::opcode::Opcode;
use rlua_core::table::LuaTable;
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
    base: usize, // base register index in the stack
    #[allow(dead_code)]
    num_results: i32, // expected results: -1 = variable, 0+ = fixed
    varargs: Vec<LuaValue>,
}

// ---------------------------------------------------------------------------
// VM State
// ---------------------------------------------------------------------------

pub struct VmState {
    stack: Vec<LuaValue>,
    frames: Vec<CallFrame>,
    globals: Rc<RefCell<LuaTable>>,
    /// Output captured during execution (for print).
    output: Vec<String>,
    /// Open upvalues: maps absolute stack index → shared UpvalRef.
    /// When a closure captures a stack slot, we create an UpvalRef here.
    /// Reading/writing that slot goes through the UpvalRef so closures see updates.
    open_upvalues: HashMap<usize, UpvalRef>,
}

impl VmState {
    pub fn new() -> Self {
        Self {
            stack: Vec::with_capacity(256),
            frames: Vec::new(),
            globals: Rc::new(RefCell::new(LuaTable::new())),
            output: Vec::new(),
            open_upvalues: HashMap::new(),
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
        // Check open upvalues first
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
        // If there's an open upvalue, update it
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

    /// Get or create an open upvalue for the given absolute stack index.
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

    /// Close upvalues at or above the given absolute stack index.
    fn close_upvalues(&mut self, from_abs: usize) {
        self.open_upvalues.retain(|&idx, _| idx < from_abs);
    }
}

impl Default for VmState {
    fn default() -> Self {
        Self::new()
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

    // Push args onto stack, fill with nil if needed
    let max_stack = proto.max_stack_size as usize;
    state.ensure_stack(base + max_stack.max(proto.num_params as usize + 1));

    for i in 0..proto.num_params as usize {
        let val = args.get(i).cloned().unwrap_or(LuaValue::Nil);
        state.set_reg(base, i as u8, val);
    }

    // Collect varargs (extra arguments beyond num_params)
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
        num_results: -1, // top-level: variable
        varargs,
    });

    let result = run_loop(state);

    // Clean up: close upvalues for this frame and truncate stack
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
                    state.frames.last_mut().unwrap().pc += 1; // skip next
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
                match table {
                    LuaValue::Table(t) => {
                        let val = t.borrow().rawget(&key);
                        state.set_reg(base, a, val);
                    }
                    _ => {
                        return Err(LuaError::Type(format!(
                            "attempt to index a {} value",
                            table.type_name()
                        )));
                    }
                }
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
                match table {
                    LuaValue::Table(t) => {
                        t.borrow_mut().rawset(key, val);
                    }
                    _ => {
                        return Err(LuaError::Type(format!(
                            "attempt to index a {} value",
                            table.type_name()
                        )));
                    }
                }
            }

            Opcode::NewTable => {
                let table = Rc::new(RefCell::new(LuaTable::new()));
                state.set_reg(base, a, LuaValue::Table(table));
            }

            Opcode::OpSelf => {
                let b = instr.b();
                let c = instr.c();
                let table = state.get_reg(base, b as u8).clone();
                let key = state.rk(base, &proto, c);
                // R(A+1) = R(B); R(A) = R(B)[RK(C)]
                state.set_reg(base, a + 1, table.clone());
                match table {
                    LuaValue::Table(t) => {
                        let val = t.borrow().rawget(&key);
                        state.set_reg(base, a, val);
                    }
                    _ => {
                        return Err(LuaError::Type(format!(
                            "attempt to index a {} value",
                            table.type_name()
                        )));
                    }
                }
            }

            Opcode::Add | Opcode::Sub | Opcode::Mul | Opcode::Div | Opcode::Mod | Opcode::Pow => {
                let b = instr.b();
                let c = instr.c();
                let lhs = state.rk(base, &proto, b);
                let rhs = state.rk(base, &proto, c);
                let ln = to_number(&lhs)?;
                let rn = to_number(&rhs)?;
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
            }

            Opcode::Unm => {
                let b = instr.b();
                let val = state.get_reg(base, b as u8).clone();
                let n = to_number(&val)?;
                state.set_reg(base, a, LuaValue::Number(-n));
            }

            Opcode::Not => {
                let b = instr.b();
                let val = state.get_reg(base, b as u8).clone();
                state.set_reg(base, a, LuaValue::Boolean(!val.is_truthy()));
            }

            Opcode::Len => {
                let b = instr.b();
                let val = state.get_reg(base, b as u8).clone();
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

            Opcode::Concat => {
                let b = instr.b() as u8;
                let c = instr.c() as u8;
                let mut result = String::new();
                for i in b..=c {
                    let val = state.get_reg(base, i).clone();
                    match &val {
                        LuaValue::String(s) => result.push_str(s),
                        LuaValue::Number(_) => result.push_str(&val.to_lua_string()),
                        _ => {
                            return Err(LuaError::Type(format!(
                                "attempt to concatenate a {} value",
                                val.type_name()
                            )));
                        }
                    }
                }
                state.set_reg(base, a, LuaValue::from(result));
            }

            Opcode::Jmp => {
                let sbx = instr.sbx();
                let new_pc = (state.frames.last().unwrap().pc as i32 + sbx) as usize;
                state.frames.last_mut().unwrap().pc = new_pc;
            }

            Opcode::Eq => {
                let b = instr.b();
                let c = instr.c();
                let lhs = state.rk(base, &proto, b);
                let rhs = state.rk(base, &proto, c);
                let equal = lua_equal(&lhs, &rhs);
                let expected = a != 0; // A=1 means skip if equal, A=0 means skip if not equal
                if equal == expected {
                    // condition met: skip next instruction
                    state.frames.last_mut().unwrap().pc += 1;
                }
            }

            Opcode::Lt => {
                let b = instr.b();
                let c = instr.c();
                let lhs = state.rk(base, &proto, b);
                let rhs = state.rk(base, &proto, c);
                let result = lua_less_than(&lhs, &rhs)?;
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
                let result = lua_less_equal(&lhs, &rhs)?;
                let expected = a != 0;
                if result == expected {
                    state.frames.last_mut().unwrap().pc += 1;
                }
            }

            Opcode::Test => {
                let c = instr.c();
                let val = state.get_reg(base, a).clone();
                // If (bool(val) != C) then skip next instruction
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

                let num_args = if b == 0 {
                    // Variable args: everything from a+1 to top
                    state.stack.len() - (base + a as usize + 1)
                } else {
                    (b - 1) as usize
                };

                let args: Vec<LuaValue> = (0..num_args)
                    .map(|i| state.get_reg(base, a + 1 + i as u8).clone())
                    .collect();

                let num_results_wanted = if c == 0 {
                    -1i32 // variable
                } else {
                    (c - 1) as i32
                };

                // Special handling for pcall
                let is_pcall = matches!(&func_val, LuaValue::Function(f) if matches!(f.as_ref(), LuaFunction::Native { name: "pcall", .. }));

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
                            let msg = match e {
                                LuaError::Runtime(s) => LuaValue::from(s),
                                LuaError::Type(s) => LuaValue::from(s),
                                LuaError::Arithmetic(s) => LuaValue::from(s),
                            };
                            vec![LuaValue::Boolean(false), msg]
                        }
                    }
                } else {
                    call_function(state, &func_val, &args)?
                };

                // Place results starting at R(A)
                let num_results = results.len();
                if num_results_wanted < 0 {
                    // Variable: push all results
                    for (i, val) in results.into_iter().enumerate() {
                        state.set_reg(base, a + i as u8, val);
                    }
                    // Set stack top
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

                let num_args = if b == 0 {
                    state.stack.len() - (base + a as usize + 1)
                } else {
                    (b - 1) as usize
                };

                let args: Vec<LuaValue> = (0..num_args)
                    .map(|i| state.get_reg(base, a + 1 + i as u8).clone())
                    .collect();

                // For M1, implement as a regular call
                let results = call_function(state, &func_val, &args)?;
                return Ok(results);
            }

            Opcode::Return => {
                let b = instr.b();
                if b == 0 {
                    // Return values from R(A) to top
                    let top = state.stack.len();
                    let start = base + a as usize;
                    let results: Vec<LuaValue> = if start < top {
                        state.stack[start..top].to_vec()
                    } else {
                        Vec::new()
                    };
                    return Ok(results);
                } else if b == 1 {
                    // No return values
                    return Ok(Vec::new());
                } else {
                    // Return B-1 values from R(A)
                    let count = (b - 1) as usize;
                    let results: Vec<LuaValue> = (0..count)
                        .map(|i| state.get_reg(base, a + i as u8).clone())
                        .collect();
                    return Ok(results);
                }
            }

            Opcode::ForLoop => {
                // R(A) += R(A+2); if R(A) <?= R(A+1) then { pc += sBx; R(A+3) = R(A) }
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
                // R(A) -= R(A+2); pc += sBx
                let index = to_number(&state.get_reg(base, a))?;
                let step = to_number(&state.get_reg(base, a + 2))?;
                state.set_reg(base, a, LuaValue::Number(index - step));
                let sbx = instr.sbx();
                let new_pc = (state.frames.last().unwrap().pc as i32 + sbx) as usize;
                state.frames.last_mut().unwrap().pc = new_pc;
            }

            Opcode::TForLoop => {
                // Call R(A)(R(A+1), R(A+2)); results in R(A+3)..R(A+2+C)
                let c = instr.c() as usize;
                let func = state.get_reg(base, a).clone();
                let s = state.get_reg(base, a + 1).clone();
                let var = state.get_reg(base, a + 2).clone();

                let results = call_function(state, &func, &[s, var])?;

                for i in 0..c {
                    let val = results.get(i).cloned().unwrap_or(LuaValue::Nil);
                    state.set_reg(base, a + 3 + i as u8, val);
                }

                // If first result is not nil, copy to control variable
                let first = state.get_reg(base, a + 3).clone();
                if first != LuaValue::Nil {
                    state.set_reg(base, a + 2, first);
                } else {
                    // Skip the JMP that follows (end loop)
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
                    let offset = (c as usize - 1) * 50; // LFIELDS_PER_FLUSH
                    for i in 1..=count {
                        let val = state.get_reg(base, a + i as u8).clone();
                        let key = LuaValue::Number((offset + i) as f64);
                        t.borrow_mut().rawset(key, val);
                    }
                }
            }

            Opcode::Close => {
                // Close open upvalues at or above R(A)
                state.close_upvalues(base + a as usize);
            }

            Opcode::Closure => {
                let bx = instr.bx() as usize;
                let child_proto = Rc::new(proto.prototypes[bx].clone());
                let mut closure = Closure::new(child_proto.clone());

                // Capture upvalues using shared references (open upvalues)
                for uv_desc in &child_proto.upvalue_descs {
                    let uv_ref = if uv_desc.in_stack {
                        // Capture from current stack — create/reuse open upvalue
                        let abs_idx = base + uv_desc.index as usize;
                        state.get_or_create_open_upvalue(abs_idx)
                    } else {
                        // Capture from parent's upvalue
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
            }

            Opcode::Vararg => {
                // VARARG A B: load B-1 varargs into R(A), R(A+1), ...
                // B=0 means load all
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
                    // Adjust stack top for variable results
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
        _ => Err(LuaError::Type(format!(
            "attempt to call a {} value",
            func_val.type_name()
        ))),
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

fn lua_equal(a: &LuaValue, b: &LuaValue) -> bool {
    a == b
}

fn lua_less_than(a: &LuaValue, b: &LuaValue) -> Result<bool, LuaError> {
    match (a, b) {
        (LuaValue::Number(a), LuaValue::Number(b)) => Ok(a < b),
        (LuaValue::String(a), LuaValue::String(b)) => Ok(a.as_str() < b.as_str()),
        _ => Err(LuaError::Type(format!(
            "attempt to compare {} with {}",
            a.type_name(),
            b.type_name()
        ))),
    }
}

fn lua_less_equal(a: &LuaValue, b: &LuaValue) -> Result<bool, LuaError> {
    match (a, b) {
        (LuaValue::Number(a), LuaValue::Number(b)) => Ok(a <= b),
        (LuaValue::String(a), LuaValue::String(b)) => Ok(a.as_str() <= b.as_str()),
        _ => Err(LuaError::Type(format!(
            "attempt to compare {} with {}",
            a.type_name(),
            b.type_name()
        ))),
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
        // R(0) = 10, R(1) = 3, R(2) = R(0) + R(1), return R(2)
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
                Instruction::encode_abx(Opcode::LoadK, 0, 1), // R(0) = 99
                Instruction::encode_abx(Opcode::SetGlobal, 0, 0), // _G["x"] = R(0)
                Instruction::encode_abx(Opcode::GetGlobal, 1, 0), // R(1) = _G["x"]
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
        // if 1 < 2 then return true else return false
        let proto = make_proto(
            vec![
                // LT 1 K(0) K(1) -- if 1 < 2, skip next
                Instruction::encode_abc(
                    Opcode::Lt,
                    1,
                    Instruction::rk_constant(0),
                    Instruction::rk_constant(1),
                ),
                // JMP +2 (skip the true branch)
                Instruction::encode_asbx(Opcode::Jmp, 0, 2),
                // LoadBool R(0) true, skip next
                Instruction::encode_abc(Opcode::LoadBool, 0, 1, 1),
                // LoadBool R(0) false
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
        // sum = 0; for i = 1, 5 do sum = sum + i end; return sum
        let proto = make_proto(
            vec![
                // R(0) = 0 (sum)
                Instruction::encode_abx(Opcode::LoadK, 0, 0),
                // R(1) = 1 (init), R(2) = 5 (limit), R(3) = 1 (step)
                Instruction::encode_abx(Opcode::LoadK, 1, 1),
                Instruction::encode_abx(Opcode::LoadK, 2, 2),
                Instruction::encode_abx(Opcode::LoadK, 3, 1),
                // FORPREP R(1) +1 (jump to FORLOOP)
                Instruction::encode_asbx(Opcode::ForPrep, 1, 1),
                // Body: R(0) = R(0) + R(4)
                Instruction::encode_abc(Opcode::Add, 0, 0, 4),
                // FORLOOP R(1) -2 (jump back to body)
                Instruction::encode_asbx(Opcode::ForLoop, 1, -2),
                // return R(0)
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
        assert_eq!(results[0], LuaValue::Number(15.0)); // 1+2+3+4+5
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
                Instruction::encode_abx(Opcode::GetGlobal, 0, 0), // R(0) = _G["add"]
                Instruction::encode_abx(Opcode::LoadK, 1, 1),     // R(1) = 10
                Instruction::encode_abx(Opcode::LoadK, 2, 2),     // R(2) = 20
                Instruction::encode_abc(Opcode::Call, 0, 3, 2),   // R(0) = R(0)(R(1), R(2))
                Instruction::encode_abc(Opcode::Return, 0, 2, 0), // return R(0)
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
                // t[1] = 42
                Instruction::encode_abc(
                    Opcode::SetTable,
                    0,
                    Instruction::rk_constant(0),
                    Instruction::rk_constant(1),
                ),
                // R(1) = t[1]
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
