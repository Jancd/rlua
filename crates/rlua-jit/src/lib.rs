use core::fmt;
use std::collections::{HashMap, hash_map::Entry};

use rlua_core::bytecode::Instruction;
use rlua_core::opcode::Opcode;
use rlua_ir::{Trace, ValueType};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct JitStats {
    pub hot_loop_triggers: u64,
    pub record_attempts: u64,
    pub trace_installs: u64,
    pub cache_hits: u64,
    pub replay_entries: u64,
    pub side_exits: u64,
}

#[derive(Debug, Clone)]
pub struct RecordingRequest<'a> {
    pub key: TraceKey,
    pub code: &'a [Instruction],
    pub slot_types: &'a [ValueType],
}

pub trait TraceRecorder {
    fn record(&mut self, request: &RecordingRequest<'_>) -> Result<Trace, JitError>;
}

pub trait CodeGenerator {
    fn compile(&mut self, trace: &Trace) -> Result<(), JitError>;
}

pub trait Deoptimizer {
    fn deopt_resume_pc(&self, guard_id: u32) -> usize;
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
            "[trace-jit] recorded trace function=0x{:x} loop_header_pc={} ops={} guards={}",
            request.key.function,
            request.key.loop_header_pc,
            trace.ops.len(),
            trace.guards.len()
        );

        Ok(trace)
    }
}

#[derive(Debug)]
pub struct JitRuntime {
    config: JitConfig,
    availability: JitAvailability,
    trace_cache: HashMap<TraceKey, Trace>,
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
        self.trace_cache.get(key)
    }

    pub fn lookup_trace(&mut self, key: &TraceKey) -> Option<&Trace> {
        if self.trace_cache.contains_key(key) {
            self.stats.cache_hits += 1;
        }
        self.trace_cache.get(key)
    }

    pub fn record_trace<R: TraceRecorder>(
        &mut self,
        recorder: &mut R,
        request: &RecordingRequest<'_>,
    ) -> Result<bool, JitError> {
        self.stats.record_attempts += 1;

        match self.trace_cache.entry(request.key) {
            Entry::Occupied(_) => Ok(false),
            Entry::Vacant(entry) => {
                let trace = recorder.record(request)?;
                entry.insert(trace);
                self.stats.trace_installs += 1;
                Ok(true)
            }
        }
    }

    pub fn install_trace(&mut self, key: TraceKey, trace: Trace) -> bool {
        match self.trace_cache.entry(key) {
            Entry::Occupied(_) => false,
            Entry::Vacant(entry) => {
                entry.insert(trace);
                self.stats.trace_installs += 1;
                true
            }
        }
    }

    pub fn trace_count(&self) -> usize {
        self.trace_cache.len()
    }

    pub const fn stats(&self) -> JitStats {
        self.stats
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
    }
}

fn push_guard_for_operand(trace: &mut Trace, operand: u16, slot_types: &[ValueType], pc: usize) {
    if Instruction::is_constant(operand) {
        return;
    }

    let slot = operand as usize;
    let expected = slot_types.get(slot).copied().unwrap_or(ValueType::Unknown);
    if expected != ValueType::Unknown {
        trace.push_guard(slot as u16, expected, pc);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recorder_emits_trace_metadata_and_guards() {
        let request = RecordingRequest {
            key: TraceKey::new(0xabc, 0),
            code: &[
                Instruction::encode_abc(Opcode::Add, 0, 1, 2),
                Instruction::encode_asbx(Opcode::Jmp, 0, -2),
            ],
            slot_types: &[ValueType::Number, ValueType::Number, ValueType::Number],
        };

        let mut recorder = LoopTraceRecorder;
        let trace = recorder.record(&request).unwrap();

        assert_eq!(trace.function_id, 0xabc);
        assert_eq!(trace.loop_header_pc, 0);
        assert_eq!(trace.exit_pc, 0);
        assert_eq!(trace.guards.len(), 2);
        assert!(matches!(trace.ops[0], rlua_ir::IrOp::GuardType(_)));
        assert!(matches!(trace.ops[2], rlua_ir::IrOp::Instruction(_)));
    }

    #[test]
    fn runtime_caches_recorded_traces() {
        let request = RecordingRequest {
            key: TraceKey::new(0xabc, 0),
            code: &[
                Instruction::encode_abc(Opcode::Add, 0, 1, 2),
                Instruction::encode_asbx(Opcode::Jmp, 0, -2),
            ],
            slot_types: &[ValueType::Number, ValueType::Number, ValueType::Number],
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
        assert_eq!(runtime.stats().trace_installs, 1);
        assert_eq!(runtime.stats().record_attempts, 2);
    }
}
