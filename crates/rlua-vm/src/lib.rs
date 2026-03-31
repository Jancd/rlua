use std::cell::RefCell;
use std::collections::HashMap;
use std::mem;
use std::rc::Rc;

use rlua_core::bytecode::RK_OFFSET;
use rlua_core::function::{
    CallOutcome, Closure, FunctionProto, LuaFunction, NativeFn, NativeVmContext, UpvalRef,
};
use rlua_core::gc::{GcRoot, GcRootProvider, MarkSweepGc, RootSource};
use rlua_core::opcode::Opcode;
use rlua_core::table::{LuaTable, TableRef};
use rlua_core::value::{LuaThread, LuaValue, ThreadRef};
#[cfg(test)]
use rlua_ir::Trace;
use rlua_ir::{IrOp, TraceDeoptExit, TraceDeoptExitKind, ValueType};
use rlua_jit::{
    ExecutionMode, JitAvailability, JitConfig, JitRuntime, JitStats, LoopTraceRecorder,
    NativeTraceOutcome, RecordingRequest, TraceCacheDebugEntry, TraceKey, TraceLifecycleState,
};

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

#[derive(Debug, Clone)]
struct PendingResume {
    target: ResumeTarget,
}

#[derive(Debug, Clone)]
enum ResumeTarget {
    Call {
        base: usize,
        register: u8,
        num_results_wanted: i32,
    },
    TailReturn,
    EntryReturn,
}

#[derive(Debug, Clone)]
struct ExecutionContext {
    stack: Vec<LuaValue>,
    frames: Vec<CallFrame>,
    open_upvalues: HashMap<usize, UpvalRef>,
    pending_side_exit: Option<TraceKey>,
    pending_resume: Option<PendingResume>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CoroutineStatus {
    Suspended,
    Running,
    Normal,
    Dead,
}

#[derive(Debug, Clone)]
struct CoroutineState {
    thread: ThreadRef,
    context: ExecutionContext,
    entry: Option<LuaValue>,
    status: CoroutineStatus,
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
    /// JIT runtime and cache management.
    jit: JitRuntime,
    /// Per-loop execution counters keyed by function and loop header pc.
    loop_hotness: HashMap<TraceKey, u32>,
    /// Prevent immediate re-entry into a trace right after a side exit.
    pending_side_exit: Option<TraceKey>,
    /// Result placement for a suspended coroutine.yield call.
    pending_resume: Option<PendingResume>,
    /// Suspended or non-running coroutine contexts keyed by thread identity.
    coroutines: HashMap<usize, CoroutineState>,
    /// The currently executing coroutine. `None` means the main thread.
    current_thread: Option<ThreadRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HotLoopCounter {
    pub function: usize,
    pub loop_header_pc: usize,
    pub hits: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VmJitDebugState {
    pub execution_mode: ExecutionMode,
    pub availability: JitAvailability,
    pub config: JitConfig,
    pub counters: Vec<HotLoopCounter>,
    pub stats: JitStats,
    pub trace_count: usize,
    pub traces: Vec<TraceCacheDebugEntry>,
}

impl VmState {
    pub fn new() -> Self {
        Self::with_jit_config(JitConfig::default())
    }

    pub fn with_jit_config(config: JitConfig) -> Self {
        Self {
            stack: Vec::with_capacity(256),
            frames: Vec::new(),
            globals: Rc::new(RefCell::new(LuaTable::new())),
            output: Vec::new(),
            open_upvalues: HashMap::new(),
            string_metatable: None,
            gc: MarkSweepGc::new(),
            jit: JitRuntime::new(config),
            loop_hotness: HashMap::new(),
            pending_side_exit: None,
            pending_resume: None,
            coroutines: HashMap::new(),
            current_thread: None,
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

    pub fn jit_debug_state(&self) -> VmJitDebugState {
        let mut counters: Vec<HotLoopCounter> = self
            .loop_hotness
            .iter()
            .map(|(key, hits)| HotLoopCounter {
                function: key.function,
                loop_header_pc: key.loop_header_pc,
                hits: *hits,
            })
            .collect();
        counters.sort_by_key(|counter| (counter.function, counter.loop_header_pc));

        VmJitDebugState {
            execution_mode: self.jit.execution_mode(),
            availability: self.jit.availability(),
            config: self.jit.config(),
            counters,
            stats: self.jit.stats(),
            trace_count: self.jit.trace_count(),
            traces: self.jit.trace_debug_entries(),
        }
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

    fn take_execution_context(&mut self) -> ExecutionContext {
        ExecutionContext {
            stack: mem::take(&mut self.stack),
            frames: mem::take(&mut self.frames),
            open_upvalues: mem::take(&mut self.open_upvalues),
            pending_side_exit: self.pending_side_exit.take(),
            pending_resume: self.pending_resume.take(),
        }
    }

    fn restore_execution_context(&mut self, context: ExecutionContext) {
        self.stack = context.stack;
        self.frames = context.frames;
        self.open_upvalues = context.open_upvalues;
        self.pending_side_exit = context.pending_side_exit;
        self.pending_resume = context.pending_resume;
    }

    fn thread_key(thread: &ThreadRef) -> usize {
        Rc::as_ptr(thread) as usize
    }

    fn create_coroutine_state(&mut self, func: LuaValue) -> Result<LuaValue, LuaError> {
        if !matches!(func, LuaValue::Function(_)) {
            return Err(LuaError::Type(format!(
                "bad argument #1 to 'create' (function expected, got {})",
                func.type_name()
            )));
        }

        let thread = Rc::new(LuaThread);
        let key = Self::thread_key(&thread);
        self.coroutines.insert(
            key,
            CoroutineState {
                thread: thread.clone(),
                context: ExecutionContext {
                    stack: Vec::with_capacity(256),
                    frames: Vec::new(),
                    open_upvalues: HashMap::new(),
                    pending_side_exit: None,
                    pending_resume: None,
                },
                entry: Some(func),
                status: CoroutineStatus::Suspended,
            },
        );
        Ok(LuaValue::Thread(thread))
    }

    fn coroutine_status_of(&self, thread: &ThreadRef) -> &'static str {
        let key = Self::thread_key(thread);
        if let Some(current) = &self.current_thread
            && Rc::ptr_eq(current, thread)
        {
            return "running";
        }

        self.coroutines
            .get(&key)
            .map(|state| match state.status {
                CoroutineStatus::Suspended => "suspended",
                CoroutineStatus::Running => "running",
                CoroutineStatus::Normal => "normal",
                CoroutineStatus::Dead => "dead",
            })
            .unwrap_or("dead")
    }

    fn cleanup_current_frame(&mut self) {
        let base = self
            .frames
            .last()
            .expect("frame cleanup requires an active call frame")
            .base;
        self.close_upvalues(base);
        self.stack.truncate(base);
        self.frames.pop();
    }

    fn cleanup_frames_to_depth(&mut self, depth: usize) {
        while self.frames.len() > depth {
            self.cleanup_current_frame();
        }
        self.pending_resume = None;
    }

    fn apply_pending_resume(
        &mut self,
        values: &[LuaValue],
    ) -> Result<Option<Vec<LuaValue>>, LuaError> {
        let Some(pending) = self.pending_resume.take() else {
            return Ok(None);
        };

        match pending.target {
            ResumeTarget::Call {
                base,
                register,
                num_results_wanted,
            } => {
                write_call_results(self, base, register, num_results_wanted, values);
                Ok(None)
            }
            ResumeTarget::TailReturn | ResumeTarget::EntryReturn => Ok(Some(values.to_vec())),
        }
    }

    fn continue_after_saved_return(
        &mut self,
        results: Vec<LuaValue>,
    ) -> Result<Option<CallOutcome>, LuaError> {
        loop {
            if !self.frames.is_empty() {
                self.cleanup_current_frame();
            }

            let Some(frame) = self.frames.last() else {
                return Ok(Some(CallOutcome::Return(results)));
            };

            let call_pc = frame
                .pc
                .checked_sub(1)
                .ok_or_else(|| LuaError::Runtime("coroutine resume lost caller pc".to_owned()))?;
            let instr = frame.closure.proto.code[call_pc];

            match instr.opcode() {
                Opcode::Call => {
                    let num_results_wanted = if instr.c() == 0 {
                        -1
                    } else {
                        (instr.c() - 1) as i32
                    };
                    write_call_results(self, frame.base, instr.a(), num_results_wanted, &results);
                    return Ok(None);
                }
                Opcode::TailCall => continue,
                opcode => {
                    return Err(LuaError::Runtime(format!(
                        "coroutine resume reached unsupported caller opcode {opcode:?}"
                    )));
                }
            }
        }
    }

    fn run_resumed_execution(&mut self, args: &[LuaValue]) -> Result<CallOutcome, LuaError> {
        if let Some(results) = self.apply_pending_resume(args)?
            && let Some(outcome) = self.continue_after_saved_return(results)?
        {
            return Ok(outcome);
        }

        loop {
            match run_loop_outcome(self)? {
                CallOutcome::Yield(values) => return Ok(CallOutcome::Yield(values)),
                CallOutcome::Return(values) => {
                    if let Some(outcome) = self.continue_after_saved_return(values)? {
                        return Ok(outcome);
                    }
                }
            }
        }
    }

    fn resume_coroutine_state(
        &mut self,
        thread: &ThreadRef,
        args: &[LuaValue],
    ) -> Result<Vec<LuaValue>, LuaError> {
        let key = Self::thread_key(thread);
        let Some(mut coroutine) = self.coroutines.remove(&key) else {
            return Ok(vec![
                LuaValue::Boolean(false),
                LuaValue::from("cannot resume dead coroutine"),
            ]);
        };

        match coroutine.status {
            CoroutineStatus::Dead => {
                self.coroutines.insert(key, coroutine);
                return Ok(vec![
                    LuaValue::Boolean(false),
                    LuaValue::from("cannot resume dead coroutine"),
                ]);
            }
            CoroutineStatus::Running | CoroutineStatus::Normal => {
                self.coroutines.insert(key, coroutine);
                return Ok(vec![
                    LuaValue::Boolean(false),
                    LuaValue::from("cannot resume non-suspended coroutine"),
                ]);
            }
            CoroutineStatus::Suspended => {}
        }

        let parent_thread = self.current_thread.clone();
        let parent_context = self.take_execution_context();
        if let Some(parent) = &parent_thread {
            let parent_key = Self::thread_key(parent);
            self.coroutines.insert(
                parent_key,
                CoroutineState {
                    thread: parent.clone(),
                    context: parent_context.clone(),
                    entry: None,
                    status: CoroutineStatus::Normal,
                },
            );
        }

        self.restore_execution_context(coroutine.context.clone());
        self.current_thread = Some(thread.clone());
        coroutine.status = CoroutineStatus::Running;

        let outcome = if let Some(entry) = coroutine.entry.take() {
            let outcome = call_function_outcome(self, &entry, args);
            if matches!(outcome, Ok(CallOutcome::Yield(_)))
                && self.frames.is_empty()
                && self.pending_resume.is_none()
            {
                self.pending_resume = Some(PendingResume {
                    target: ResumeTarget::EntryReturn,
                });
            }
            outcome
        } else {
            self.run_resumed_execution(args)
        };

        let active_context = self.take_execution_context();
        let mut resume_results = Vec::new();

        match outcome {
            Ok(CallOutcome::Return(values)) => {
                coroutine.context = active_context;
                coroutine.status = CoroutineStatus::Dead;
                resume_results.push(LuaValue::Boolean(true));
                resume_results.extend(values);
            }
            Ok(CallOutcome::Yield(values)) => {
                coroutine.context = active_context;
                coroutine.status = CoroutineStatus::Suspended;
                resume_results.push(LuaValue::Boolean(true));
                resume_results.extend(values);
            }
            Err(err) => {
                coroutine.context = active_context;
                coroutine.status = CoroutineStatus::Dead;
                resume_results.push(LuaValue::Boolean(false));
                resume_results.push(lua_error_to_value(err));
            }
        }

        if coroutine.status != CoroutineStatus::Dead {
            self.coroutines.insert(key, coroutine);
        }

        if let Some(parent) = parent_thread {
            let parent_key = Self::thread_key(&parent);
            let mut parent_state = self
                .coroutines
                .remove(&parent_key)
                .expect("parent coroutine context missing");
            parent_state.status = CoroutineStatus::Running;
            self.restore_execution_context(parent_state.context);
            self.current_thread = Some(parent);
        } else {
            self.restore_execution_context(parent_context);
            self.current_thread = None;
        }

        Ok(resume_results)
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

    fn slot_value_type(&self, abs_idx: usize) -> ValueType {
        if let Some(value) = self.open_upvalues.get(&abs_idx) {
            return value_type_of(&value.borrow());
        }
        value_type_of(&self.stack.get(abs_idx).cloned().unwrap_or(LuaValue::Nil))
    }

    fn track_loop_hotness(&mut self, loop_header_pc: usize) {
        if self.current_thread.is_some() {
            return;
        }

        if !self.jit.is_active() {
            return;
        }

        let Some(frame) = self.frames.last() else {
            return;
        };

        let base = frame.base;
        let proto = frame.closure.proto.clone();
        let key = TraceKey::new(Rc::as_ptr(&proto) as usize, loop_header_pc);
        let hits = {
            let counter = self.loop_hotness.entry(key).or_insert(0);
            *counter = counter.saturating_add(1);
            *counter
        };

        if !self.jit.should_record_trace(&key) {
            return;
        }

        if hits < self.jit.hot_threshold() {
            return;
        }

        self.jit.note_hot_loop_trigger(key);

        let max_stack = proto.max_stack_size as usize;
        let slot_types: Vec<ValueType> = (0..max_stack)
            .map(|idx| self.slot_value_type(base + idx))
            .collect();
        let request = RecordingRequest {
            key,
            code: &proto.code,
            constants: &proto.constants,
            slot_types: &slot_types,
        };
        let mut recorder = LoopTraceRecorder;
        if let Err(err) = self.jit.record_trace(&mut recorder, &request) {
            #[cfg(feature = "trace-jit")]
            eprintln!(
                "[trace-jit] failed to record trace at function=0x{:x} loop_header_pc={}: {:?}",
                key.function, key.loop_header_pc, err
            );

            #[cfg(not(feature = "trace-jit"))]
            let _ = err;
        }
    }
}

impl Default for VmState {
    fn default() -> Self {
        Self::new()
    }
}

impl NativeVmContext for VmState {
    fn call_function(
        &mut self,
        func: &LuaValue,
        args: &[LuaValue],
    ) -> Result<Vec<LuaValue>, String> {
        call_function(self, func, args).map_err(|err| err.to_string())
    }

    fn source_location(&self) -> String {
        VmState::source_location(self)
    }

    fn create_coroutine(&mut self, func: &LuaValue) -> Result<LuaValue, String> {
        self.create_coroutine_state(func.clone())
            .map_err(|err| err.to_string())
    }

    fn resume_coroutine(
        &mut self,
        thread: &LuaValue,
        args: &[LuaValue],
    ) -> Result<Vec<LuaValue>, String> {
        let LuaValue::Thread(thread) = thread else {
            return Err(format!(
                "bad argument #1 to 'resume' (thread expected, got {})",
                thread.type_name()
            ));
        };
        self.resume_coroutine_state(thread, args)
            .map_err(|err| err.to_string())
    }

    fn running_coroutine(&self) -> Option<LuaValue> {
        self.current_thread
            .as_ref()
            .map(|thread| LuaValue::Thread(thread.clone()))
    }

    fn coroutine_status(&self, thread: &LuaValue) -> Result<&'static str, String> {
        let LuaValue::Thread(thread) = thread else {
            return Err(format!(
                "bad argument #1 to 'status' (thread expected, got {})",
                thread.type_name()
            ));
        };
        Ok(self.coroutine_status_of(thread))
    }

    fn yield_current(&mut self, args: &[LuaValue]) -> Result<CallOutcome, String> {
        if self.current_thread.is_none() {
            return Err("attempt to yield from outside a coroutine".to_owned());
        }
        Ok(CallOutcome::Yield(args.to_vec()))
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
        for coroutine in self.coroutines.values() {
            roots.push(GcRoot {
                source: RootSource::OpenUpvalues,
                value: LuaValue::Thread(coroutine.thread.clone()),
            });
            if let Some(entry) = &coroutine.entry {
                roots.push(GcRoot {
                    source: RootSource::OpenUpvalues,
                    value: entry.clone(),
                });
            }
            for value in &coroutine.context.stack {
                if !matches!(value, LuaValue::Nil) {
                    roots.push(GcRoot {
                        source: RootSource::OpenUpvalues,
                        value: value.clone(),
                    });
                }
            }
            for uv in coroutine.context.open_upvalues.values() {
                roots.push(GcRoot {
                    source: RootSource::OpenUpvalues,
                    value: uv.borrow().clone(),
                });
            }
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

fn value_type_of(value: &LuaValue) -> ValueType {
    match value {
        LuaValue::Nil => ValueType::Nil,
        LuaValue::Boolean(_) => ValueType::Boolean,
        LuaValue::Number(_) => ValueType::Number,
        LuaValue::String(_) => ValueType::String,
        LuaValue::Table(_) => ValueType::Table,
        LuaValue::Function(_) => ValueType::Function,
        LuaValue::Thread(_) => ValueType::Thread,
    }
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
    match execute_closure_outcome(state, closure, args)? {
        CallOutcome::Return(values) => Ok(values),
        CallOutcome::Yield(_) => {
            state.pending_resume = None;
            Err(LuaError::Runtime(
                "attempt to yield from outside a coroutine".to_owned(),
            ))
        }
    }
}

fn execute_closure_outcome(
    state: &mut VmState,
    closure: Rc<Closure>,
    args: &[LuaValue],
) -> Result<CallOutcome, LuaError> {
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

    let result = run_loop_outcome(state);
    let should_cleanup =
        !matches!(result, Ok(CallOutcome::Yield(_))) || state.current_thread.is_none();
    if should_cleanup {
        state.close_upvalues(base);
        state.stack.truncate(base);
        state.frames.pop();
    }

    result
}

fn maybe_run_cached_trace(
    state: &mut VmState,
    base: usize,
    pc: usize,
    proto: &Rc<FunctionProto>,
) -> Result<bool, LuaError> {
    if state.current_thread.is_some() {
        return Ok(false);
    }

    if !state.jit.is_active() {
        return Ok(false);
    }

    let key = TraceKey::new(Rc::as_ptr(proto) as usize, pc);

    if let Some(pending) = state.pending_side_exit {
        if pending == key {
            state.pending_side_exit = None;
            return Ok(false);
        }
        state.pending_side_exit = None;
    }

    let cached = {
        let Some(cached) = state.jit.lookup_cached_trace(&key) else {
            return Ok(false);
        };
        cached
    };

    if cached.lifecycle_state == TraceLifecycleState::Invalidated {
        state.jit.note_invalidated_bypass(key);
        #[cfg(feature = "trace-jit")]
        eprintln!(
            "[trace-jit] bypass-invalidated function=0x{:x} loop_header_pc={} generation={}",
            key.function, key.loop_header_pc, cached.generation
        );
        return Ok(false);
    }

    if cached.lifecycle_state == TraceLifecycleState::Active {
        match try_run_native_trace(state, base, key, &cached)? {
            NativeTraceResult::Completed => {
                state.frames.last_mut().unwrap().pc = cached.trace.exit_pc;
                return Ok(true);
            }
            NativeTraceResult::SideExit(exit) => {
                state
                    .jit
                    .note_side_exit(key, exit.resume_pc, Some(&exit.deopt));
                state.pending_side_exit = Some(key);
                state.frames.last_mut().unwrap().pc = exit.resume_pc;
                return Ok(true);
            }
            NativeTraceResult::FallbackToReplay => {}
        }
    }

    state.jit.note_replay_entry(key);

    match replay_trace(state, base, proto, key, &cached)? {
        TraceReplayResult::Completed => {
            state.frames.last_mut().unwrap().pc = cached.trace.exit_pc;
        }
        TraceReplayResult::SideExit(exit) => {
            state
                .jit
                .note_side_exit(key, exit.resume_pc, Some(&exit.deopt));
            state.pending_side_exit = Some(key);
            state.frames.last_mut().unwrap().pc = exit.resume_pc;
        }
    }

    Ok(true)
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TraceReplayResult {
    Completed,
    SideExit(TraceExitState),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum NativeTraceResult {
    Completed,
    SideExit(TraceExitState),
    FallbackToReplay,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TraceExitState {
    resume_pc: usize,
    deopt: TraceDeoptExit,
}

fn try_run_native_trace(
    state: &mut VmState,
    base: usize,
    key: TraceKey,
    cached: &rlua_jit::CachedTraceHandle,
) -> Result<NativeTraceResult, LuaError> {
    let Some(native) = cached.native.as_ref() else {
        return Ok(NativeTraceResult::FallbackToReplay);
    };

    for guard in &cached.optimized.guards {
        let actual = value_type_of(&state.get_reg(base, guard.slot as u8));
        if actual != guard.expected {
            return Ok(NativeTraceResult::SideExit(TraceExitState {
                resume_pc: guard.exit.resume_pc,
                deopt: guard_deopt_exit(cached, guard.id, guard.slot, guard.exit.resume_pc),
            }));
        }
    }

    let mut slots = vec![0.0; native.slot_count()];
    for slot in trace_live_in_slots(cached) {
        let Some(value) = state.get_reg(base, slot as u8).to_number() else {
            state.jit.note_native_failure(key);
            return Ok(NativeTraceResult::FallbackToReplay);
        };
        slots[slot as usize] = value;
    }

    match native.execute(&mut slots) {
        NativeTraceOutcome::ContinueLoop => {
            sync_native_slots(state, base, native.written_slots(), &slots);
            state.jit.note_native_entry(key);
            Ok(NativeTraceResult::Completed)
        }
        NativeTraceOutcome::SideExit(side_exit_pc) => {
            let deopt = side_exit_deopt(
                cached,
                side_exit_pc,
                side_exit_pc.saturating_add(1),
                native.written_slots(),
            );
            sync_native_slots(state, base, &deopt.materialized_slots, &slots);
            state.jit.note_native_entry(key);
            Ok(NativeTraceResult::SideExit(TraceExitState {
                resume_pc: deopt.resume_pc,
                deopt,
            }))
        }
        NativeTraceOutcome::Unavailable => {
            state.jit.note_native_failure(key);
            Ok(NativeTraceResult::FallbackToReplay)
        }
    }
}

fn sync_native_slots(state: &mut VmState, base: usize, written_slots: &[u16], slots: &[f64]) {
    for &slot in written_slots {
        if let Some(value) = slots.get(slot as usize) {
            state.set_reg(base, slot as u8, LuaValue::Number(*value));
        }
    }
}

fn trace_live_in_slots(cached: &rlua_jit::CachedTraceHandle) -> Vec<u16> {
    cached
        .deopt_exits
        .first()
        .map(|exit| exit.live_in_slots.clone())
        .unwrap_or_else(|| cached.optimized.read_slots())
}

fn guard_deopt_exit(
    cached: &rlua_jit::CachedTraceHandle,
    guard_id: u32,
    slot: u16,
    resume_pc: usize,
) -> TraceDeoptExit {
    cached
        .optimized
        .guard_deopt_exit(guard_id)
        .cloned()
        .unwrap_or_else(|| TraceDeoptExit {
            kind: TraceDeoptExitKind::Guard { guard_id, slot },
            resume_pc,
            live_in_slots: trace_live_in_slots(cached),
            materialized_slots: Vec::new(),
        })
}

fn side_exit_deopt(
    cached: &rlua_jit::CachedTraceHandle,
    side_exit_pc: usize,
    resume_pc: usize,
    materialized_slots: &[u16],
) -> TraceDeoptExit {
    cached
        .optimized
        .side_exit_deopt(side_exit_pc)
        .cloned()
        .unwrap_or_else(|| TraceDeoptExit {
            kind: TraceDeoptExitKind::SideExit { pc: side_exit_pc },
            resume_pc,
            live_in_slots: trace_live_in_slots(cached),
            materialized_slots: materialized_slots.to_vec(),
        })
}

fn replay_trace(
    state: &mut VmState,
    base: usize,
    proto: &FunctionProto,
    key: TraceKey,
    cached: &rlua_jit::CachedTraceHandle,
) -> Result<TraceReplayResult, LuaError> {
    for op in &cached.trace.ops {
        match op {
            IrOp::GuardType(guard) => {
                let actual = value_type_of(&state.get_reg(base, guard.slot as u8));
                if actual != guard.expected {
                    return Ok(TraceReplayResult::SideExit(TraceExitState {
                        resume_pc: guard.exit.resume_pc,
                        deopt: guard_deopt_exit(cached, guard.id, guard.slot, guard.exit.resume_pc),
                    }));
                }
            }
            IrOp::Instruction(recorded) => {
                if let Some(resume_pc) = execute_replay_instruction(
                    state,
                    base,
                    proto,
                    key,
                    recorded.pc,
                    recorded.instruction,
                )? {
                    return Ok(TraceReplayResult::SideExit(TraceExitState {
                        resume_pc,
                        deopt: side_exit_deopt(cached, recorded.pc, resume_pc, &[]),
                    }));
                }
            }
        }
    }

    Ok(TraceReplayResult::Completed)
}

fn execute_replay_instruction(
    state: &mut VmState,
    base: usize,
    proto: &FunctionProto,
    key: TraceKey,
    pc: usize,
    instruction: rlua_core::bytecode::Instruction,
) -> Result<Option<usize>, LuaError> {
    let op = instruction.opcode();
    let a = instruction.a();

    match op {
        Opcode::Move => {
            let b = instruction.b();
            let val = state.get_reg(base, b as u8);
            state.set_reg(base, a, val);
            Ok(None)
        }
        Opcode::LoadK => {
            let bx = instruction.bx() as usize;
            let val = proto.constants[bx].clone();
            state.set_reg(base, a, val);
            Ok(None)
        }
        Opcode::LoadBool => {
            let b = instruction.b();
            state.set_reg(base, a, LuaValue::Boolean(b != 0));
            Ok(None)
        }
        Opcode::LoadNil => {
            let b = instruction.b() as u8;
            for i in a..=a + b {
                state.set_reg(base, i, LuaValue::Nil);
            }
            Ok(None)
        }
        Opcode::Add | Opcode::Sub | Opcode::Mul | Opcode::Div | Opcode::Mod | Opcode::Pow => {
            let lhs = state.rk(base, proto, instruction.b());
            let rhs = state.rk(base, proto, instruction.c());
            let (Some(ln), Some(rn)) = (lhs.to_number(), rhs.to_number()) else {
                return Ok(Some(pc));
            };

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
            Ok(None)
        }
        Opcode::Unm => {
            let b = instruction.b();
            let Some(num) = state.get_reg(base, b as u8).to_number() else {
                return Ok(Some(pc));
            };
            state.set_reg(base, a, LuaValue::Number(-num));
            Ok(None)
        }
        Opcode::Not => {
            let b = instruction.b();
            let value = state.get_reg(base, b as u8);
            state.set_reg(base, a, LuaValue::Boolean(!value.is_truthy()));
            Ok(None)
        }
        Opcode::Jmp => {
            let target_pc = (pc as i32 + 1 + instruction.sbx()) as usize;
            if target_pc == key.loop_header_pc {
                Ok(None)
            } else {
                Ok(Some(pc))
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
                Ok(None)
            } else {
                Ok(Some(pc + 1))
            }
        }
        _ => Ok(Some(pc)),
    }
}

fn run_loop_outcome(state: &mut VmState) -> Result<CallOutcome, LuaError> {
    loop {
        let frame = state.frames.last().unwrap();
        let base = frame.base;
        let pc = frame.pc;
        let proto = frame.closure.proto.clone();

        if pc >= proto.code.len() {
            return Ok(CallOutcome::Return(Vec::new()));
        }

        if maybe_run_cached_trace(state, base, pc, &proto)? {
            continue;
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
                if sbx < 0 {
                    state.track_loop_hotness(new_pc);
                    if state.gc.alloc_count() >= state.gc.threshold() {
                        state.run_gc();
                    }
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
                    match call_function_outcome(state, &func_val, &args)? {
                        CallOutcome::Return(results) => results,
                        CallOutcome::Yield(values) => {
                            if state.pending_resume.is_none() {
                                state.pending_resume = Some(PendingResume {
                                    target: ResumeTarget::Call {
                                        base,
                                        register: a,
                                        num_results_wanted,
                                    },
                                });
                            }
                            return Ok(CallOutcome::Yield(values));
                        }
                    }
                };

                write_call_results(state, base, a, num_results_wanted, &results);
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

                // Tail call optimization: if calling a Lua closure, reuse current frame.
                if let LuaValue::Function(f) = &func_val
                    && let LuaFunction::Lua(closure) = f.as_ref()
                {
                    let new_proto = &closure.proto;
                    state.close_upvalues(base);

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

                    let frame = state.frames.last_mut().unwrap();
                    frame.closure = closure.clone();
                    frame.pc = 0;
                    frame.varargs = varargs;
                    continue;
                }

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
                    match call_function_outcome(state, &func_val, &args)? {
                        CallOutcome::Return(results) => results,
                        CallOutcome::Yield(values) => {
                            if state.pending_resume.is_none() {
                                state.pending_resume = Some(PendingResume {
                                    target: ResumeTarget::TailReturn,
                                });
                            }
                            return Ok(CallOutcome::Yield(values));
                        }
                    }
                };

                return Ok(CallOutcome::Return(results));
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
                    return Ok(CallOutcome::Return(results));
                } else if b == 1 {
                    return Ok(CallOutcome::Return(Vec::new()));
                } else {
                    let count = (b - 1) as usize;
                    let results: Vec<LuaValue> = (0..count)
                        .map(|i| state.get_reg(base, a + i as u8).clone())
                        .collect();
                    return Ok(CallOutcome::Return(results));
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
                    if sbx < 0 {
                        state.track_loop_hotness(new_pc);
                    }
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
                return Ok(CallOutcome::Return(Vec::new()));
            }
        }
    }
}

fn call_function_outcome(
    state: &mut VmState,
    func_val: &LuaValue,
    args: &[LuaValue],
) -> Result<CallOutcome, LuaError> {
    match func_val {
        LuaValue::Function(f) => match f.as_ref() {
            LuaFunction::Native { func, .. } => func(state, args).map_err(LuaError::Runtime),
            LuaFunction::Lua(closure) => execute_closure_outcome(state, closure.clone(), args),
            LuaFunction::WrappedCoroutine { thread } => {
                let mut results = state.resume_coroutine_state(thread, args)?;
                if matches!(results.first(), Some(LuaValue::Boolean(true))) {
                    results.remove(0);
                    Ok(CallOutcome::Return(results))
                } else {
                    let err = results
                        .get(1)
                        .cloned()
                        .unwrap_or_else(|| LuaValue::from("coroutine.wrap failed"));
                    Err(LuaError::Runtime(err.to_lua_string()))
                }
            }
        },
        LuaValue::Table(t) => {
            // __call metamethod
            if let Some(mm) = t.borrow().get_metamethod("__call") {
                let mut call_args = vec![func_val.clone()];
                call_args.extend_from_slice(args);
                call_function_outcome(state, &mm, &call_args)
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

fn call_function(
    state: &mut VmState,
    func_val: &LuaValue,
    args: &[LuaValue],
) -> Result<Vec<LuaValue>, LuaError> {
    let frame_depth = state.frames.len();
    match call_function_outcome(state, func_val, args)? {
        CallOutcome::Return(values) => Ok(values),
        CallOutcome::Yield(_) => {
            state.cleanup_frames_to_depth(frame_depth);
            Err(LuaError::Runtime(
                "attempt to yield across a native callback boundary".to_owned(),
            ))
        }
    }
}

fn lua_error_to_value(e: LuaError) -> LuaValue {
    match e {
        LuaError::Runtime(s) | LuaError::Type(s) | LuaError::Arithmetic(s) => LuaValue::from(s),
    }
}

fn write_call_results(
    state: &mut VmState,
    base: usize,
    register: u8,
    num_results_wanted: i32,
    results: &[LuaValue],
) {
    let num_results = results.len();
    if num_results_wanted < 0 {
        for (i, val) in results.iter().cloned().enumerate() {
            state.set_reg(base, register + i as u8, val);
        }
        state.stack.truncate(base + register as usize + num_results);
    } else {
        for i in 0..num_results_wanted as usize {
            let val = results.get(i).cloned().unwrap_or(LuaValue::Nil);
            state.set_reg(base, register + i as u8, val);
        }
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
    use rlua_jit::TraceExecutionState;

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
    fn records_hot_loop_trace_once_threshold_is_reached() {
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

        let mut state = VmState::with_jit_config(JitConfig {
            hot_threshold: 2,
            ..JitConfig::default()
        });
        let results = execute(&mut state, proto).unwrap();
        let debug = state.jit_debug_state();

        assert_eq!(results[0], LuaValue::Number(15.0));
        assert_eq!(debug.trace_count, 1);
        assert_eq!(debug.stats.hot_loop_triggers, 1);
        assert_eq!(debug.stats.trace_installs, 1);
        assert_eq!(debug.stats.optimize_attempts, 1);
        assert_eq!(debug.traces.len(), 1);
        assert_eq!(debug.traces[0].side_exit_count, 1);
        assert!(matches!(
            debug.traces[0].last_deopt.as_ref().map(|exit| exit.kind),
            Some(TraceDeoptExitKind::SideExit { .. })
        ));
        assert_eq!(
            debug.traces[0]
                .last_deopt
                .as_ref()
                .map(|exit| exit.resume_pc),
            Some(7)
        );
        if cfg!(target_arch = "x86_64") {
            assert!(debug.stats.native_compile_installs >= 1);
            assert!(debug.stats.native_entries >= 1);
        } else {
            assert_eq!(debug.availability, JitAvailability::UnsupportedArch);
            assert!(debug.stats.replay_entries >= 1);
            assert_eq!(debug.stats.native_entries, 0);
        }
        assert!(
            debug
                .counters
                .iter()
                .any(|counter| counter.loop_header_pc == 5 && counter.hits >= 2)
        );
    }

    #[test]
    fn disabled_jit_keeps_interpreter_only_mode() {
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

        let mut state = VmState::with_jit_config(JitConfig {
            enabled: false,
            hot_threshold: 1,
            ..JitConfig::default()
        });
        let results = execute(&mut state, proto).unwrap();
        let debug = state.jit_debug_state();

        assert_eq!(results[0], LuaValue::Number(15.0));
        assert_eq!(debug.execution_mode, ExecutionMode::InterpreterOnly);
        assert_eq!(debug.trace_count, 0);
        assert_eq!(debug.stats.hot_loop_triggers, 0);
    }

    #[test]
    fn guard_failure_side_exits_back_to_interpreter_once() {
        let proto = Rc::new(FunctionProto {
            code: vec![
                Instruction::encode_abc(Opcode::Move, 0, 0, 0),
                Instruction::encode_abc(Opcode::Return, 0, 2, 0),
            ],
            max_stack_size: 2,
            num_params: 1,
            is_vararg: false,
            ..FunctionProto::new()
        });
        let closure = Rc::new(Closure::new(proto.clone()));
        let key = TraceKey::new(Rc::as_ptr(&proto) as usize, 0);
        let mut trace = Trace::new(key.function, 0);
        trace.push_guard(0, ValueType::Number, 0);
        trace.push_instruction(0, Instruction::encode_abc(Opcode::Move, 0, 0, 0));
        trace.set_exit_pc(1);

        let mut state = VmState::with_jit_config(JitConfig {
            hot_threshold: 1,
            ..JitConfig::default()
        });
        assert!(state.jit.install_trace(key, trace));

        let results =
            execute_closure(&mut state, closure, &[LuaValue::from("not-a-number")]).unwrap();
        let debug = state.jit_debug_state();

        assert_eq!(results, vec![LuaValue::from("not-a-number")]);
        assert_eq!(debug.stats.replay_entries, 1);
        assert_eq!(debug.stats.side_exits, 1);
        assert_eq!(debug.trace_count, 1);
        assert_eq!(debug.traces[0].replay_entries, 1);
        assert_eq!(
            debug.traces[0].lifecycle_state,
            TraceLifecycleState::ReplayOnly
        );
        assert_eq!(
            debug.traces[0].last_execution,
            TraceExecutionState::InterpreterFallback
        );
        assert_eq!(debug.traces[0].side_exit_count, 1);
        assert!(matches!(
            debug.traces[0].last_deopt.as_ref().map(|exit| exit.kind),
            Some(TraceDeoptExitKind::Guard {
                guard_id: 0,
                slot: 0
            })
        ));
    }

    #[test]
    fn invalidated_trace_is_bypassed_after_repeated_side_exits() {
        let proto = Rc::new(FunctionProto {
            code: vec![
                Instruction::encode_abc(Opcode::Move, 0, 0, 0),
                Instruction::encode_abc(Opcode::Return, 0, 2, 0),
            ],
            max_stack_size: 2,
            num_params: 1,
            is_vararg: false,
            ..FunctionProto::new()
        });
        let closure = Rc::new(Closure::new(proto.clone()));
        let key = TraceKey::new(Rc::as_ptr(&proto) as usize, 0);
        let mut trace = Trace::new(key.function, 0);
        trace.push_guard(0, ValueType::Number, 0);
        trace.push_instruction(0, Instruction::encode_abc(Opcode::Move, 0, 0, 0));
        trace.set_exit_pc(1);

        let mut state = VmState::with_jit_config(JitConfig {
            hot_threshold: 1,
            side_exit_threshold: 1,
            ..JitConfig::default()
        });
        assert!(state.jit.install_trace(key, trace));

        let first = execute_closure(
            &mut state,
            closure.clone(),
            &[LuaValue::from("not-a-number")],
        )
        .unwrap();
        let second = execute_closure(
            &mut state,
            closure.clone(),
            &[LuaValue::from("still-not-a-number")],
        )
        .unwrap();
        let debug_after_second = state.jit_debug_state();
        let replay_entries_after_second = debug_after_second.stats.replay_entries;

        let third =
            execute_closure(&mut state, closure, &[LuaValue::from("bypassed-now")]).unwrap();
        let debug = state.jit_debug_state();

        assert_eq!(first, vec![LuaValue::from("not-a-number")]);
        assert_eq!(second, vec![LuaValue::from("still-not-a-number")]);
        assert_eq!(third, vec![LuaValue::from("bypassed-now")]);
        assert_eq!(debug.trace_count, 1);
        assert_eq!(
            debug.traces[0].lifecycle_state,
            TraceLifecycleState::Invalidated
        );
        assert_eq!(debug.traces[0].side_exit_count, 2);
        assert_eq!(debug.stats.replay_entries, replay_entries_after_second);
        assert_eq!(debug.stats.side_exits, 2);
        assert_eq!(debug.stats.invalidated_bypasses, 1);
        assert_eq!(debug.traces[0].invalidated_bypasses, 1);
        assert_eq!(
            debug.traces[0].last_execution,
            TraceExecutionState::InterpreterFallback
        );
    }

    #[test]
    fn execute_native_call() {
        fn my_add(
            _ctx: &mut dyn NativeVmContext,
            args: &[LuaValue],
        ) -> Result<CallOutcome, String> {
            let a = args.first().and_then(|v| v.to_number()).unwrap_or(0.0);
            let b = args.get(1).and_then(|v| v.to_number()).unwrap_or(0.0);
            Ok(CallOutcome::Return(vec![LuaValue::Number(a + b)]))
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
