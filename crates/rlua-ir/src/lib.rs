use rlua_core::bytecode::Instruction;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trace {
    pub function_id: usize,
    pub loop_header_pc: usize,
    pub exit_pc: usize,
    pub guards: Vec<TraceGuard>,
    pub ops: Vec<IrOp>,
}

impl Trace {
    pub fn new(function_id: usize, loop_header_pc: usize) -> Self {
        Self {
            function_id,
            loop_header_pc,
            exit_pc: loop_header_pc,
            guards: Vec::new(),
            ops: Vec::new(),
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

    pub fn set_exit_pc(&mut self, exit_pc: usize) {
        self.exit_pc = exit_pc;
    }

    pub fn push(&mut self, op: IrOp) {
        #[cfg(feature = "ir-dump")]
        eprintln!("[ir-dump] trace[{}]: {:?}", self.ops.len(), op);

        self.ops.push(op);
    }
}

pub trait TraceOptimizer {
    fn optimize(&self, trace: &mut Trace);
}

#[derive(Debug, Default)]
pub struct NoopOptimizer;

impl TraceOptimizer for NoopOptimizer {
    fn optimize(&self, _trace: &mut Trace) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use rlua_core::opcode::Opcode;

    #[test]
    fn trace_records_instruction_and_guard_metadata() {
        let mut trace = Trace::new(0xfeed, 4);
        trace.push_instruction(4, Instruction::encode_abc(Opcode::Add, 0, 1, 2));
        let guard_id = trace.push_guard(1, ValueType::Number, 4);
        trace.set_exit_pc(7);

        assert_eq!(trace.function_id, 0xfeed);
        assert_eq!(trace.loop_header_pc, 4);
        assert_eq!(trace.exit_pc, 7);
        assert_eq!(guard_id, 0);
        assert_eq!(trace.guards.len(), 1);
        assert_eq!(trace.guards[0].slot, 1);
        assert_eq!(trace.guards[0].expected, ValueType::Number);
        assert_eq!(trace.guards[0].exit.resume_pc, 4);
        assert!(matches!(trace.ops[0], IrOp::Instruction(_)));
        assert!(matches!(trace.ops[1], IrOp::GuardType(_)));
    }
}
