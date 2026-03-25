#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueType {
    Unknown,
    Number,
    Boolean,
    String,
    Nil,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IrOp {
    Nop,
    GuardType { slot: u16, expected: ValueType },
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Trace {
    pub ops: Vec<IrOp>,
}

impl Trace {
    pub fn new() -> Self {
        Self { ops: Vec::new() }
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
