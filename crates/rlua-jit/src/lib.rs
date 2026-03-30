mod executable;
mod x86_64;

use core::fmt;
use std::collections::HashMap;
use std::rc::Rc;

use executable::ExecutableBuffer;
use rlua_core::bytecode::Instruction;
use rlua_core::opcode::Opcode;
use rlua_core::value::LuaValue;
use rlua_ir::{
    ArithmeticOp, ConstantValue, OptimizationReport, OptimizedTrace, Trace, TraceOperand,
    TraceStep, TraceStepKind, ValueType, optimize_trace,
};
use x86_64::{EncodedTrace, X86_64TraceCompiler};

pub const DEFAULT_HOT_THRESHOLD: u32 = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JitAvailability {
    Available,
    UnsupportedArch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionMode {
    InterpreterOnly,
    JitEnabled,
    JitUnavailable,
}

impl fmt::Display for ExecutionMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InterpreterOnly => f.write_str("interpreter-only"),
            Self::JitEnabled => f.write_str("jit-enabled"),
            Self::JitUnavailable => f.write_str("jit-unavailable"),
        }
    }
}

pub const fn detect_jit_availability() -> JitAvailability {
    #[cfg(target_arch = "x86_64")]
    {
        JitAvailability::Available
    }

    #[cfg(not(target_arch = "x86_64"))]
    {
        JitAvailability::UnsupportedArch
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JitConfig {
    pub enabled: bool,
    pub hot_threshold: u32,
}

impl Default for JitConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            hot_threshold: DEFAULT_HOT_THRESHOLD,
        }
    }
}

#[derive(Debug)]
pub enum JitError {
    Unsupported,
    EmptyLoop(TraceKey),
    MissingLoopTerminator(TraceKey),
    UnsupportedTrace(String),
    Codegen(String),
    ExecutableBuffer(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TraceKey {
    pub function: usize,
    pub loop_header_pc: usize,
}

impl TraceKey {
    pub const fn new(function: usize, loop_header_pc: usize) -> Self {
        Self {
            function,
            loop_header_pc,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeArtifactState {
    Unavailable,
    UnsupportedArch,
    UnsupportedTrace,
    Installed,
    CompileFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct JitStats {
    pub hot_loop_triggers: u64,
    pub record_attempts: u64,
    pub trace_installs: u64,
    pub cache_hits: u64,
    pub replay_entries: u64,
    pub side_exits: u64,
    pub optimize_attempts: u64,
    pub optimized_traces: u64,
    pub native_compile_attempts: u64,
    pub native_compile_installs: u64,
    pub native_compile_skips: u64,
    pub native_entries: u64,
    pub native_failures: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceCacheDebugEntry {
    pub function: usize,
    pub loop_header_pc: usize,
    pub optimized: bool,
    pub optimization_report: OptimizationReport,
    pub native_state: NativeArtifactState,
    pub native_entries: u64,
}

#[derive(Debug, Clone)]
pub struct CachedTraceHandle {
    pub trace: Trace,
    pub optimized: OptimizedTrace,
    pub native: Option<Rc<NativeTraceArtifact>>,
    pub native_state: NativeArtifactState,
}

#[derive(Debug, Clone)]
pub struct RecordingRequest<'a> {
    pub key: TraceKey,
    pub code: &'a [Instruction],
    pub constants: &'a [LuaValue],
    pub slot_types: &'a [ValueType],
}

pub trait TraceRecorder {
    fn record(&mut self, request: &RecordingRequest<'_>) -> Result<Trace, JitError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeTraceOutcome {
    ContinueLoop,
    SideExit(usize),
    Unavailable,
}

#[derive(Debug)]
pub struct NativeTraceArtifact {
    #[cfg_attr(not(target_arch = "x86_64"), allow(dead_code))]
    executable: ExecutableBuffer,
    code: Vec<u8>,
    slot_count: usize,
    written_slots: Vec<u16>,
    side_exit_pc: usize,
}

impl NativeTraceArtifact {
    fn install(encoded: EncodedTrace) -> Result<Self, JitError> {
        let executable = ExecutableBuffer::install(&encoded.code)?;
        Ok(Self {
            executable,
            code: encoded.code,
            slot_count: encoded.slot_count,
            written_slots: encoded.written_slots,
            side_exit_pc: encoded.side_exit_pc,
        })
    }

    pub fn code(&self) -> &[u8] {
        &self.code
    }

    pub fn slot_count(&self) -> usize {
        self.slot_count
    }

    pub fn written_slots(&self) -> &[u16] {
        &self.written_slots
    }

    pub fn side_exit_pc(&self) -> usize {
        self.side_exit_pc
    }

    #[cfg(target_arch = "x86_64")]
    pub fn execute(&self, slots: &mut [f64]) -> NativeTraceOutcome {
        type NativeTraceFn = unsafe extern "C" fn(*mut f64) -> u32;

        if slots.len() < self.slot_count {
            return NativeTraceOutcome::Unavailable;
        }

        let entry: NativeTraceFn = unsafe { std::mem::transmute(self.executable.as_ptr()) };
        let raw = unsafe { entry(slots.as_mut_ptr()) };
        if raw == 0 {
            NativeTraceOutcome::ContinueLoop
        } else {
            NativeTraceOutcome::SideExit(self.side_exit_pc)
        }
    }

    #[cfg(not(target_arch = "x86_64"))]
    pub fn execute(&self, _slots: &mut [f64]) -> NativeTraceOutcome {
        NativeTraceOutcome::Unavailable
    }
}

#[derive(Debug, Default)]
pub struct LoopTraceRecorder;

impl TraceRecorder for LoopTraceRecorder {
    fn record(&mut self, request: &RecordingRequest<'_>) -> Result<Trace, JitError> {
        if request.key.loop_header_pc >= request.code.len() {
            return Err(JitError::EmptyLoop(request.key));
        }

        let mut trace = Trace::new(request.key.function, request.key.loop_header_pc);
        let mut saw_back_edge = false;

        for (pc, instruction) in request
            .code
            .iter()
            .copied()
            .enumerate()
            .skip(request.key.loop_header_pc)
        {
            append_guards(&mut trace, instruction, request.slot_types, pc);
            trace.push_instruction(pc, instruction);
            trace.push_step(lower_step(pc, instruction, request.constants));

            if let Some(target_pc) = backward_edge_target(instruction, pc) {
                trace.set_exit_pc(target_pc);
                saw_back_edge = true;
                break;
            }
        }

        if !saw_back_edge {
            return Err(JitError::MissingLoopTerminator(request.key));
        }

        #[cfg(feature = "trace-jit")]
        eprintln!(
            "[trace-jit] recorded trace function=0x{:x} loop_header_pc={} ops={} guards={} steps={}",
            request.key.function,
            request.key.loop_header_pc,
            trace.ops.len(),
            trace.guards.len(),
            trace.steps.len()
        );

        Ok(trace)
    }
}

#[derive(Debug)]
struct CachedTrace {
    trace: Trace,
    optimized: OptimizedTrace,
    native: Option<Rc<NativeTraceArtifact>>,
    native_state: NativeArtifactState,
    native_entries: u64,
}

impl CachedTrace {
    fn handle(&self) -> CachedTraceHandle {
        CachedTraceHandle {
            trace: self.trace.clone(),
            optimized: self.optimized.clone(),
            native: self.native.clone(),
            native_state: self.native_state,
        }
    }

    fn debug_entry(&self, key: TraceKey) -> TraceCacheDebugEntry {
        TraceCacheDebugEntry {
            function: key.function,
            loop_header_pc: key.loop_header_pc,
            optimized: !self.optimized.steps.is_empty(),
            optimization_report: self.optimized.report,
            native_state: self.native_state,
            native_entries: self.native_entries,
        }
    }
}

#[derive(Debug)]
pub struct JitRuntime {
    config: JitConfig,
    availability: JitAvailability,
    trace_cache: HashMap<TraceKey, CachedTrace>,
    stats: JitStats,
}

impl JitRuntime {
    pub fn new(config: JitConfig) -> Self {
        let availability = detect_jit_availability();

        #[cfg(feature = "trace-jit")]
        eprintln!(
            "[trace-jit] JitRuntime init: enabled={}, hot_threshold={}, availability={:?}",
            config.enabled, config.hot_threshold, availability
        );

        Self {
            config: JitConfig {
                hot_threshold: config.hot_threshold.max(1),
                ..config
            },
            availability,
            trace_cache: HashMap::new(),
            stats: JitStats::default(),
        }
    }

    pub const fn config(&self) -> JitConfig {
        self.config
    }

    pub const fn availability(&self) -> JitAvailability {
        self.availability
    }

    pub const fn hot_threshold(&self) -> u32 {
        self.config.hot_threshold
    }

    pub const fn execution_mode(&self) -> ExecutionMode {
        match (self.config.enabled, self.availability) {
            (false, _) => ExecutionMode::InterpreterOnly,
            (true, JitAvailability::Available) => ExecutionMode::JitEnabled,
            (true, JitAvailability::UnsupportedArch) => ExecutionMode::JitUnavailable,
        }
    }

    pub const fn is_active(&self) -> bool {
        self.config.enabled
    }

    pub fn note_hot_loop_trigger(&mut self, key: TraceKey) {
        self.stats.hot_loop_triggers += 1;

        #[cfg(feature = "trace-jit")]
        eprintln!(
            "[trace-jit] hot loop function=0x{:x} loop_header_pc={}",
            key.function, key.loop_header_pc
        );

        #[cfg(not(feature = "trace-jit"))]
        let _ = key;
    }

    pub fn trace_ref(&self, key: &TraceKey) -> Option<&Trace> {
        self.trace_cache.get(key).map(|cached| &cached.trace)
    }

    pub fn optimized_trace_ref(&self, key: &TraceKey) -> Option<&OptimizedTrace> {
        self.trace_cache.get(key).map(|cached| &cached.optimized)
    }

    pub fn lookup_trace(&mut self, key: &TraceKey) -> Option<&Trace> {
        if self.trace_cache.contains_key(key) {
            self.stats.cache_hits += 1;
        }
        self.trace_cache.get(key).map(|cached| &cached.trace)
    }

    pub fn lookup_cached_trace(&mut self, key: &TraceKey) -> Option<CachedTraceHandle> {
        if self.trace_cache.contains_key(key) {
            self.stats.cache_hits += 1;
        }
        self.trace_cache.get(key).map(CachedTrace::handle)
    }

    pub fn record_trace<R: TraceRecorder>(
        &mut self,
        recorder: &mut R,
        request: &RecordingRequest<'_>,
    ) -> Result<bool, JitError> {
        self.stats.record_attempts += 1;

        if self.trace_cache.contains_key(&request.key) {
            return Ok(false);
        }

        let trace = recorder.record(request)?;
        self.stats.optimize_attempts += 1;
        let optimized = optimize_trace(&trace);
        self.stats.optimized_traces += 1;

        let (native, native_state) = self.prepare_native_artifact(&optimized);

        self.trace_cache.insert(
            request.key,
            CachedTrace {
                trace,
                optimized,
                native,
                native_state,
                native_entries: 0,
            },
        );
        self.stats.trace_installs += 1;
        Ok(true)
    }

    pub fn install_trace(&mut self, key: TraceKey, trace: Trace) -> bool {
        if self.trace_cache.contains_key(&key) {
            return false;
        }

        self.stats.optimize_attempts += 1;
        let optimized = optimize_trace(&trace);
        self.stats.optimized_traces += 1;
        let (native, native_state) = self.prepare_native_artifact(&optimized);

        self.trace_cache.insert(
            key,
            CachedTrace {
                trace,
                optimized,
                native,
                native_state,
                native_entries: 0,
            },
        );
        self.stats.trace_installs += 1;
        true
    }

    pub fn trace_count(&self) -> usize {
        self.trace_cache.len()
    }

    pub const fn stats(&self) -> JitStats {
        self.stats
    }

    pub fn trace_debug_entries(&self) -> Vec<TraceCacheDebugEntry> {
        let mut entries: Vec<TraceCacheDebugEntry> = self
            .trace_cache
            .iter()
            .map(|(key, cached)| cached.debug_entry(*key))
            .collect();
        entries.sort_by_key(|entry| (entry.function, entry.loop_header_pc));
        entries
    }

    pub fn note_replay_entry(&mut self, key: TraceKey) {
        self.stats.replay_entries += 1;

        #[cfg(feature = "trace-jit")]
        eprintln!(
            "[trace-jit] replay function=0x{:x} loop_header_pc={}",
            key.function, key.loop_header_pc
        );

        #[cfg(not(feature = "trace-jit"))]
        let _ = key;
    }

    pub fn note_native_entry(&mut self, key: TraceKey) {
        self.stats.native_entries += 1;
        if let Some(cached) = self.trace_cache.get_mut(&key) {
            cached.native_entries += 1;
        }

        #[cfg(feature = "trace-jit")]
        eprintln!(
            "[trace-jit] native-entry function=0x{:x} loop_header_pc={}",
            key.function, key.loop_header_pc
        );

        #[cfg(not(feature = "trace-jit"))]
        let _ = key;
    }

    pub fn note_native_failure(&mut self, key: TraceKey) {
        self.stats.native_failures += 1;

        #[cfg(feature = "trace-jit")]
        eprintln!(
            "[trace-jit] native-failure function=0x{:x} loop_header_pc={}",
            key.function, key.loop_header_pc
        );

        #[cfg(not(feature = "trace-jit"))]
        let _ = key;
    }

    pub fn note_side_exit(&mut self, key: TraceKey, resume_pc: usize) {
        self.stats.side_exits += 1;

        #[cfg(feature = "trace-jit")]
        eprintln!(
            "[trace-jit] side-exit function=0x{:x} loop_header_pc={} resume_pc={}",
            key.function, key.loop_header_pc, resume_pc
        );

        #[cfg(not(feature = "trace-jit"))]
        let _ = (key, resume_pc);
    }

    fn prepare_native_artifact(
        &mut self,
        optimized: &OptimizedTrace,
    ) -> (Option<Rc<NativeTraceArtifact>>, NativeArtifactState) {
        self.stats.native_compile_attempts += 1;

        if self.availability != JitAvailability::Available {
            self.stats.native_compile_skips += 1;
            return (None, NativeArtifactState::UnsupportedArch);
        }

        if !optimized.native_supported {
            self.stats.native_compile_skips += 1;
            return (None, NativeArtifactState::UnsupportedTrace);
        }

        let encoded = match X86_64TraceCompiler::compile(optimized) {
            Ok(encoded) => encoded,
            Err(JitError::UnsupportedTrace(_)) => {
                self.stats.native_compile_skips += 1;
                return (None, NativeArtifactState::UnsupportedTrace);
            }
            Err(_err) => {
                self.stats.native_failures += 1;

                #[cfg(feature = "trace-jit")]
                eprintln!("[trace-jit] native codegen failed: {:?}", _err);

                return (None, NativeArtifactState::CompileFailed);
            }
        };

        match NativeTraceArtifact::install(encoded) {
            Ok(artifact) => {
                self.stats.native_compile_installs += 1;
                (Some(Rc::new(artifact)), NativeArtifactState::Installed)
            }
            Err(err) => {
                self.stats.native_failures += 1;

                #[cfg(feature = "trace-jit")]
                eprintln!("[trace-jit] native install failed: {:?}", err);

                let _ = err;
                (None, NativeArtifactState::CompileFailed)
            }
        }
    }
}

fn backward_edge_target(instruction: Instruction, pc: usize) -> Option<usize> {
    match instruction.opcode() {
        Opcode::Jmp | Opcode::ForLoop => {
            let target = (pc as i32 + 1 + instruction.sbx()) as usize;
            (target <= pc).then_some(target)
        }
        _ => None,
    }
}

fn append_guards(trace: &mut Trace, instruction: Instruction, slot_types: &[ValueType], pc: usize) {
    let op = instruction.opcode();

    if matches!(
        op,
        Opcode::Add
            | Opcode::Sub
            | Opcode::Mul
            | Opcode::Div
            | Opcode::Mod
            | Opcode::Pow
            | Opcode::Eq
            | Opcode::Lt
            | Opcode::Le
    ) {
        push_guard_for_operand(trace, instruction.b(), slot_types, pc);
        push_guard_for_operand(trace, instruction.c(), slot_types, pc);
    } else if matches!(op, Opcode::Unm | Opcode::Not | Opcode::Len) {
        push_guard_for_operand(trace, instruction.b(), slot_types, pc);
    } else if matches!(op, Opcode::ForLoop) {
        push_guard_for_slot(trace, instruction.a() as u16, slot_types, pc);
        push_guard_for_slot(trace, instruction.a() as u16 + 1, slot_types, pc);
        push_guard_for_slot(trace, instruction.a() as u16 + 2, slot_types, pc);
    }
}

fn push_guard_for_operand(trace: &mut Trace, operand: u16, slot_types: &[ValueType], pc: usize) {
    if Instruction::is_constant(operand) {
        return;
    }

    push_guard_for_slot(trace, operand, slot_types, pc);
}

fn push_guard_for_slot(trace: &mut Trace, slot: u16, slot_types: &[ValueType], pc: usize) {
    let expected = slot_types
        .get(slot as usize)
        .copied()
        .unwrap_or(ValueType::Unknown);
    if expected != ValueType::Unknown {
        trace.push_guard(slot, expected, pc);
    }
}

fn lower_step(pc: usize, instruction: Instruction, constants: &[LuaValue]) -> TraceStep {
    let a = instruction.a() as u16;
    let kind = match instruction.opcode() {
        Opcode::Move => TraceStepKind::Copy {
            dst: a,
            value: TraceOperand::Slot(instruction.b()),
        },
        Opcode::Close => TraceStepKind::Close { from: a },
        Opcode::LoadK => {
            let constant = constants
                .get(instruction.bx() as usize)
                .and_then(constant_value_from_lua);
            match constant {
                Some(constant) => TraceStepKind::Copy {
                    dst: a,
                    value: TraceOperand::Constant(constant),
                },
                None => TraceStepKind::Unsupported,
            }
        }
        opcode => {
            if let Some(op) = ArithmeticOp::from_opcode(opcode) {
                match (
                    lower_rk_operand(instruction.b(), constants),
                    lower_rk_operand(instruction.c(), constants),
                ) {
                    (Some(lhs), Some(rhs)) => TraceStepKind::Arithmetic {
                        dst: a,
                        op,
                        lhs,
                        rhs,
                    },
                    _ => TraceStepKind::Unsupported,
                }
            } else if matches!(opcode, Opcode::ForLoop) {
                TraceStepKind::ForLoop {
                    base: a,
                    exit_resume_pc: pc + 1,
                }
            } else {
                TraceStepKind::Unsupported
            }
        }
    };

    TraceStep::new(pc, instruction, kind)
}

fn lower_rk_operand(operand: u16, constants: &[LuaValue]) -> Option<TraceOperand> {
    if Instruction::is_constant(operand) {
        constants
            .get((operand - rlua_core::bytecode::RK_OFFSET) as usize)
            .and_then(constant_value_from_lua)
            .map(TraceOperand::Constant)
    } else {
        Some(TraceOperand::Slot(operand))
    }
}

fn constant_value_from_lua(value: &LuaValue) -> Option<ConstantValue> {
    match value {
        LuaValue::Nil => Some(ConstantValue::Nil),
        LuaValue::Boolean(value) => Some(ConstantValue::Boolean(*value)),
        LuaValue::Number(value) => Some(ConstantValue::Number(*value)),
        LuaValue::String(_) | LuaValue::Table(_) | LuaValue::Function(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recorder_emits_trace_metadata_guards_and_lowered_steps() {
        let request = RecordingRequest {
            key: TraceKey::new(0xabc, 0),
            code: &[
                Instruction::encode_abc(Opcode::Add, 0, 1, 2),
                Instruction::encode_asbx(Opcode::ForLoop, 1, -2),
            ],
            constants: &[],
            slot_types: &[
                ValueType::Number,
                ValueType::Number,
                ValueType::Number,
                ValueType::Number,
            ],
        };

        let mut recorder = LoopTraceRecorder;
        let trace = recorder.record(&request).unwrap();

        assert_eq!(trace.function_id, 0xabc);
        assert_eq!(trace.loop_header_pc, 0);
        assert_eq!(trace.exit_pc, 0);
        assert_eq!(trace.guards.len(), 5);
        assert_eq!(trace.steps.len(), 2);
        assert!(matches!(trace.ops[0], rlua_ir::IrOp::GuardType(_)));
        assert!(matches!(
            trace.steps[0].kind,
            TraceStepKind::Arithmetic {
                dst: 0,
                op: ArithmeticOp::Add,
                lhs: TraceOperand::Slot(1),
                rhs: TraceOperand::Slot(2)
            }
        ));
        assert!(matches!(
            trace.steps[1].kind,
            TraceStepKind::ForLoop {
                base: 1,
                exit_resume_pc: 2
            }
        ));
    }

    #[test]
    fn runtime_caches_recorded_traces_and_optimizer_metadata() {
        let request = RecordingRequest {
            key: TraceKey::new(0xabc, 0),
            code: &[
                Instruction::encode_abc(Opcode::Add, 0, 1, 2),
                Instruction::encode_asbx(Opcode::ForLoop, 1, -2),
            ],
            constants: &[],
            slot_types: &[
                ValueType::Number,
                ValueType::Number,
                ValueType::Number,
                ValueType::Number,
            ],
        };

        let mut runtime = JitRuntime::new(JitConfig {
            hot_threshold: 2,
            ..JitConfig::default()
        });
        let mut recorder = LoopTraceRecorder;

        assert!(runtime.record_trace(&mut recorder, &request).unwrap());
        assert!(!runtime.record_trace(&mut recorder, &request).unwrap());
        assert_eq!(runtime.trace_count(), 1);
        assert!(runtime.trace_ref(&request.key).is_some());
        assert!(runtime.optimized_trace_ref(&request.key).is_some());
        assert_eq!(runtime.stats().trace_installs, 1);
        assert_eq!(runtime.stats().record_attempts, 2);
        assert_eq!(runtime.stats().optimize_attempts, 1);

        let debug = runtime.trace_debug_entries();
        assert_eq!(debug.len(), 1);
        assert!(debug[0].optimized);
    }

    #[test]
    fn unsupported_trace_keeps_replay_only_state() {
        let request = RecordingRequest {
            key: TraceKey::new(0xabc, 0),
            code: &[
                Instruction::encode_abc(Opcode::Call, 0, 1, 1),
                Instruction::encode_asbx(Opcode::Jmp, 0, -2),
            ],
            constants: &[],
            slot_types: &[ValueType::Function],
        };

        let mut runtime = JitRuntime::new(JitConfig::default());
        let mut recorder = LoopTraceRecorder;
        runtime.record_trace(&mut recorder, &request).unwrap();

        let debug = runtime.trace_debug_entries();
        assert_eq!(debug.len(), 1);
        let expected = if runtime.availability() == JitAvailability::Available {
            NativeArtifactState::UnsupportedTrace
        } else {
            NativeArtifactState::UnsupportedArch
        };
        assert_eq!(debug[0].native_state, expected);
    }

    #[test]
    #[cfg(target_arch = "x86_64")]
    fn native_artifact_executes_supported_numeric_trace() {
        let optimized = OptimizedTrace {
            function_id: 0xabc,
            loop_header_pc: 5,
            exit_pc: 5,
            guards: Vec::new(),
            steps: vec![
                TraceStep::new(
                    5,
                    Instruction::encode_abc(Opcode::Add, 0, 0, 4),
                    TraceStepKind::Arithmetic {
                        dst: 0,
                        op: ArithmeticOp::Add,
                        lhs: TraceOperand::Slot(0),
                        rhs: TraceOperand::Slot(4),
                    },
                ),
                TraceStep::new(
                    6,
                    Instruction::encode_asbx(Opcode::ForLoop, 1, -2),
                    TraceStepKind::ForLoop {
                        base: 1,
                        exit_resume_pc: 7,
                    },
                ),
            ],
            report: OptimizationReport::default(),
            native_supported: true,
        };

        let encoded = X86_64TraceCompiler::compile(&optimized).unwrap();
        let artifact = NativeTraceArtifact::install(encoded).unwrap();
        let mut slots = vec![10.0, 1.0, 4.0, 1.0, 1.0];

        let outcome = artifact.execute(&mut slots);

        assert_eq!(outcome, NativeTraceOutcome::ContinueLoop);
        assert_eq!(slots[0], 11.0);
        assert_eq!(slots[1], 2.0);
        assert_eq!(slots[4], 2.0);
    }
}
