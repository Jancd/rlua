use core::fmt;
use rlua_ir::Trace;

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

#[derive(Debug, Clone, Copy)]
pub struct JitConfig {
    pub enabled: bool,
}

impl Default for JitConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

#[derive(Debug)]
pub enum JitError {
    Unsupported,
}

pub trait TraceRecorder {
    fn record(&mut self, loop_header_pc: usize) -> Trace;
}

pub trait CodeGenerator {
    fn compile(&mut self, trace: &Trace) -> Result<(), JitError>;
}

pub trait Deoptimizer {
    fn deopt_resume_pc(&self, guard_id: u32) -> usize;
}

#[derive(Debug, Clone, Copy)]
pub struct JitRuntime {
    enabled: bool,
    availability: JitAvailability,
}

impl JitRuntime {
    pub fn new(config: JitConfig) -> Self {
        let availability = detect_jit_availability();

        #[cfg(feature = "trace-jit")]
        eprintln!(
            "[trace-jit] JitRuntime init: enabled={}, availability={:?}",
            config.enabled, availability
        );

        Self {
            enabled: config.enabled,
            availability,
        }
    }

    pub const fn execution_mode(self) -> ExecutionMode {
        match (self.enabled, self.availability) {
            (false, _) => ExecutionMode::InterpreterOnly,
            (true, JitAvailability::Available) => ExecutionMode::JitEnabled,
            (true, JitAvailability::UnsupportedArch) => ExecutionMode::JitUnavailable,
        }
    }

    pub const fn is_active(self) -> bool {
        matches!(self.execution_mode(), ExecutionMode::JitEnabled)
    }
}
