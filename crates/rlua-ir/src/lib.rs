use std::collections::{HashMap, HashSet};

use rlua_core::bytecode::Instruction;
use rlua_core::opcode::Opcode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueType {
    Unknown,
    Number,
    Boolean,
    String,
    Nil,
    Table,
    Function,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SideExit {
    pub guard_id: u32,
    pub resume_pc: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceDeoptExitKind {
    Guard { guard_id: u32, slot: u16 },
    SideExit { pc: usize },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceDeoptExit {
    pub kind: TraceDeoptExitKind,
    pub resume_pc: usize,
    pub live_in_slots: Vec<u16>,
    pub materialized_slots: Vec<u16>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TraceGuard {
    pub id: u32,
    pub slot: u16,
    pub expected: ValueType,
    pub exit: SideExit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TraceInstruction {
    pub pc: usize,
    pub instruction: Instruction,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IrOp {
    Instruction(TraceInstruction),
    GuardType(TraceGuard),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArithmeticOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
}

impl ArithmeticOp {
    pub fn from_opcode(opcode: Opcode) -> Option<Self> {
        match opcode {
            Opcode::Add => Some(Self::Add),
            Opcode::Sub => Some(Self::Sub),
            Opcode::Mul => Some(Self::Mul),
            Opcode::Div => Some(Self::Div),
            Opcode::Mod => Some(Self::Mod),
            Opcode::Pow => Some(Self::Pow),
            _ => None,
        }
    }

    pub fn apply(self, lhs: f64, rhs: f64) -> f64 {
        match self {
            Self::Add => lhs + rhs,
            Self::Sub => lhs - rhs,
            Self::Mul => lhs * rhs,
            Self::Div => lhs / rhs,
            Self::Mod => {
                let remainder = lhs % rhs;
                if remainder != 0.0 && (remainder < 0.0) != (rhs < 0.0) {
                    remainder + rhs
                } else {
                    remainder
                }
            }
            Self::Pow => lhs.powf(rhs),
        }
    }

    pub const fn supports_m4_native(self) -> bool {
        matches!(self, Self::Add | Self::Sub | Self::Mul | Self::Div)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConstantValue {
    Number(f64),
    Boolean(bool),
    Nil,
}

impl ConstantValue {
    pub const fn as_number(self) -> Option<f64> {
        match self {
            Self::Number(value) => Some(value),
            Self::Boolean(_) | Self::Nil => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TraceOperand {
    Slot(u16),
    Constant(ConstantValue),
}

impl TraceOperand {
    pub const fn slot(slot: u16) -> Self {
        Self::Slot(slot)
    }

    pub const fn constant(value: ConstantValue) -> Self {
        Self::Constant(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TraceStepKind {
    Copy {
        dst: u16,
        value: TraceOperand,
    },
    Arithmetic {
        dst: u16,
        op: ArithmeticOp,
        lhs: TraceOperand,
        rhs: TraceOperand,
    },
    ForLoop {
        base: u16,
        exit_resume_pc: usize,
    },
    Close {
        from: u16,
    },
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TraceStep {
    pub source: TraceInstruction,
    pub kind: TraceStepKind,
}

impl TraceStep {
    pub const fn new(pc: usize, instruction: Instruction, kind: TraceStepKind) -> Self {
        Self {
            source: TraceInstruction { pc, instruction },
            kind,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Trace {
    pub function_id: usize,
    pub loop_header_pc: usize,
    pub exit_pc: usize,
    pub guards: Vec<TraceGuard>,
    pub ops: Vec<IrOp>,
    pub steps: Vec<TraceStep>,
}

impl Trace {
    pub fn new(function_id: usize, loop_header_pc: usize) -> Self {
        Self {
            function_id,
            loop_header_pc,
            exit_pc: loop_header_pc,
            guards: Vec::new(),
            ops: Vec::new(),
            steps: Vec::new(),
        }
    }

    pub fn push_instruction(&mut self, pc: usize, instruction: Instruction) {
        self.push(IrOp::Instruction(TraceInstruction { pc, instruction }));
    }

    pub fn push_guard(&mut self, slot: u16, expected: ValueType, resume_pc: usize) -> u32 {
        let guard_id = self.guards.len() as u32;
        let guard = TraceGuard {
            id: guard_id,
            slot,
            expected,
            exit: SideExit {
                guard_id,
                resume_pc,
            },
        };
        self.guards.push(guard);
        self.push(IrOp::GuardType(guard));
        guard_id
    }

    pub fn push_step(&mut self, step: TraceStep) {
        #[cfg(feature = "ir-dump")]
        eprintln!("[ir-dump] step[{}]: {:?}", self.steps.len(), step);

        self.steps.push(step);
    }

    pub fn set_exit_pc(&mut self, exit_pc: usize) {
        self.exit_pc = exit_pc;
    }

    pub fn push(&mut self, op: IrOp) {
        #[cfg(feature = "ir-dump")]
        eprintln!("[ir-dump] trace[{}]: {:?}", self.ops.len(), op);

        self.ops.push(op);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct OptimizationReport {
    pub constant_folds: u32,
    pub dead_code_eliminated: u32,
    pub simplified_guards: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OptimizedTrace {
    pub function_id: usize,
    pub loop_header_pc: usize,
    pub exit_pc: usize,
    pub guards: Vec<TraceGuard>,
    pub steps: Vec<TraceStep>,
    pub deopt_exits: Vec<TraceDeoptExit>,
    pub report: OptimizationReport,
    pub native_supported: bool,
}

impl OptimizedTrace {
    pub fn from_trace(trace: &Trace) -> Self {
        Self {
            function_id: trace.function_id,
            loop_header_pc: trace.loop_header_pc,
            exit_pc: trace.exit_pc,
            guards: trace.guards.clone(),
            steps: trace.steps.clone(),
            deopt_exits: Vec::new(),
            report: OptimizationReport::default(),
            native_supported: false,
        }
    }

    pub fn max_slot(&self) -> Option<u16> {
        let mut max_slot = None;

        for step in &self.steps {
            for slot in slots_read_by_step(step) {
                max_slot = Some(max_slot.map_or(slot, |current: u16| current.max(slot)));
            }
            for slot in slots_written_by_step(step) {
                max_slot = Some(max_slot.map_or(slot, |current: u16| current.max(slot)));
            }
        }

        for guard in &self.guards {
            max_slot = Some(max_slot.map_or(guard.slot, |current| current.max(guard.slot)));
        }

        max_slot
    }

    pub fn written_slots(&self) -> Vec<u16> {
        let mut slots = HashSet::new();
        for step in &self.steps {
            slots.extend(slots_written_by_step(step));
        }

        let mut slots: Vec<u16> = slots.into_iter().collect();
        slots.sort_unstable();
        slots
    }

    pub fn read_slots(&self) -> Vec<u16> {
        trace_live_in_slots(&self.guards, &self.steps)
    }

    pub fn guard_deopt_exit(&self, guard_id: u32) -> Option<&TraceDeoptExit> {
        self.deopt_exits.iter().find(|exit| {
            matches!(
                exit.kind,
                TraceDeoptExitKind::Guard {
                    guard_id: candidate,
                    ..
                } if candidate == guard_id
            )
        })
    }

    pub fn side_exit_deopt(&self, pc: usize) -> Option<&TraceDeoptExit> {
        self.deopt_exits.iter().find(|exit| {
            matches!(
                exit.kind,
                TraceDeoptExitKind::SideExit { pc: candidate } if candidate == pc
            )
        })
    }
}

pub trait TraceOptimizer {
    fn optimize(&self, trace: &Trace) -> OptimizedTrace;
}

#[derive(Debug, Default)]
pub struct NoopOptimizer;

impl TraceOptimizer for NoopOptimizer {
    fn optimize(&self, trace: &Trace) -> OptimizedTrace {
        let mut optimized = OptimizedTrace::from_trace(trace);
        optimized.deopt_exits = derive_deopt_exits(&optimized.guards, &optimized.steps);
        optimized.native_supported = is_native_trace_supported(&trace.steps);
        optimized
    }
}

#[derive(Debug, Default)]
pub struct M4TraceOptimizer;

impl TraceOptimizer for M4TraceOptimizer {
    fn optimize(&self, trace: &Trace) -> OptimizedTrace {
        let mut optimized = OptimizedTrace::from_trace(trace);
        let mut report = OptimizationReport::default();

        let (guards, simplified) = simplify_guards(&trace.guards);
        report.simplified_guards = simplified;

        let folded_steps = constant_fold_steps(&trace.steps, &mut report);
        let folded_steps = strip_close_steps(folded_steps);
        let live_out = initial_live_slots(&guards, &folded_steps);
        let steps = eliminate_dead_code(folded_steps, &live_out, &mut report);

        optimized.guards = guards;
        optimized.steps = steps;
        optimized.deopt_exits = derive_deopt_exits(&optimized.guards, &optimized.steps);
        optimized.native_supported = is_native_trace_supported(&optimized.steps);
        optimized.report = report;
        optimized
    }
}

pub fn optimize_trace(trace: &Trace) -> OptimizedTrace {
    M4TraceOptimizer.optimize(trace)
}

fn simplify_guards(guards: &[TraceGuard]) -> (Vec<TraceGuard>, u32) {
    let mut simplified = Vec::with_capacity(guards.len());
    let mut removed = 0;

    for guard in guards {
        let duplicate = simplified.last().is_some_and(|previous: &TraceGuard| {
            previous.slot == guard.slot
                && previous.expected == guard.expected
                && previous.exit.resume_pc == guard.exit.resume_pc
        });

        if duplicate {
            removed += 1;
        } else {
            let mut deduped = *guard;
            deduped.id = simplified.len() as u32;
            deduped.exit.guard_id = deduped.id;
            simplified.push(deduped);
        }
    }

    (simplified, removed)
}

fn constant_fold_steps(steps: &[TraceStep], report: &mut OptimizationReport) -> Vec<TraceStep> {
    let mut constants = HashMap::<u16, ConstantValue>::new();
    let mut folded = Vec::with_capacity(steps.len());

    for step in steps {
        let kind = match step.kind {
            TraceStepKind::Copy { dst, value } => {
                let value = resolve_operand(value, &constants);
                match value {
                    TraceOperand::Constant(constant) => {
                        constants.insert(dst, constant);
                    }
                    TraceOperand::Slot(_) => {
                        constants.remove(&dst);
                    }
                }
                TraceStepKind::Copy { dst, value }
            }
            TraceStepKind::Arithmetic { dst, op, lhs, rhs } => {
                let lhs = resolve_operand(lhs, &constants);
                let rhs = resolve_operand(rhs, &constants);

                if let (TraceOperand::Constant(lhs), TraceOperand::Constant(rhs)) = (lhs, rhs) {
                    if let (Some(lhs), Some(rhs)) = (lhs.as_number(), rhs.as_number()) {
                        report.constant_folds += 1;
                        let value = ConstantValue::Number(op.apply(lhs, rhs));
                        constants.insert(dst, value);
                        TraceStepKind::Copy {
                            dst,
                            value: TraceOperand::Constant(value),
                        }
                    } else {
                        constants.remove(&dst);
                        TraceStepKind::Arithmetic {
                            dst,
                            op,
                            lhs: TraceOperand::Constant(lhs),
                            rhs: TraceOperand::Constant(rhs),
                        }
                    }
                } else {
                    constants.remove(&dst);
                    TraceStepKind::Arithmetic { dst, op, lhs, rhs }
                }
            }
            TraceStepKind::ForLoop {
                base,
                exit_resume_pc,
            } => {
                constants.remove(&base);
                constants.remove(&(base + 3));
                TraceStepKind::ForLoop {
                    base,
                    exit_resume_pc,
                }
            }
            TraceStepKind::Close { from } => TraceStepKind::Close { from },
            TraceStepKind::Unsupported => {
                constants.clear();
                TraceStepKind::Unsupported
            }
        };

        folded.push(TraceStep {
            source: step.source,
            kind,
        });
    }

    folded
}

fn strip_close_steps(steps: Vec<TraceStep>) -> Vec<TraceStep> {
    steps
        .into_iter()
        .filter(|step| !matches!(step.kind, TraceStepKind::Close { .. }))
        .collect()
}

fn resolve_operand(operand: TraceOperand, constants: &HashMap<u16, ConstantValue>) -> TraceOperand {
    match operand {
        TraceOperand::Slot(slot) => constants
            .get(&slot)
            .copied()
            .map_or(TraceOperand::Slot(slot), TraceOperand::Constant),
        TraceOperand::Constant(_) => operand,
    }
}

fn derive_deopt_exits(guards: &[TraceGuard], steps: &[TraceStep]) -> Vec<TraceDeoptExit> {
    let live_in_slots = trace_live_in_slots(guards, steps);
    let mut exits = Vec::new();

    for guard in guards {
        exits.push(TraceDeoptExit {
            kind: TraceDeoptExitKind::Guard {
                guard_id: guard.id,
                slot: guard.slot,
            },
            resume_pc: guard.exit.resume_pc,
            live_in_slots: live_in_slots.clone(),
            materialized_slots: Vec::new(),
        });
    }

    let mut materialized = HashSet::new();
    for step in steps {
        if let TraceStepKind::ForLoop { exit_resume_pc, .. } = step.kind {
            exits.push(TraceDeoptExit {
                kind: TraceDeoptExitKind::SideExit { pc: step.source.pc },
                resume_pc: exit_resume_pc,
                live_in_slots: live_in_slots.clone(),
                materialized_slots: sorted_slots(materialized.iter().copied().collect()),
            });
        }

        materialized.extend(slots_written_by_step(step));
    }

    exits
}

fn trace_live_in_slots(guards: &[TraceGuard], steps: &[TraceStep]) -> Vec<u16> {
    let mut slots = HashSet::new();
    let mut defined = HashSet::new();

    for guard in guards {
        slots.insert(guard.slot);
    }

    for step in steps {
        for slot in slots_read_by_step(step) {
            if !defined.contains(&slot) {
                slots.insert(slot);
            }
        }
        defined.extend(slots_written_by_step(step));
    }

    sorted_slots(slots)
}

fn sorted_slots(slots: HashSet<u16>) -> Vec<u16> {
    let mut slots: Vec<u16> = slots.into_iter().collect();
    slots.sort_unstable();
    slots
}

fn initial_live_slots(guards: &[TraceGuard], steps: &[TraceStep]) -> HashSet<u16> {
    let mut live = HashSet::new();

    for guard in guards {
        live.insert(guard.slot);
    }

    for step in steps {
        live.extend(slots_read_by_step(step));
    }

    live
}

fn eliminate_dead_code(
    steps: Vec<TraceStep>,
    live_out: &HashSet<u16>,
    report: &mut OptimizationReport,
) -> Vec<TraceStep> {
    let all_slots = all_touched_slots(&steps);
    let mut live = live_out.clone();
    let mut kept = Vec::with_capacity(steps.len());

    for step in steps.into_iter().rev() {
        match step.kind {
            TraceStepKind::Copy { dst, value } => {
                if live.contains(&dst) {
                    live.remove(&dst);
                    if let TraceOperand::Slot(slot) = value {
                        live.insert(slot);
                    }
                    kept.push(step);
                } else {
                    report.dead_code_eliminated += 1;
                }
            }
            TraceStepKind::Arithmetic { dst, lhs, rhs, .. } => {
                if live.contains(&dst) {
                    live.remove(&dst);
                    if let TraceOperand::Slot(slot) = lhs {
                        live.insert(slot);
                    }
                    if let TraceOperand::Slot(slot) = rhs {
                        live.insert(slot);
                    }
                    kept.push(step);
                } else {
                    report.dead_code_eliminated += 1;
                }
            }
            TraceStepKind::ForLoop { base, .. } => {
                live.remove(&base);
                live.remove(&(base + 3));
                live.insert(base);
                live.insert(base + 1);
                live.insert(base + 2);
                kept.push(step);
            }
            TraceStepKind::Close { .. } => {
                kept.push(step);
            }
            TraceStepKind::Unsupported => {
                live.extend(all_slots.iter().copied());
                kept.push(step);
            }
        }
    }

    kept.reverse();
    kept
}

fn all_touched_slots(steps: &[TraceStep]) -> HashSet<u16> {
    let mut slots = HashSet::new();
    for step in steps {
        slots.extend(slots_read_by_step(step));
        slots.extend(slots_written_by_step(step));
    }
    slots
}

fn slots_read_by_step(step: &TraceStep) -> Vec<u16> {
    match step.kind {
        TraceStepKind::Copy { value, .. } => operand_slots(value).into_iter().collect(),
        TraceStepKind::Arithmetic { lhs, rhs, .. } => {
            let mut slots = operand_slots(lhs);
            slots.extend(operand_slots(rhs));
            let mut slots: Vec<u16> = slots.into_iter().collect();
            slots.sort_unstable();
            slots
        }
        TraceStepKind::ForLoop { base, .. } => vec![base, base + 1, base + 2],
        TraceStepKind::Close { .. } => Vec::new(),
        TraceStepKind::Unsupported => Vec::new(),
    }
}

fn slots_written_by_step(step: &TraceStep) -> Vec<u16> {
    match step.kind {
        TraceStepKind::Copy { dst, .. } | TraceStepKind::Arithmetic { dst, .. } => vec![dst],
        TraceStepKind::ForLoop { base, .. } => vec![base, base + 3],
        TraceStepKind::Close { .. } => Vec::new(),
        TraceStepKind::Unsupported => Vec::new(),
    }
}

fn operand_slots(operand: TraceOperand) -> HashSet<u16> {
    let mut slots = HashSet::new();
    if let TraceOperand::Slot(slot) = operand {
        slots.insert(slot);
    }
    slots
}

fn is_native_step_supported(step: &TraceStep) -> bool {
    match step.kind {
        TraceStepKind::Copy {
            value: TraceOperand::Slot(_),
            ..
        } => true,
        TraceStepKind::Copy { .. } => false,
        TraceStepKind::Arithmetic { op, lhs, rhs, .. } => {
            op.supports_m4_native()
                && matches!(lhs, TraceOperand::Slot(_))
                && matches!(rhs, TraceOperand::Slot(_))
        }
        TraceStepKind::ForLoop { .. } => true,
        TraceStepKind::Close { .. } => false,
        TraceStepKind::Unsupported => false,
    }
}

fn is_native_trace_supported(steps: &[TraceStep]) -> bool {
    let Some((last, prefix)) = steps.split_last() else {
        return false;
    };

    matches!(last.kind, TraceStepKind::ForLoop { .. })
        && prefix.iter().all(|step| {
            !matches!(step.kind, TraceStepKind::ForLoop { .. }) && is_native_step_supported(step)
        })
        && is_native_step_supported(last)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq)]
    #[allow(dead_code)]
    enum SlotValue {
        Nil,
        Boolean(bool),
        Number(f64),
        Opaque,
    }

    impl SlotValue {
        fn from_constant(value: ConstantValue) -> Self {
            match value {
                ConstantValue::Nil => Self::Nil,
                ConstantValue::Boolean(value) => Self::Boolean(value),
                ConstantValue::Number(value) => Self::Number(value),
            }
        }

        fn value_type(self) -> ValueType {
            match self {
                Self::Nil => ValueType::Nil,
                Self::Boolean(_) => ValueType::Boolean,
                Self::Number(_) => ValueType::Number,
                Self::Opaque => ValueType::Table,
            }
        }

        fn as_number(self) -> Option<f64> {
            match self {
                Self::Number(value) => Some(value),
                Self::Nil | Self::Boolean(_) | Self::Opaque => None,
            }
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    enum EvalResult {
        Completed(Vec<SlotValue>),
        SideExit(usize, Vec<SlotValue>),
    }

    #[test]
    fn trace_records_instruction_guard_and_step_metadata() {
        let mut trace = Trace::new(0xfeed, 4);
        trace.push_instruction(4, Instruction::encode_abc(Opcode::Add, 0, 1, 2));
        let guard_id = trace.push_guard(1, ValueType::Number, 4);
        trace.push_step(TraceStep::new(
            4,
            Instruction::encode_abc(Opcode::Add, 0, 1, 2),
            TraceStepKind::Arithmetic {
                dst: 0,
                op: ArithmeticOp::Add,
                lhs: TraceOperand::slot(1),
                rhs: TraceOperand::slot(2),
            },
        ));
        trace.set_exit_pc(7);

        assert_eq!(trace.function_id, 0xfeed);
        assert_eq!(trace.loop_header_pc, 4);
        assert_eq!(trace.exit_pc, 7);
        assert_eq!(guard_id, 0);
        assert_eq!(trace.guards.len(), 1);
        assert_eq!(trace.steps.len(), 1);
        assert_eq!(trace.guards[0].slot, 1);
        assert_eq!(trace.guards[0].expected, ValueType::Number);
        assert_eq!(trace.guards[0].exit.resume_pc, 4);
        assert!(matches!(trace.ops[0], IrOp::Instruction(_)));
        assert!(matches!(trace.ops[1], IrOp::GuardType(_)));
    }

    #[test]
    fn optimizer_constant_folds_without_changing_semantics() {
        let trace = make_arithmetic_trace();
        let original = execute_steps(&trace.guards, &trace.steps, &[SlotValue::Number(4.0)]);
        let optimized = optimize_trace(&trace);
        let lowered = execute_steps(
            &optimized.guards,
            &optimized.steps,
            &[SlotValue::Number(4.0)],
        );

        assert_eq!(slot_zero(&original), slot_zero(&lowered));
        assert_eq!(optimized.report.constant_folds, 1);
        assert!(matches!(
            optimized.steps[0].kind,
            TraceStepKind::Arithmetic {
                dst: 0,
                op: ArithmeticOp::Add,
                lhs: TraceOperand::Slot(0),
                rhs: TraceOperand::Constant(ConstantValue::Number(3.0))
            }
        ));
    }

    #[test]
    fn optimizer_eliminates_dead_pure_ops() {
        let mut trace = Trace::new(1, 0);
        trace.push_step(TraceStep::new(
            0,
            Instruction::encode_abc(Opcode::Add, 2, 0, 1),
            TraceStepKind::Arithmetic {
                dst: 2,
                op: ArithmeticOp::Add,
                lhs: TraceOperand::slot(0),
                rhs: TraceOperand::slot(1),
            },
        ));
        trace.push_step(TraceStep::new(
            1,
            Instruction::encode_abc(Opcode::Move, 0, 0, 0),
            TraceStepKind::Copy {
                dst: 0,
                value: TraceOperand::slot(0),
            },
        ));

        let optimized = optimize_trace(&trace);

        assert_eq!(optimized.report.dead_code_eliminated, 1);
        assert_eq!(optimized.steps.len(), 1);
        assert!(matches!(
            optimized.steps[0].kind,
            TraceStepKind::Copy { .. }
        ));
    }

    #[test]
    fn optimizer_preserves_unsupported_regions_safely() {
        let mut trace = Trace::new(1, 0);
        trace.push_step(TraceStep::new(
            0,
            Instruction::encode_abc(Opcode::Call, 0, 1, 1),
            TraceStepKind::Unsupported,
        ));
        trace.push_step(TraceStep::new(
            1,
            Instruction::encode_abc(Opcode::Add, 0, 0, 1),
            TraceStepKind::Arithmetic {
                dst: 0,
                op: ArithmeticOp::Add,
                lhs: TraceOperand::slot(0),
                rhs: TraceOperand::slot(1),
            },
        ));

        let optimized = optimize_trace(&trace);

        assert!(matches!(
            optimized.steps[0].kind,
            TraceStepKind::Unsupported
        ));
        assert!(!optimized.native_supported);
        assert_eq!(optimized.report.dead_code_eliminated, 0);
    }

    #[test]
    fn optimizer_simplifies_duplicate_guards() {
        let mut trace = Trace::new(1, 0);
        trace.push_guard(0, ValueType::Number, 4);
        trace.push_guard(0, ValueType::Number, 4);

        let optimized = optimize_trace(&trace);

        assert_eq!(optimized.guards.len(), 1);
        assert_eq!(optimized.guards[0].id, 0);
        assert_eq!(optimized.report.simplified_guards, 1);
    }

    #[test]
    fn optimizer_derives_explicit_deopt_exits_for_guards_and_loop_exit() {
        let mut trace = Trace::new(1, 5);
        let guard_id = trace.push_guard(0, ValueType::Number, 5);
        trace.push_step(TraceStep::new(
            5,
            Instruction::encode_abc(Opcode::Add, 5, 0, 4),
            TraceStepKind::Arithmetic {
                dst: 5,
                op: ArithmeticOp::Add,
                lhs: TraceOperand::slot(0),
                rhs: TraceOperand::slot(4),
            },
        ));
        trace.push_step(TraceStep::new(
            6,
            Instruction::encode_abc(Opcode::Move, 0, 5, 0),
            TraceStepKind::Copy {
                dst: 0,
                value: TraceOperand::slot(5),
            },
        ));
        trace.push_step(TraceStep::new(
            7,
            Instruction::encode_asbx(Opcode::ForLoop, 1, -3),
            TraceStepKind::ForLoop {
                base: 1,
                exit_resume_pc: 8,
            },
        ));

        let optimized = optimize_trace(&trace);
        let guard_exit = optimized.guard_deopt_exit(guard_id).unwrap();
        let side_exit = optimized.side_exit_deopt(7).unwrap();

        assert_eq!(optimized.deopt_exits.len(), 2);
        assert_eq!(guard_exit.resume_pc, 5);
        assert_eq!(guard_exit.live_in_slots, vec![0, 1, 2, 3, 4]);
        assert!(guard_exit.materialized_slots.is_empty());
        assert_eq!(side_exit.resume_pc, 8);
        assert_eq!(side_exit.live_in_slots, vec![0, 1, 2, 3, 4]);
        assert_eq!(side_exit.materialized_slots, vec![0, 5]);
        assert!(matches!(
            guard_exit.kind,
            TraceDeoptExitKind::Guard {
                guard_id: 0,
                slot: 0
            }
        ));
        assert!(matches!(
            side_exit.kind,
            TraceDeoptExitKind::SideExit { pc: 7 }
        ));
    }

    #[test]
    fn optimizer_removes_close_and_preserves_live_inputs_for_numeric_loop_shape() {
        let mut trace = Trace::new(1, 5);
        trace.push_step(TraceStep::new(
            5,
            Instruction::encode_abc(Opcode::Add, 5, 0, 4),
            TraceStepKind::Arithmetic {
                dst: 5,
                op: ArithmeticOp::Add,
                lhs: TraceOperand::slot(0),
                rhs: TraceOperand::slot(4),
            },
        ));
        trace.push_step(TraceStep::new(
            6,
            Instruction::encode_abc(Opcode::Move, 0, 5, 0),
            TraceStepKind::Copy {
                dst: 0,
                value: TraceOperand::slot(5),
            },
        ));
        trace.push_step(TraceStep::new(
            7,
            Instruction::encode_abc(Opcode::Close, 4, 0, 0),
            TraceStepKind::Close { from: 4 },
        ));
        trace.push_step(TraceStep::new(
            8,
            Instruction::encode_asbx(Opcode::ForLoop, 1, -4),
            TraceStepKind::ForLoop {
                base: 1,
                exit_resume_pc: 9,
            },
        ));

        let initial = [
            SlotValue::Number(0.0),
            SlotValue::Number(1.0),
            SlotValue::Number(8.0),
            SlotValue::Number(1.0),
            SlotValue::Number(1.0),
        ];
        let original = execute_steps(&trace.guards, &trace.steps, &initial);
        let optimized = optimize_trace(&trace);
        let lowered = execute_steps(&optimized.guards, &optimized.steps, &initial);

        assert_eq!(original, lowered);
        assert!(optimized.native_supported);
        assert_eq!(optimized.read_slots(), vec![0, 1, 2, 3, 4]);
        assert!(
            optimized
                .steps
                .iter()
                .all(|step| !matches!(step.kind, TraceStepKind::Close { .. }))
        );
    }

    fn make_arithmetic_trace() -> Trace {
        let mut trace = Trace::new(0x1, 0);
        trace.push_guard(0, ValueType::Number, 0);
        trace.push_step(TraceStep::new(
            0,
            Instruction::encode_abc(Opcode::Add, 1, 2, 3),
            TraceStepKind::Arithmetic {
                dst: 1,
                op: ArithmeticOp::Add,
                lhs: TraceOperand::constant(ConstantValue::Number(1.0)),
                rhs: TraceOperand::constant(ConstantValue::Number(2.0)),
            },
        ));
        trace.push_step(TraceStep::new(
            1,
            Instruction::encode_abc(Opcode::Add, 0, 0, 1),
            TraceStepKind::Arithmetic {
                dst: 0,
                op: ArithmeticOp::Add,
                lhs: TraceOperand::slot(0),
                rhs: TraceOperand::slot(1),
            },
        ));
        trace
    }

    fn execute_steps(
        guards: &[TraceGuard],
        steps: &[TraceStep],
        initial: &[SlotValue],
    ) -> EvalResult {
        let mut slots = initial.to_vec();
        ensure_slot_capacity(steps, &mut slots);

        for guard in guards {
            let actual = slots
                .get(guard.slot as usize)
                .copied()
                .unwrap_or(SlotValue::Nil)
                .value_type();
            if actual != guard.expected {
                return EvalResult::SideExit(guard.exit.resume_pc, slots);
            }
        }

        for step in steps {
            match step.kind {
                TraceStepKind::Copy { dst, value } => {
                    let value = operand_value(value, &slots);
                    slots[dst as usize] = value;
                }
                TraceStepKind::Arithmetic { dst, op, lhs, rhs } => {
                    let lhs = operand_value(lhs, &slots).as_number();
                    let rhs = operand_value(rhs, &slots).as_number();
                    let (Some(lhs), Some(rhs)) = (lhs, rhs) else {
                        return EvalResult::SideExit(step.source.pc, slots);
                    };
                    slots[dst as usize] = SlotValue::Number(op.apply(lhs, rhs));
                }
                TraceStepKind::ForLoop {
                    base,
                    exit_resume_pc,
                } => {
                    let index = slots[base as usize].as_number().unwrap();
                    let limit = slots[base as usize + 1].as_number().unwrap();
                    let step_value = slots[base as usize + 2].as_number().unwrap();
                    let new_index = index + step_value;
                    let in_range = if step_value > 0.0 {
                        new_index <= limit
                    } else {
                        new_index >= limit
                    };

                    if in_range {
                        slots[base as usize] = SlotValue::Number(new_index);
                        slots[base as usize + 3] = SlotValue::Number(new_index);
                    } else {
                        return EvalResult::SideExit(exit_resume_pc, slots);
                    }
                }
                TraceStepKind::Close { .. } => {}
                TraceStepKind::Unsupported => {}
            }
        }

        EvalResult::Completed(slots)
    }

    fn ensure_slot_capacity(steps: &[TraceStep], slots: &mut Vec<SlotValue>) {
        let max_slot = steps
            .iter()
            .flat_map(|step| {
                let mut touched = slots_read_by_step(step);
                touched.extend(slots_written_by_step(step));
                touched
            })
            .max()
            .unwrap_or(0);

        if slots.len() <= max_slot as usize {
            slots.resize(max_slot as usize + 1, SlotValue::Nil);
        }
    }

    fn operand_value(operand: TraceOperand, slots: &[SlotValue]) -> SlotValue {
        match operand {
            TraceOperand::Slot(slot) => slots.get(slot as usize).copied().unwrap_or(SlotValue::Nil),
            TraceOperand::Constant(value) => SlotValue::from_constant(value),
        }
    }

    fn slot_zero(result: &EvalResult) -> Option<SlotValue> {
        match result {
            EvalResult::Completed(slots) | EvalResult::SideExit(_, slots) => slots.first().copied(),
        }
    }
}
