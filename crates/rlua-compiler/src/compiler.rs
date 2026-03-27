use std::rc::Rc;

use rlua_core::bytecode::{Instruction, RK_OFFSET};
use rlua_core::function::{FunctionProto, UpvalueDesc};
use rlua_core::opcode::Opcode;
use rlua_core::value::LuaValue;
use rlua_parser::ParseError;
use rlua_parser::ast::*;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum CompileError {
    Parse(ParseError),
    TooManyLocals,
    TooManyConstants,
    TooManyUpvalues,
    BreakOutsideLoop,
    VarargOutsideVarargFunc,
    General(String),
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parse(e) => write!(f, "parse error [{}:{}]: {}", e.line, e.column, e.message),
            Self::TooManyLocals => write!(f, "too many local variables (limit 200)"),
            Self::TooManyConstants => write!(f, "too many constants"),
            Self::TooManyUpvalues => write!(f, "too many upvalues (limit 255)"),
            Self::BreakOutsideLoop => write!(f, "'break' outside loop"),
            Self::VarargOutsideVarargFunc => write!(f, "'...' outside vararg function"),
            Self::General(msg) => write!(f, "compile error: {msg}"),
        }
    }
}

// ---------------------------------------------------------------------------
// Local variable & scope tracking
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct Local {
    name: String,
    register: u8,
    depth: usize,
    #[allow(dead_code)]
    start_pc: u32,
}

// ---------------------------------------------------------------------------
// Per-function compilation state
// ---------------------------------------------------------------------------

struct FuncState {
    proto: FunctionProto,
    locals: Vec<Local>,
    scope_depth: usize,
    free_reg: u8,
    /// Stack of break jump lists. Each loop pushes a Vec, break appends to it,
    /// and loop end patches them all.
    break_lists: Vec<Vec<usize>>,
    /// Upvalue info for this function.
    upvalues: Vec<UpvalInfo>,
    is_vararg: bool,
    #[allow(dead_code)]
    num_params: u8,
    /// Parent function's locals (for upvalue resolution). Each entry is (name, register).
    enclosing_locals: Vec<(String, u8)>,
    /// Parent function's upvalues (for chained upvalue resolution). Each entry is (name, upvalue_index).
    enclosing_upvalues: Vec<(String, u8)>,
    /// Grandparent function's locals (for 3+ level upvalue resolution).
    grandparent_locals: Vec<(String, u8)>,
    /// Grandparent function's upvalues (for 3+ level upvalue resolution).
    grandparent_upvalues: Vec<(String, u8)>,
    /// Current source line being compiled — written into line_info for each emitted instruction.
    current_line: u32,
}

#[derive(Debug, Clone)]
struct UpvalInfo {
    name: String,
    in_stack: bool,
    index: u8,
}

impl FuncState {
    fn new(source: &str, is_vararg: bool, num_params: u8) -> Self {
        Self {
            proto: FunctionProto {
                source_name: source.to_owned(),
                num_params,
                is_vararg,
                max_stack_size: 2, // minimum per Lua spec
                ..FunctionProto::new()
            },
            locals: Vec::new(),
            scope_depth: 0,
            free_reg: 0,
            break_lists: Vec::new(),
            upvalues: Vec::new(),
            is_vararg,
            num_params,
            enclosing_locals: Vec::new(),
            enclosing_upvalues: Vec::new(),
            grandparent_locals: Vec::new(),
            grandparent_upvalues: Vec::new(),
            current_line: 0,
        }
    }

    fn emit(&mut self, instr: Instruction) -> usize {
        let pc = self.proto.code.len();
        self.proto.code.push(instr);
        self.proto.line_info.push(self.current_line);
        pc
    }

    fn emit_abc(&mut self, op: Opcode, a: u8, b: u16, c: u16) -> usize {
        self.emit(Instruction::encode_abc(op, a, b, c))
    }

    fn emit_abx(&mut self, op: Opcode, a: u8, bx: u32) -> usize {
        self.emit(Instruction::encode_abx(op, a, bx))
    }

    fn emit_asbx(&mut self, op: Opcode, a: u8, sbx: i32) -> usize {
        self.emit(Instruction::encode_asbx(op, a, sbx))
    }

    fn current_pc(&self) -> usize {
        self.proto.code.len()
    }

    fn alloc_reg(&mut self) -> Result<u8, CompileError> {
        let r = self.free_reg;
        if r >= 249 {
            return Err(CompileError::TooManyLocals);
        }
        self.free_reg = r + 1;
        if self.free_reg > self.proto.max_stack_size {
            self.proto.max_stack_size = self.free_reg;
        }
        Ok(r)
    }

    fn free_reg_to(&mut self, r: u8) {
        self.free_reg = r;
    }

    #[allow(dead_code)]
    fn reserve_regs(&mut self, n: u8) -> Result<u8, CompileError> {
        let base = self.free_reg;
        for _ in 0..n {
            self.alloc_reg()?;
        }
        Ok(base)
    }

    // -- Constants --

    fn add_constant(&mut self, val: LuaValue) -> Result<u32, CompileError> {
        // Check for duplicate
        for (i, existing) in self.proto.constants.iter().enumerate() {
            if constants_equal(existing, &val) {
                return Ok(i as u32);
            }
        }
        let idx = self.proto.constants.len();
        if idx > u32::MAX as usize {
            return Err(CompileError::TooManyConstants);
        }
        self.proto.constants.push(val);
        Ok(idx as u32)
    }

    fn add_string_constant(&mut self, s: &str) -> Result<u32, CompileError> {
        self.add_constant(LuaValue::String(Rc::new(s.to_owned())))
    }

    fn add_number_constant(&mut self, n: f64) -> Result<u32, CompileError> {
        self.add_constant(LuaValue::Number(n))
    }

    /// Return an RK value for a constant. If the constant index fits in 9 bits
    /// (i.e. < 256), we can encode it as RK. Otherwise we need to load it.
    fn constant_rk(&mut self, val: LuaValue) -> Result<u16, CompileError> {
        let idx = self.add_constant(val)?;
        if idx < RK_OFFSET as u32 {
            Ok(Instruction::rk_constant(idx as u16))
        } else {
            // Too many constants for RK encoding — fall back to loading into register
            Err(CompileError::TooManyConstants)
        }
    }

    // -- Locals & Scopes --

    fn enter_scope(&mut self) {
        self.scope_depth += 1;
    }

    fn leave_scope(&mut self) {
        let depth = self.scope_depth;
        while let Some(last) = self.locals.last() {
            if last.depth < depth {
                break;
            }
            self.locals.pop();
        }
        if let Some(last) = self.locals.last() {
            self.free_reg = last.register + 1;
        } else {
            self.free_reg = 0;
        }
        self.scope_depth -= 1;
    }

    fn add_local(&mut self, name: &str) -> Result<u8, CompileError> {
        let reg = self.alloc_reg()?;
        self.locals.push(Local {
            name: name.to_owned(),
            register: reg,
            depth: self.scope_depth,
            start_pc: self.current_pc() as u32,
        });
        Ok(reg)
    }

    /// Declare a local without allocating a register (register is pre-allocated).
    fn declare_local(&mut self, name: &str, reg: u8) {
        self.locals.push(Local {
            name: name.to_owned(),
            register: reg,
            depth: self.scope_depth,
            start_pc: self.current_pc() as u32,
        });
    }

    fn resolve_local(&self, name: &str) -> Option<u8> {
        for local in self.locals.iter().rev() {
            if local.name == name {
                return Some(local.register);
            }
        }
        None
    }

    // -- Upvalues --

    fn resolve_upvalue(&mut self, name: &str) -> Option<u8> {
        // Check existing upvalues
        for (i, uv) in self.upvalues.iter().enumerate() {
            if uv.name == name {
                return Some(i as u8);
            }
        }
        // Check enclosing locals (parent's locals → in_stack=true)
        for (enc_name, enc_reg) in &self.enclosing_locals {
            if enc_name == name {
                let idx = self.upvalues.len() as u8;
                self.upvalues.push(UpvalInfo {
                    name: name.to_owned(),
                    in_stack: true,
                    index: *enc_reg,
                });
                return Some(idx);
            }
        }
        // Check enclosing upvalues (parent's upvalues → in_stack=false)
        for (enc_name, enc_idx) in &self.enclosing_upvalues {
            if enc_name == name {
                let idx = self.upvalues.len() as u8;
                self.upvalues.push(UpvalInfo {
                    name: name.to_owned(),
                    in_stack: false,
                    index: *enc_idx,
                });
                return Some(idx);
            }
        }
        // Check grandparent context (variables 2+ levels up)
        // These will need the parent to create an upvalue too (done in post-processing)
        let gp_locals = self.grandparent_locals.clone();
        for (enc_name, _enc_reg) in &gp_locals {
            if enc_name == name {
                let idx = self.upvalues.len() as u8;
                self.upvalues.push(UpvalInfo {
                    name: name.to_owned(),
                    in_stack: false,
                    index: 255, // placeholder — will be fixed in post-processing
                });
                return Some(idx);
            }
        }
        let gp_upvalues = self.grandparent_upvalues.clone();
        for (enc_name, _enc_idx) in &gp_upvalues {
            if enc_name == name {
                let idx = self.upvalues.len() as u8;
                self.upvalues.push(UpvalInfo {
                    name: name.to_owned(),
                    in_stack: false,
                    index: 255, // placeholder — will be fixed in post-processing
                });
                return Some(idx);
            }
        }
        None
    }

    #[allow(dead_code)]
    fn add_upvalue(&mut self, name: &str, in_stack: bool, index: u8) -> Result<u8, CompileError> {
        // Check if already exists
        if let Some(idx) = self.resolve_upvalue(name) {
            return Ok(idx);
        }
        let idx = self.upvalues.len();
        if idx >= 255 {
            return Err(CompileError::TooManyUpvalues);
        }
        self.upvalues.push(UpvalInfo {
            name: name.to_owned(),
            in_stack,
            index,
        });
        Ok(idx as u8)
    }

    // -- Jump patching --

    fn emit_jump(&mut self) -> usize {
        self.emit_asbx(Opcode::Jmp, 0, 0) // placeholder, will patch
    }

    fn patch_jump(&mut self, jmp_pc: usize) {
        let target = self.current_pc();
        let offset = target as i32 - jmp_pc as i32 - 1;
        self.proto.code[jmp_pc] = Instruction::encode_asbx(Opcode::Jmp, 0, offset);
    }

    fn patch_jump_to(&mut self, jmp_pc: usize, target: usize) {
        let offset = target as i32 - jmp_pc as i32 - 1;
        self.proto.code[jmp_pc] = Instruction::encode_asbx(Opcode::Jmp, 0, offset);
    }

    fn finalize(mut self) -> FunctionProto {
        self.proto.num_upvalues = self.upvalues.len() as u8;
        self.proto.upvalue_descs = self
            .upvalues
            .iter()
            .map(|uv| UpvalueDesc {
                in_stack: uv.in_stack,
                index: uv.index,
            })
            .collect();
        self.proto.upvalue_names = self.upvalues.iter().map(|uv| uv.name.clone()).collect();
        self.proto
    }
}

/// Compare constants for deduplication (NaN-safe).
fn constants_equal(a: &LuaValue, b: &LuaValue) -> bool {
    match (a, b) {
        (LuaValue::Nil, LuaValue::Nil) => true,
        (LuaValue::Boolean(a), LuaValue::Boolean(b)) => a == b,
        (LuaValue::Number(a), LuaValue::Number(b)) => a.to_bits() == b.to_bits(),
        (LuaValue::String(a), LuaValue::String(b)) => a == b,
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// Compiler
// ---------------------------------------------------------------------------

pub struct Compiler<'src> {
    source: &'src str,
}

impl<'src> Compiler<'src> {
    pub fn new(source: &'src str) -> Self {
        Self { source }
    }

    pub fn compile_main(&mut self, block: &Block) -> Result<FunctionProto, CompileError> {
        let mut fs = FuncState::new(self.source, true, 0);
        self.compile_block(&mut fs, block)?;
        // Implicit return at end of main chunk
        fs.emit_abc(Opcode::Return, 0, 1, 0);
        Ok(fs.finalize())
    }

    // -- Block & Statements --

    fn compile_block(&mut self, fs: &mut FuncState, block: &Block) -> Result<(), CompileError> {
        for spanned in &block.stmts {
            fs.current_line = spanned.line;
            self.compile_stmt(fs, &spanned.stmt)?;
        }
        if let Some(ref ret_exprs) = block.ret {
            fs.current_line = block.ret_line;
            self.compile_return(fs, ret_exprs)?;
        }
        Ok(())
    }

    fn compile_stmt(&mut self, fs: &mut FuncState, stmt: &Stmt) -> Result<(), CompileError> {
        match stmt {
            Stmt::LocalAssign { names, values } => self.compile_local_assign(fs, names, values),
            Stmt::Assign { targets, values } => self.compile_assign(fs, targets, values),
            Stmt::FunctionCall(call) => {
                let reg = fs.free_reg;
                self.compile_call_expr(fs, &call.callee, &call.args, reg, 1)?;
                // C=1 means 0 results (discard)
                fs.free_reg_to(reg);
                Ok(())
            }
            Stmt::MethodCall {
                object,
                method,
                args,
            } => {
                let reg = fs.free_reg;
                self.compile_method_call_expr(fs, object, method, args, reg, 1)?;
                fs.free_reg_to(reg);
                Ok(())
            }
            Stmt::DoBlock { body } => {
                fs.enter_scope();
                self.compile_block(fs, body)?;
                fs.leave_scope();
                Ok(())
            }
            Stmt::While { condition, body } => self.compile_while(fs, condition, body),
            Stmt::Repeat { body, condition } => self.compile_repeat(fs, body, condition),
            Stmt::If {
                condition,
                then_body,
                elseif_clauses,
                else_body,
            } => self.compile_if(fs, condition, then_body, elseif_clauses, else_body),
            Stmt::NumericFor {
                name,
                start,
                limit,
                step,
                body,
            } => self.compile_numeric_for(fs, name, start, limit, step.as_ref(), body),
            Stmt::GenericFor {
                names,
                iterators,
                body,
            } => self.compile_generic_for(fs, names, iterators, body),
            Stmt::LocalFunction {
                name,
                params,
                is_vararg,
                body,
            } => self.compile_local_function(fs, name, params, *is_vararg, body),
            Stmt::FunctionDef {
                name,
                params,
                is_vararg,
                body,
            } => self.compile_function_def(fs, name, params, *is_vararg, body),
            Stmt::Break => self.compile_break(fs),
        }
    }

    fn compile_local_assign(
        &mut self,
        fs: &mut FuncState,
        names: &[String],
        values: &[Expr],
    ) -> Result<(), CompileError> {
        let base = fs.free_reg;

        if values.is_empty() {
            // local x, y, z  (all nil)
            for name in names {
                let reg = fs.add_local(name)?;
                fs.emit_abc(Opcode::LoadNil, reg, 0, 0);
            }
            return Ok(());
        }

        // Compile values
        let num_names = names.len();
        let num_values = values.len();

        for (i, val) in values.iter().enumerate() {
            let is_last = i == num_values - 1;
            if is_last && num_names > num_values {
                // Last value: if it's a call or vararg, request enough results
                let needed = (num_names - i) as u16;
                let c_val = needed + 1; // C encoding: needed results + 1
                match val {
                    Expr::FunctionCall(call) => {
                        let dest = base + i as u8;
                        self.compile_call_expr(fs, &call.callee, &call.args, dest, c_val)?;
                        fs.free_reg_to(base + num_names as u8);
                        break;
                    }
                    Expr::MethodCall {
                        object,
                        method,
                        args,
                    } => {
                        let dest = base + i as u8;
                        self.compile_method_call_expr(fs, object, method, args, dest, c_val)?;
                        fs.free_reg_to(base + num_names as u8);
                        break;
                    }
                    Expr::Vararg => {
                        if !fs.is_vararg {
                            return Err(CompileError::VarargOutsideVarargFunc);
                        }
                        let dest = base + i as u8;
                        fs.emit_abc(Opcode::Vararg, dest, c_val, 0);
                        fs.free_reg_to(base + num_names as u8);
                        break;
                    }
                    _ => {
                        let reg = fs.alloc_reg()?;
                        self.compile_expr_to_reg(fs, val, reg)?;
                    }
                }
            } else {
                let reg = fs.alloc_reg()?;
                self.compile_expr_to_reg(fs, val, reg)?;
            }
        }

        // Fill remaining with nil if needed
        let regs_filled = fs.free_reg - base;
        if (regs_filled as usize) < num_names {
            let start_nil = base + regs_filled;
            for _ in regs_filled as usize..num_names {
                fs.alloc_reg()?;
            }
            let count = num_names as u8 - regs_filled;
            fs.emit_abc(
                Opcode::LoadNil,
                start_nil,
                count.saturating_sub(1) as u16,
                0,
            );
        }

        // Now register the locals (this prevents them from being visible during init)
        fs.free_reg_to(base);
        for name in names {
            fs.add_local(name)?;
        }

        Ok(())
    }

    fn compile_assign(
        &mut self,
        fs: &mut FuncState,
        targets: &[Expr],
        values: &[Expr],
    ) -> Result<(), CompileError> {
        let base = fs.free_reg;
        let num_targets = targets.len();
        let num_values = values.len();

        // First, compile all values into temporary registers
        let val_base = base;
        for (i, val) in values.iter().enumerate() {
            let is_last = i == num_values - 1;
            if is_last && num_targets > num_values {
                let needed = (num_targets - i) as u16;
                let c_val = needed + 1;
                match val {
                    Expr::FunctionCall(call) => {
                        let dest = val_base + i as u8;
                        self.compile_call_expr(fs, &call.callee, &call.args, dest, c_val)?;
                        fs.free_reg_to(val_base + num_targets as u8);
                        break;
                    }
                    Expr::MethodCall {
                        object,
                        method,
                        args,
                    } => {
                        let dest = val_base + i as u8;
                        self.compile_method_call_expr(fs, object, method, args, dest, c_val)?;
                        fs.free_reg_to(val_base + num_targets as u8);
                        break;
                    }
                    Expr::Vararg => {
                        if !fs.is_vararg {
                            return Err(CompileError::VarargOutsideVarargFunc);
                        }
                        let dest = val_base + i as u8;
                        fs.emit_abc(Opcode::Vararg, dest, c_val, 0);
                        fs.free_reg_to(val_base + num_targets as u8);
                        break;
                    }
                    _ => {
                        let reg = fs.alloc_reg()?;
                        self.compile_expr_to_reg(fs, val, reg)?;
                    }
                }
            } else {
                let reg = fs.alloc_reg()?;
                self.compile_expr_to_reg(fs, val, reg)?;
            }
        }

        // Fill remaining with nil
        let regs_filled = (fs.free_reg - val_base) as usize;
        if regs_filled < num_targets {
            let start_nil = val_base + regs_filled as u8;
            for _ in regs_filled..num_targets {
                fs.alloc_reg()?;
            }
            let count = num_targets - regs_filled;
            fs.emit_abc(
                Opcode::LoadNil,
                start_nil,
                count.saturating_sub(1) as u16,
                0,
            );
        }

        // Now assign each target from its temporary register
        for (i, target) in targets.iter().enumerate() {
            let src_reg = val_base + i as u8;
            match target {
                Expr::Name(name) => {
                    if let Some(local_reg) = fs.resolve_local(name) {
                        if local_reg != src_reg {
                            fs.emit_abc(Opcode::Move, local_reg, src_reg as u16, 0);
                        }
                    } else if let Some(uv_idx) = fs.resolve_upvalue(name) {
                        fs.emit_abc(Opcode::SetUpval, src_reg, uv_idx as u16, 0);
                    } else {
                        let k = fs.add_string_constant(name)?;
                        fs.emit_abx(Opcode::SetGlobal, src_reg, k);
                    }
                }
                Expr::Index { table, key } => {
                    let table_reg = fs.alloc_reg()?;
                    self.compile_expr_to_reg(fs, table, table_reg)?;
                    let key_rk = self.expr_to_rk(fs, key)?;
                    fs.emit_abc(Opcode::SetTable, table_reg, key_rk, src_reg as u16);
                    fs.free_reg_to(table_reg);
                }
                Expr::Field { table, name } => {
                    let table_reg = fs.alloc_reg()?;
                    self.compile_expr_to_reg(fs, table, table_reg)?;
                    let key_rk = fs.constant_rk(LuaValue::String(Rc::new(name.clone())))?;
                    fs.emit_abc(Opcode::SetTable, table_reg, key_rk, src_reg as u16);
                    fs.free_reg_to(table_reg);
                }
                _ => {
                    return Err(CompileError::General(
                        "invalid assignment target".to_owned(),
                    ));
                }
            }
        }

        fs.free_reg_to(base);
        Ok(())
    }

    fn compile_while(
        &mut self,
        fs: &mut FuncState,
        condition: &Expr,
        body: &Block,
    ) -> Result<(), CompileError> {
        let loop_start = fs.current_pc();
        fs.break_lists.push(Vec::new());

        // Compile condition
        let cond_reg = fs.alloc_reg()?;
        self.compile_expr_to_reg(fs, condition, cond_reg)?;
        fs.emit_abc(Opcode::Test, cond_reg, 0, 0); // test: if falsy, skip next
        let exit_jmp = fs.emit_jump();
        fs.free_reg_to(cond_reg);

        // Body
        fs.enter_scope();
        self.compile_block(fs, body)?;
        fs.leave_scope();

        // Jump back to loop start
        let back_jmp = fs.current_pc();
        fs.emit_asbx(Opcode::Jmp, 0, 0);
        fs.patch_jump_to(back_jmp, loop_start);

        // Patch exit
        fs.patch_jump(exit_jmp);

        // Patch breaks
        let breaks = fs.break_lists.pop().unwrap();
        for brk in breaks {
            fs.patch_jump(brk);
        }

        Ok(())
    }

    fn compile_repeat(
        &mut self,
        fs: &mut FuncState,
        body: &Block,
        condition: &Expr,
    ) -> Result<(), CompileError> {
        let loop_start = fs.current_pc();
        fs.break_lists.push(Vec::new());

        fs.enter_scope();
        self.compile_block(fs, body)?;

        // Condition
        let cond_reg = fs.alloc_reg()?;
        self.compile_expr_to_reg(fs, condition, cond_reg)?;
        fs.emit_abc(Opcode::Test, cond_reg, 0, 0); // if falsy, skip next
        let cont_jmp = fs.current_pc();
        fs.emit_asbx(Opcode::Jmp, 0, 0);
        fs.patch_jump_to(cont_jmp, loop_start);
        fs.free_reg_to(cond_reg);

        fs.leave_scope();

        // Patch breaks
        let breaks = fs.break_lists.pop().unwrap();
        for brk in breaks {
            fs.patch_jump(brk);
        }

        Ok(())
    }

    fn compile_if(
        &mut self,
        fs: &mut FuncState,
        condition: &Expr,
        then_body: &Block,
        elseif_clauses: &[(Expr, Block)],
        else_body: &Option<Block>,
    ) -> Result<(), CompileError> {
        let mut exit_jumps = Vec::new();

        // Main if
        let cond_reg = fs.alloc_reg()?;
        self.compile_expr_to_reg(fs, condition, cond_reg)?;
        fs.emit_abc(Opcode::Test, cond_reg, 0, 0);
        let false_jmp = fs.emit_jump();
        fs.free_reg_to(cond_reg);

        fs.enter_scope();
        self.compile_block(fs, then_body)?;
        fs.leave_scope();

        if !elseif_clauses.is_empty() || else_body.is_some() {
            exit_jumps.push(fs.emit_jump());
        }
        fs.patch_jump(false_jmp);

        // Elseif clauses
        for (ei_cond, ei_body) in elseif_clauses {
            let cond_reg = fs.alloc_reg()?;
            self.compile_expr_to_reg(fs, ei_cond, cond_reg)?;
            fs.emit_abc(Opcode::Test, cond_reg, 0, 0);
            let false_jmp = fs.emit_jump();
            fs.free_reg_to(cond_reg);

            fs.enter_scope();
            self.compile_block(fs, ei_body)?;
            fs.leave_scope();

            exit_jumps.push(fs.emit_jump());
            fs.patch_jump(false_jmp);
        }

        // Else clause
        if let Some(else_block) = else_body {
            fs.enter_scope();
            self.compile_block(fs, else_block)?;
            fs.leave_scope();
        }

        // Patch all exit jumps to here
        for jmp in exit_jumps {
            fs.patch_jump(jmp);
        }

        Ok(())
    }

    fn compile_numeric_for(
        &mut self,
        fs: &mut FuncState,
        name: &str,
        start: &Expr,
        limit: &Expr,
        step: Option<&Expr>,
        body: &Block,
    ) -> Result<(), CompileError> {
        fs.enter_scope();
        fs.break_lists.push(Vec::new());

        // Reserve 3 internal registers + 1 external variable
        let base = fs.free_reg;
        let r_init = fs.alloc_reg()?; // (internal) index
        let r_limit = fs.alloc_reg()?; // (internal) limit
        let r_step = fs.alloc_reg()?; // (internal) step
        let r_var = fs.alloc_reg()?; // (external) loop variable

        self.compile_expr_to_reg(fs, start, r_init)?;
        self.compile_expr_to_reg(fs, limit, r_limit)?;
        if let Some(step_expr) = step {
            self.compile_expr_to_reg(fs, step_expr, r_step)?;
        } else {
            // Default step = 1
            let k = fs.add_number_constant(1.0)?;
            fs.emit_abx(Opcode::LoadK, r_step, k);
        }

        // FORPREP: init -= step, jump to FORLOOP
        let prep_pc = fs.emit_asbx(Opcode::ForPrep, base, 0);

        // Body — declare loop variable
        fs.declare_local(name, r_var);
        self.compile_block(fs, body)?;

        // Close upvalues for the loop variable before looping
        fs.emit_abc(Opcode::Close, r_var, 0, 0);

        // FORLOOP: step, test, jump back to body start
        let loop_pc = fs.current_pc();
        fs.emit_asbx(Opcode::ForLoop, base, 0);

        // Patch FORPREP to jump to FORLOOP
        let prep_offset = loop_pc as i32 - prep_pc as i32 - 1;
        fs.proto.code[prep_pc] = Instruction::encode_asbx(Opcode::ForPrep, base, prep_offset);

        // Patch FORLOOP to jump back to body start (prep_pc + 1)
        let body_start = prep_pc + 1;
        let loop_offset = body_start as i32 - loop_pc as i32 - 1;
        fs.proto.code[loop_pc] = Instruction::encode_asbx(Opcode::ForLoop, base, loop_offset);

        // Patch breaks
        let breaks = fs.break_lists.pop().unwrap();
        for brk in breaks {
            fs.patch_jump(brk);
        }

        fs.leave_scope();
        Ok(())
    }

    fn compile_generic_for(
        &mut self,
        fs: &mut FuncState,
        names: &[String],
        iterators: &[Expr],
        body: &Block,
    ) -> Result<(), CompileError> {
        fs.enter_scope();
        fs.break_lists.push(Vec::new());

        let base = fs.free_reg;

        // Reserve 3 internal slots: iterator function, state, control variable
        let r_iter = fs.alloc_reg()?;
        let _r_state = fs.alloc_reg()?;
        let _r_control = fs.alloc_reg()?;

        // Compile iterator expressions (typically: iterator, state, initial)
        // Need to adjust to exactly 3 values
        let save_free = fs.free_reg;
        fs.free_reg_to(r_iter);

        for (i, iter_expr) in iterators.iter().enumerate() {
            let is_last = i == iterators.len() - 1;
            if is_last && i < 2 {
                // Last expression, need multiple results
                match iter_expr {
                    Expr::FunctionCall(call) => {
                        let dest = r_iter + i as u8;
                        let needed = 3 - i as u16;
                        self.compile_call_expr(fs, &call.callee, &call.args, dest, needed + 1)?;
                        fs.free_reg_to(r_iter + 3);
                        break;
                    }
                    _ => {
                        let reg = fs.alloc_reg()?;
                        self.compile_expr_to_reg(fs, iter_expr, reg)?;
                    }
                }
            } else if i < 3 {
                let reg = fs.alloc_reg()?;
                self.compile_expr_to_reg(fs, iter_expr, reg)?;
            }
            // ignore excess iterator expressions
        }

        // Fill remaining internal slots with nil
        let filled = (fs.free_reg - r_iter) as usize;
        if filled < 3 {
            for _ in filled..3 {
                let reg = fs.alloc_reg()?;
                fs.emit_abc(Opcode::LoadNil, reg, 0, 0);
            }
        }
        fs.free_reg_to(r_iter + 3);
        let _ = save_free;

        // Reserve slots for loop variables
        let num_vars = names.len() as u8;
        for name in names {
            fs.add_local(name)?;
        }

        // Jump to TFORLOOP
        let prep_jmp = fs.emit_jump();
        let body_start = fs.current_pc();

        // Body
        self.compile_block(fs, body)?;

        // TFORLOOP: call iterator, assign results
        fs.patch_jump(prep_jmp);
        fs.emit_abc(Opcode::TForLoop, base, 0, num_vars as u16);

        // If first result is not nil, jump back to body
        let loop_jmp = fs.current_pc();
        fs.emit_asbx(Opcode::Jmp, 0, 0);
        fs.patch_jump_to(loop_jmp, body_start);

        // Patch breaks
        let breaks = fs.break_lists.pop().unwrap();
        for brk in breaks {
            fs.patch_jump(brk);
        }

        fs.leave_scope();
        Ok(())
    }

    fn compile_local_function(
        &mut self,
        fs: &mut FuncState,
        name: &str,
        params: &[String],
        is_vararg: bool,
        body: &Block,
    ) -> Result<(), CompileError> {
        // Declare local first (for self-recursion)
        let reg = fs.add_local(name)?;

        let child_idx = self.compile_child_function(fs, params, is_vararg, body)?;
        fs.emit_abx(Opcode::Closure, reg, child_idx as u32);
        Ok(())
    }

    fn compile_function_def(
        &mut self,
        fs: &mut FuncState,
        name: &FuncName,
        params: &[String],
        is_vararg: bool,
        body: &Block,
    ) -> Result<(), CompileError> {
        // For method definitions, add implicit "self" parameter
        let (actual_params, actual_vararg) = if name.method.is_some() {
            let mut p = vec!["self".to_owned()];
            p.extend(params.iter().cloned());
            (p, is_vararg)
        } else {
            (params.to_vec(), is_vararg)
        };

        let child_idx = self.compile_child_function(fs, &actual_params, actual_vararg, body)?;

        // Assign to the name path
        let reg = fs.alloc_reg()?;
        fs.emit_abx(Opcode::Closure, reg, child_idx as u32);

        if name.parts.len() == 1 && name.method.is_none() {
            // Simple: function foo() ... end -> SetGlobal or local
            let func_name = &name.parts[0];
            if let Some(local_reg) = fs.resolve_local(func_name) {
                fs.emit_abc(Opcode::Move, local_reg, reg as u16, 0);
            } else {
                let k = fs.add_string_constant(func_name)?;
                fs.emit_abx(Opcode::SetGlobal, reg, k);
            }
        } else {
            // Dotted path: function a.b.c() or method: function a.b:c()
            // Get the base object
            let base_name = &name.parts[0];
            let obj_reg = fs.alloc_reg()?;
            if let Some(local_reg) = fs.resolve_local(base_name) {
                fs.emit_abc(Opcode::Move, obj_reg, local_reg as u16, 0);
            } else {
                let k = fs.add_string_constant(base_name)?;
                fs.emit_abx(Opcode::GetGlobal, obj_reg, k);
            }

            // Navigate intermediate parts (all but last for non-method, all for method)
            let (nav_parts, set_name) = if let Some(ref method) = name.method {
                // function a.b:c() → navigate a.b, set "c"
                (&name.parts[1..], method.as_str())
            } else {
                // function a.b.c() → navigate a.b (parts[1..n-1]), set parts[n-1]
                let last = name.parts.len() - 1;
                (&name.parts[1..last], name.parts[last].as_str())
            };

            for part in nav_parts {
                let key_rk = fs.constant_rk(LuaValue::String(Rc::new(part.clone())))?;
                fs.emit_abc(Opcode::GetTable, obj_reg, obj_reg as u16, key_rk);
            }

            let key_rk = fs.constant_rk(LuaValue::String(Rc::new(set_name.to_owned())))?;
            fs.emit_abc(Opcode::SetTable, obj_reg, key_rk, reg as u16);

            fs.free_reg_to(obj_reg);
        }

        fs.free_reg_to(reg);
        Ok(())
    }

    fn compile_break(&mut self, fs: &mut FuncState) -> Result<(), CompileError> {
        if fs.break_lists.is_empty() {
            return Err(CompileError::BreakOutsideLoop);
        }
        let jmp = fs.emit_jump();
        fs.break_lists.last_mut().unwrap().push(jmp);
        Ok(())
    }

    fn compile_return(&mut self, fs: &mut FuncState, exprs: &[Expr]) -> Result<(), CompileError> {
        if exprs.is_empty() {
            fs.emit_abc(Opcode::Return, 0, 1, 0);
            return Ok(());
        }

        let base = fs.free_reg;

        if exprs.len() == 1 {
            // Single return value — check for tail call
            if let Expr::FunctionCall(call) = &exprs[0] {
                // Compile as tail call
                self.compile_call_expr(fs, &call.callee, &call.args, base, 0)?;
                // Convert the CALL to TAILCALL
                let last_pc = fs.proto.code.len() - 1;
                let instr = fs.proto.code[last_pc];
                if instr.opcode() == Opcode::Call {
                    fs.proto.code[last_pc] =
                        Instruction::encode_abc(Opcode::TailCall, instr.a(), instr.b(), 0);
                }
                fs.emit_abc(Opcode::Return, base, 0, 0); // 0 = variable returns
                fs.free_reg_to(base);
                return Ok(());
            }
        }

        // Compile return expressions
        let mut var_results = false;
        for (i, expr) in exprs.iter().enumerate() {
            let is_last = i == exprs.len() - 1;
            if is_last {
                match expr {
                    Expr::FunctionCall(call) => {
                        let dest = base + i as u8;
                        self.compile_call_expr(fs, &call.callee, &call.args, dest, 0)?;
                        var_results = true;
                    }
                    Expr::MethodCall {
                        object,
                        method,
                        args,
                    } => {
                        let dest = base + i as u8;
                        self.compile_method_call_expr(fs, object, method, args, dest, 0)?;
                        var_results = true;
                    }
                    Expr::Vararg => {
                        if !fs.is_vararg {
                            return Err(CompileError::VarargOutsideVarargFunc);
                        }
                        let dest = base + i as u8;
                        fs.emit_abc(Opcode::Vararg, dest, 0, 0); // 0 = all varargs
                        var_results = true;
                    }
                    _ => {
                        let reg = fs.alloc_reg()?;
                        self.compile_expr_to_reg(fs, expr, reg)?;
                    }
                }
            } else {
                let reg = fs.alloc_reg()?;
                self.compile_expr_to_reg(fs, expr, reg)?;
            }
        }

        if var_results {
            fs.emit_abc(Opcode::Return, base, 0, 0); // 0 = variable
        } else {
            let count = (fs.free_reg - base) as u16 + 1;
            fs.emit_abc(Opcode::Return, base, count, 0);
        }
        fs.free_reg_to(base);
        Ok(())
    }

    // -- Child function compilation --

    fn compile_child_function(
        &mut self,
        parent: &mut FuncState,
        params: &[String],
        is_vararg: bool,
        body: &Block,
    ) -> Result<usize, CompileError> {
        let mut child_fs = FuncState::new(self.source, is_vararg, params.len() as u8);

        // Pass parent's locals as enclosing locals for upvalue resolution
        child_fs.enclosing_locals = parent
            .locals
            .iter()
            .map(|l| (l.name.clone(), l.register))
            .collect();
        child_fs.enclosing_upvalues = parent
            .upvalues
            .iter()
            .enumerate()
            .map(|(i, uv)| (uv.name.clone(), i as u8))
            .collect();

        // Also pass the grandparent context so the child can find variables
        // from any ancestor. These are stored separately and will trigger
        // upvalue creation in the parent during post-processing.
        child_fs.grandparent_locals = parent.enclosing_locals.clone();
        child_fs.grandparent_upvalues = parent.enclosing_upvalues.clone();

        child_fs.enter_scope();

        for param in params {
            child_fs.add_local(param)?;
        }

        self.compile_block(&mut child_fs, body)?;

        child_fs.emit_abc(Opcode::Return, 0, 1, 0);
        child_fs.leave_scope();

        // Post-process: for upvalues the child found in the grandparent context,
        // ensure the parent has them as upvalues too.
        for uv in &child_fs.upvalues {
            if !uv.in_stack && (uv.index as usize) >= parent.upvalues.len() {
                // This upvalue was resolved from grandparent context.
                // Force the parent to resolve it from its own enclosing context.
                parent.resolve_upvalue(&uv.name);
            }
        }

        // Re-map child upvalue indices now that parent upvalues may have changed
        let parent_upval_map: std::collections::HashMap<String, u8> = parent
            .upvalues
            .iter()
            .enumerate()
            .map(|(i, uv)| (uv.name.clone(), i as u8))
            .collect();

        for uv in &mut child_fs.upvalues {
            if !uv.in_stack
                && let Some(&parent_idx) = parent_upval_map.get(&uv.name)
            {
                uv.index = parent_idx;
            }
        }

        let proto = child_fs.finalize();
        let idx = parent.proto.prototypes.len();
        parent.proto.prototypes.push(proto);
        Ok(idx)
    }

    // -- Expression compilation --

    fn compile_expr_to_reg(
        &mut self,
        fs: &mut FuncState,
        expr: &Expr,
        dest: u8,
    ) -> Result<(), CompileError> {
        match expr {
            Expr::Nil => {
                fs.emit_abc(Opcode::LoadNil, dest, 0, 0);
            }
            Expr::True => {
                fs.emit_abc(Opcode::LoadBool, dest, 1, 0);
            }
            Expr::False => {
                fs.emit_abc(Opcode::LoadBool, dest, 0, 0);
            }
            Expr::Number(n) => {
                let k = fs.add_number_constant(*n)?;
                fs.emit_abx(Opcode::LoadK, dest, k);
            }
            Expr::StringLit(s) => {
                let k = fs.add_string_constant(s)?;
                fs.emit_abx(Opcode::LoadK, dest, k);
            }
            Expr::Name(name) => {
                if let Some(local_reg) = fs.resolve_local(name) {
                    if local_reg != dest {
                        fs.emit_abc(Opcode::Move, dest, local_reg as u16, 0);
                    }
                } else if let Some(uv_idx) = fs.resolve_upvalue(name) {
                    fs.emit_abc(Opcode::GetUpval, dest, uv_idx as u16, 0);
                } else {
                    let k = fs.add_string_constant(name)?;
                    fs.emit_abx(Opcode::GetGlobal, dest, k);
                }
            }
            Expr::Index { table, key } => {
                let table_reg = fs.alloc_reg()?;
                self.compile_expr_to_reg(fs, table, table_reg)?;
                let key_rk = self.expr_to_rk(fs, key)?;
                fs.emit_abc(Opcode::GetTable, dest, table_reg as u16, key_rk);
                fs.free_reg_to(table_reg);
            }
            Expr::Field { table, name } => {
                let table_reg = fs.alloc_reg()?;
                self.compile_expr_to_reg(fs, table, table_reg)?;
                let key_rk = fs.constant_rk(LuaValue::String(Rc::new(name.clone())))?;
                fs.emit_abc(Opcode::GetTable, dest, table_reg as u16, key_rk);
                fs.free_reg_to(table_reg);
            }
            Expr::BinOp { op, left, right } => {
                self.compile_binop(fs, *op, left, right, dest)?;
            }
            Expr::UnOp { op, operand } => {
                self.compile_unop(fs, *op, operand, dest)?;
            }
            Expr::FunctionCall(call) => {
                self.compile_call_expr(fs, &call.callee, &call.args, dest, 2)?; // C=2: 1 result
                fs.free_reg_to(dest + 1);
            }
            Expr::MethodCall {
                object,
                method,
                args,
            } => {
                self.compile_method_call_expr(fs, object, method, args, dest, 2)?; // C=2: 1 result
                fs.free_reg_to(dest + 1);
            }
            Expr::FunctionDef {
                params,
                is_vararg,
                body,
            } => {
                let child_idx = self.compile_child_function(fs, params, *is_vararg, body)?;
                fs.emit_abx(Opcode::Closure, dest, child_idx as u32);
            }
            Expr::TableConstructor { fields } => {
                self.compile_table_constructor(fs, fields, dest)?;
            }
            Expr::Vararg => {
                if !fs.is_vararg {
                    return Err(CompileError::VarargOutsideVarargFunc);
                }
                fs.emit_abc(Opcode::Vararg, dest, 2, 0); // 2 = 1 result
            }
        }
        Ok(())
    }

    fn compile_binop(
        &mut self,
        fs: &mut FuncState,
        op: BinOp,
        left: &Expr,
        right: &Expr,
        dest: u8,
    ) -> Result<(), CompileError> {
        match op {
            BinOp::And => {
                // Short-circuit: if left is falsy, result is left; otherwise result is right
                self.compile_expr_to_reg(fs, left, dest)?;
                fs.emit_abc(Opcode::TestSet, dest, dest as u16, 0); // if false, keep dest; else skip
                let skip_jmp = fs.emit_jump();
                self.compile_expr_to_reg(fs, right, dest)?;
                fs.patch_jump(skip_jmp);
            }
            BinOp::Or => {
                // Short-circuit: if left is truthy, result is left; otherwise result is right
                self.compile_expr_to_reg(fs, left, dest)?;
                fs.emit_abc(Opcode::TestSet, dest, dest as u16, 1); // if true, keep dest; else skip
                let skip_jmp = fs.emit_jump();
                self.compile_expr_to_reg(fs, right, dest)?;
                fs.patch_jump(skip_jmp);
            }
            BinOp::Concat => {
                // CONCAT A B C: R(A) = R(B) .. ... .. R(C)
                let base = fs.alloc_reg()?;
                self.compile_expr_to_reg(fs, left, base)?;
                let right_reg = fs.alloc_reg()?;
                self.compile_expr_to_reg(fs, right, right_reg)?;
                fs.emit_abc(Opcode::Concat, dest, base as u16, right_reg as u16);
                fs.free_reg_to(base);
            }
            BinOp::Eq | BinOp::Neq | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                self.compile_comparison(fs, op, left, right, dest)?;
            }
            _ => {
                // Arithmetic: ADD, SUB, MUL, DIV, MOD, POW
                let save = fs.free_reg;
                let lhs_rk = self.expr_to_rk(fs, left)?;
                let rhs_rk = self.expr_to_rk(fs, right)?;
                let opcode = match op {
                    BinOp::Add => Opcode::Add,
                    BinOp::Sub => Opcode::Sub,
                    BinOp::Mul => Opcode::Mul,
                    BinOp::Div => Opcode::Div,
                    BinOp::Mod => Opcode::Mod,
                    BinOp::Pow => Opcode::Pow,
                    _ => unreachable!(),
                };
                fs.emit_abc(opcode, dest, lhs_rk, rhs_rk);
                fs.free_reg_to(save);
            }
        }
        Ok(())
    }

    fn compile_comparison(
        &mut self,
        fs: &mut FuncState,
        op: BinOp,
        left: &Expr,
        right: &Expr,
        dest: u8,
    ) -> Result<(), CompileError> {
        let (opcode, a_val, lhs, rhs) = match op {
            BinOp::Eq => (Opcode::Eq, 1u8, left, right),
            BinOp::Neq => (Opcode::Eq, 0, left, right),
            BinOp::Lt => (Opcode::Lt, 1, left, right),
            BinOp::Le => (Opcode::Le, 1, left, right),
            BinOp::Gt => (Opcode::Lt, 1, right, left), // a > b == b < a
            BinOp::Ge => (Opcode::Le, 1, right, left), // a >= b == b <= a
            _ => unreachable!(),
        };

        let save = fs.free_reg;
        let lhs_rk = self.expr_to_rk(fs, lhs)?;
        let rhs_rk = self.expr_to_rk(fs, rhs)?;

        // EQ/LT/LE A B C: if result == A then skip next (JMP)
        fs.emit_abc(opcode, a_val, lhs_rk, rhs_rk);
        fs.free_reg_to(save);
        let false_jmp = fs.emit_jump(); // when comparison fails, jump to load false
        fs.emit_abc(Opcode::LoadBool, dest, 1, 1); // load true, skip next
        fs.patch_jump(false_jmp); // false_jmp targets the LoadBool false below
        fs.emit_abc(Opcode::LoadBool, dest, 0, 0); // load false

        Ok(())
    }

    fn compile_unop(
        &mut self,
        fs: &mut FuncState,
        op: UnOp,
        operand: &Expr,
        dest: u8,
    ) -> Result<(), CompileError> {
        let src = fs.alloc_reg()?;
        self.compile_expr_to_reg(fs, operand, src)?;
        let opcode = match op {
            UnOp::Neg => Opcode::Unm,
            UnOp::Not => Opcode::Not,
            UnOp::Len => Opcode::Len,
        };
        fs.emit_abc(opcode, dest, src as u16, 0);
        fs.free_reg_to(src);
        Ok(())
    }

    fn compile_call_expr(
        &mut self,
        fs: &mut FuncState,
        callee: &Expr,
        args: &[Expr],
        dest: u8,
        num_results: u16,
    ) -> Result<(), CompileError> {
        let save = fs.free_reg;
        fs.free_reg_to(dest);

        let func_reg = fs.alloc_reg()?;
        self.compile_expr_to_reg(fs, callee, func_reg)?;

        // Compile arguments
        let mut var_args = false;
        for (i, arg) in args.iter().enumerate() {
            let is_last = i == args.len() - 1;
            if is_last {
                match arg {
                    Expr::FunctionCall(call) => {
                        let arg_dest = fs.alloc_reg()?;
                        self.compile_call_expr(fs, &call.callee, &call.args, arg_dest, 0)?;
                        var_args = true;
                        continue;
                    }
                    Expr::MethodCall {
                        object,
                        method,
                        args: margs,
                    } => {
                        let arg_dest = fs.alloc_reg()?;
                        self.compile_method_call_expr(fs, object, method, margs, arg_dest, 0)?;
                        var_args = true;
                        continue;
                    }
                    Expr::Vararg => {
                        if !fs.is_vararg {
                            return Err(CompileError::VarargOutsideVarargFunc);
                        }
                        let arg_dest = fs.alloc_reg()?;
                        fs.emit_abc(Opcode::Vararg, arg_dest, 0, 0);
                        var_args = true;
                        continue;
                    }
                    _ => {}
                }
            }
            let reg = fs.alloc_reg()?;
            self.compile_expr_to_reg(fs, arg, reg)?;
        }

        let b = if var_args {
            0u16
        } else {
            args.len() as u16 + 1
        };
        // CALL A B C encoding:
        //   B = num_args + 1 (0 = variable args)
        //   C = num_results + 1 (0 = variable results)
        // Our convention: num_results is the raw C value passed by callers.
        //   0 = variable results (C=0), 1 = 0 results (C=1), 2 = 1 result (C=2), etc.
        fs.emit_abc(Opcode::Call, func_reg, b, num_results);

        // Restore free_reg
        if num_results == 0 {
            // Variable results — caller manages free_reg
        } else {
            let results = num_results.saturating_sub(1);
            fs.free_reg_to(save.max(dest + results as u8));
        }
        Ok(())
    }

    fn compile_method_call_expr(
        &mut self,
        fs: &mut FuncState,
        object: &Expr,
        method: &str,
        args: &[Expr],
        dest: u8,
        num_results: u16,
    ) -> Result<(), CompileError> {
        let save = fs.free_reg;
        fs.free_reg_to(dest);

        // SELF A B C: R(A+1) = R(B); R(A) = R(B)[RK(C)]
        let func_reg = fs.alloc_reg()?; // A
        let self_reg = fs.alloc_reg()?; // A+1

        self.compile_expr_to_reg(fs, object, self_reg)?;
        let method_rk = fs.constant_rk(LuaValue::String(Rc::new(method.to_owned())))?;
        fs.emit_abc(Opcode::OpSelf, func_reg, self_reg as u16, method_rk);

        // Compile arguments (after self)
        let mut var_args = false;
        for (i, arg) in args.iter().enumerate() {
            let is_last = i == args.len() - 1;
            if is_last {
                match arg {
                    Expr::FunctionCall(call) => {
                        let arg_dest = fs.alloc_reg()?;
                        self.compile_call_expr(fs, &call.callee, &call.args, arg_dest, 0)?;
                        var_args = true;
                        continue;
                    }
                    Expr::MethodCall {
                        object,
                        method,
                        args: margs,
                    } => {
                        let arg_dest = fs.alloc_reg()?;
                        self.compile_method_call_expr(fs, object, method, margs, arg_dest, 0)?;
                        var_args = true;
                        continue;
                    }
                    Expr::Vararg => {
                        if !fs.is_vararg {
                            return Err(CompileError::VarargOutsideVarargFunc);
                        }
                        let arg_dest = fs.alloc_reg()?;
                        fs.emit_abc(Opcode::Vararg, arg_dest, 0, 0);
                        var_args = true;
                        continue;
                    }
                    _ => {}
                }
            }
            let reg = fs.alloc_reg()?;
            self.compile_expr_to_reg(fs, arg, reg)?;
        }

        let b = if var_args {
            0u16
        } else {
            args.len() as u16 + 2 // +1 for self, +1 for encoding
        };
        // num_results is raw C value (same convention as compile_call_expr)
        fs.emit_abc(Opcode::Call, func_reg, b, num_results);

        if num_results == 0 {
            // Variable results — caller manages free_reg
        } else {
            let results = num_results.saturating_sub(1);
            fs.free_reg_to(save.max(dest + results as u8));
        }
        Ok(())
    }

    fn compile_table_constructor(
        &mut self,
        fs: &mut FuncState,
        fields: &[TableField],
        dest: u8,
    ) -> Result<(), CompileError> {
        // Count array and hash parts
        let mut array_count = 0u32;
        let mut hash_count = 0u32;
        for field in fields {
            match field {
                TableField::PositionalField { .. } => array_count += 1,
                _ => hash_count += 1,
            }
        }

        // NEWTABLE A B C: B = array size hint (FPF encoded), C = hash size hint
        let b_hint = float_byte(array_count);
        let c_hint = float_byte(hash_count);
        fs.emit_abc(Opcode::NewTable, dest, b_hint as u16, c_hint as u16);

        let mut array_idx = 0u32;
        let fields_per_flush = 50u32; // LFIELDS_PER_FLUSH

        let mut last_is_varresult = false;
        for (fi, field) in fields.iter().enumerate() {
            let is_last = fi == fields.len() - 1;
            match field {
                TableField::PositionalField { value } => {
                    array_idx += 1;
                    let val_reg = fs.alloc_reg()?;

                    // If this is the last positional field and it's a multi-result
                    // expression, compile with variable results
                    if is_last {
                        match value {
                            Expr::FunctionCall(call) => {
                                self.compile_call_expr(fs, &call.callee, &call.args, val_reg, 0)?;
                                last_is_varresult = true;
                            }
                            Expr::MethodCall {
                                object,
                                method,
                                args,
                            } => {
                                self.compile_method_call_expr(
                                    fs, object, method, args, val_reg, 0,
                                )?;
                                last_is_varresult = true;
                            }
                            Expr::Vararg => {
                                if !fs.is_vararg {
                                    return Err(CompileError::VarargOutsideVarargFunc);
                                }
                                fs.emit_abc(Opcode::Vararg, val_reg, 0, 0); // 0 = all
                                last_is_varresult = true;
                            }
                            _ => {
                                self.compile_expr_to_reg(fs, value, val_reg)?;
                            }
                        }
                    } else {
                        self.compile_expr_to_reg(fs, value, val_reg)?;
                    }

                    if array_idx.is_multiple_of(fields_per_flush) && !last_is_varresult {
                        // Flush
                        let count = fields_per_flush;
                        let batch = (array_idx / fields_per_flush) as u16;
                        fs.emit_abc(Opcode::SetList, dest, count as u16, batch);
                        fs.free_reg_to(dest + 1);
                    }
                }
                TableField::NamedField { name, value } => {
                    let save = fs.free_reg;
                    let key_rk = fs.constant_rk(LuaValue::String(Rc::new(name.clone())))?;
                    let val_rk = self.expr_to_rk(fs, value)?;
                    fs.emit_abc(Opcode::SetTable, dest, key_rk, val_rk);
                    fs.free_reg_to(save);
                }
                TableField::IndexedField { key, value } => {
                    let save = fs.free_reg;
                    let key_rk = self.expr_to_rk(fs, key)?;
                    let val_rk = self.expr_to_rk(fs, value)?;
                    fs.emit_abc(Opcode::SetTable, dest, key_rk, val_rk);
                    fs.free_reg_to(save);
                }
            }
        }

        // Final flush for remaining array items
        if last_is_varresult {
            // B=0 means "up to top of stack"
            let batch = (array_idx / fields_per_flush) as u16 + 1;
            fs.emit_abc(Opcode::SetList, dest, 0, batch);
        } else {
            let remaining = array_idx % fields_per_flush;
            if remaining > 0 || (array_idx > 0 && array_idx <= fields_per_flush) {
                let count = if array_idx <= fields_per_flush {
                    array_idx
                } else {
                    remaining
                };
                let batch = (array_idx / fields_per_flush) as u16 + 1;
                if count > 0 {
                    fs.emit_abc(Opcode::SetList, dest, count as u16, batch);
                }
            }
        }

        fs.free_reg_to(dest + 1);
        Ok(())
    }

    /// Try to encode an expression as an RK value (constant or register).
    fn expr_to_rk(&mut self, fs: &mut FuncState, expr: &Expr) -> Result<u16, CompileError> {
        match expr {
            Expr::Number(n) => {
                let k = fs.add_number_constant(*n)?;
                if k < RK_OFFSET as u32 {
                    return Ok(Instruction::rk_constant(k as u16));
                }
            }
            Expr::StringLit(s) => {
                let k = fs.add_string_constant(s)?;
                if k < RK_OFFSET as u32 {
                    return Ok(Instruction::rk_constant(k as u16));
                }
            }
            Expr::True => {
                let k = fs.add_constant(LuaValue::Boolean(true))?;
                if k < RK_OFFSET as u32 {
                    return Ok(Instruction::rk_constant(k as u16));
                }
            }
            Expr::False => {
                let k = fs.add_constant(LuaValue::Boolean(false))?;
                if k < RK_OFFSET as u32 {
                    return Ok(Instruction::rk_constant(k as u16));
                }
            }
            Expr::Nil => {
                let k = fs.add_constant(LuaValue::Nil)?;
                if k < RK_OFFSET as u32 {
                    return Ok(Instruction::rk_constant(k as u16));
                }
            }
            Expr::Name(name) => {
                if let Some(local_reg) = fs.resolve_local(name) {
                    return Ok(local_reg as u16);
                }
            }
            _ => {}
        }
        // Fall back: compile to a temp register
        let reg = fs.alloc_reg()?;
        self.compile_expr_to_reg(fs, expr, reg)?;
        Ok(reg as u16)
    }
}

// Lua 5.1 "floating point byte" encoding for table size hints
fn float_byte(x: u32) -> u8 {
    if x == 0 {
        return 0;
    }
    if x <= 8 {
        return x as u8;
    }
    let mut e = 0u32;
    let mut m = x;
    while m >= 16 {
        m = (m + 1) >> 1;
        e += 1;
    }
    ((e + 1) << 3 | (m - 8)) as u8
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rlua_core::opcode::Opcode;

    fn compile(src: &str) -> FunctionProto {
        let block = rlua_parser::parse(src).expect("parse failed");
        let mut compiler = Compiler::new(src);
        compiler.compile_main(&block).expect("compile failed")
    }

    fn opcodes(proto: &FunctionProto) -> Vec<Opcode> {
        proto.code.iter().map(|i| i.opcode()).collect()
    }

    #[test]
    fn compile_empty() {
        let proto = compile("");
        assert_eq!(opcodes(&proto), vec![Opcode::Return]);
    }

    #[test]
    fn compile_return_number() {
        let proto = compile("return 42");
        assert_eq!(proto.constants.len(), 1);
        assert_eq!(proto.constants[0], LuaValue::Number(42.0));
        assert!(opcodes(&proto).contains(&Opcode::LoadK));
        assert!(opcodes(&proto).contains(&Opcode::Return));
    }

    #[test]
    fn compile_local_assignment() {
        let proto = compile("local x = 1");
        let ops = opcodes(&proto);
        assert!(ops.contains(&Opcode::LoadK));
        assert!(ops.contains(&Opcode::Return));
    }

    #[test]
    fn compile_arithmetic() {
        let proto = compile("local x = 1 + 2");
        let ops = opcodes(&proto);
        assert!(ops.contains(&Opcode::Add));
    }

    #[test]
    fn compile_global_assign() {
        let proto = compile("x = 42");
        let ops = opcodes(&proto);
        assert!(ops.contains(&Opcode::SetGlobal));
    }

    #[test]
    fn compile_if_statement() {
        let proto = compile("if true then local x = 1 end");
        let ops = opcodes(&proto);
        assert!(ops.contains(&Opcode::LoadBool));
        assert!(ops.contains(&Opcode::Test));
        assert!(ops.contains(&Opcode::Jmp));
    }

    #[test]
    fn compile_while_loop() {
        let proto = compile("local i = 0 while i do break end");
        let ops = opcodes(&proto);
        assert!(ops.contains(&Opcode::Test));
        assert!(ops.contains(&Opcode::Jmp));
    }

    #[test]
    fn compile_numeric_for() {
        let proto = compile("for i = 1, 10 do end");
        let ops = opcodes(&proto);
        assert!(ops.contains(&Opcode::ForPrep));
        assert!(ops.contains(&Opcode::ForLoop));
    }

    #[test]
    fn compile_function_call() {
        let proto = compile("print(1)");
        let ops = opcodes(&proto);
        assert!(ops.contains(&Opcode::GetGlobal));
        assert!(ops.contains(&Opcode::Call));
    }

    #[test]
    fn compile_table_constructor() {
        let proto = compile("local t = {1, 2, 3}");
        let ops = opcodes(&proto);
        assert!(ops.contains(&Opcode::NewTable));
        assert!(ops.contains(&Opcode::SetList));
    }

    #[test]
    fn compile_function_def() {
        let proto = compile("local function f(a, b) return a + b end");
        let ops = opcodes(&proto);
        assert!(ops.contains(&Opcode::Closure));
        assert_eq!(proto.prototypes.len(), 1);
        let child = &proto.prototypes[0];
        assert_eq!(child.num_params, 2);
        let child_ops: Vec<_> = child.code.iter().map(|i| i.opcode()).collect();
        assert!(child_ops.contains(&Opcode::Add));
        assert!(child_ops.contains(&Opcode::Return));
    }

    #[test]
    fn compile_string_concat() {
        let proto = compile("local x = 'a' .. 'b'");
        let ops = opcodes(&proto);
        assert!(ops.contains(&Opcode::Concat));
    }

    #[test]
    fn compile_comparison() {
        let proto = compile("local x = 1 < 2");
        let ops = opcodes(&proto);
        assert!(ops.contains(&Opcode::Lt));
    }

    #[test]
    fn compile_unary() {
        let proto = compile("local x = -1");
        let ops = opcodes(&proto);
        assert!(ops.contains(&Opcode::Unm));
    }

    #[test]
    fn compile_not() {
        let proto = compile("local x = not true");
        let ops = opcodes(&proto);
        assert!(ops.contains(&Opcode::Not));
    }

    #[test]
    fn compile_multiple_return() {
        let proto = compile("return 1, 2, 3");
        let ops = opcodes(&proto);
        assert_eq!(ops.iter().filter(|&&o| o == Opcode::LoadK).count(), 3);
        assert!(ops.contains(&Opcode::Return));
    }

    #[test]
    fn compile_break_outside_loop_errors() {
        let block = rlua_parser::parse("break").unwrap();
        let mut compiler = Compiler::new("break");
        let result = compiler.compile_main(&block);
        assert!(matches!(result, Err(CompileError::BreakOutsideLoop)));
    }

    #[test]
    fn constant_dedup() {
        let proto = compile("local a = 42; local b = 42");
        // 42 should appear only once in constants
        let num_count = proto
            .constants
            .iter()
            .filter(|c| matches!(c, LuaValue::Number(n) if *n == 42.0))
            .count();
        assert_eq!(num_count, 1);
    }
}
