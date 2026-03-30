use rlua_ir::{ArithmeticOp, OptimizedTrace, TraceOperand, TraceStepKind};

use crate::JitError;

const ARG_REG_RDI: u8 = 0b111;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EncodedTrace {
    pub(crate) code: Vec<u8>,
    pub(crate) slot_count: usize,
    pub(crate) written_slots: Vec<u16>,
    pub(crate) side_exit_pc: usize,
}

#[derive(Debug, Default)]
pub(crate) struct X86_64TraceCompiler;

impl X86_64TraceCompiler {
    pub(crate) fn compile(trace: &OptimizedTrace) -> Result<EncodedTrace, JitError> {
        let Some((last, body)) = trace.steps.split_last() else {
            return Err(JitError::Codegen(
                "cannot compile an empty optimized trace".to_string(),
            ));
        };

        let TraceStepKind::ForLoop {
            base,
            exit_resume_pc,
        } = last.kind
        else {
            return Err(JitError::UnsupportedTrace(
                "native traces must terminate with a ForLoop back-edge".to_string(),
            ));
        };

        let mut emitter = Emitter::default();

        for step in body {
            match step.kind {
                TraceStepKind::Copy {
                    dst,
                    value: TraceOperand::Slot(src),
                } => emitter.emit_slot_copy(dst, src),
                TraceStepKind::Arithmetic {
                    dst,
                    op,
                    lhs: TraceOperand::Slot(lhs),
                    rhs: TraceOperand::Slot(rhs),
                } if op.supports_m4_native() => emitter.emit_numeric_arithmetic(op, dst, lhs, rhs),
                _ => {
                    return Err(JitError::UnsupportedTrace(format!(
                        "unsupported step for x86_64 backend at pc {}: {:?}",
                        step.source.pc, step.kind
                    )));
                }
            }
        }

        emitter.emit_numeric_for_loop(base);

        Ok(EncodedTrace {
            code: emitter.finish(),
            slot_count: trace.max_slot().map_or(0, |slot| slot as usize + 1),
            written_slots: trace.written_slots(),
            side_exit_pc: exit_resume_pc,
        })
    }
}

#[derive(Debug, Default)]
struct Emitter {
    bytes: Vec<u8>,
}

impl Emitter {
    fn finish(self) -> Vec<u8> {
        self.bytes
    }

    fn emit_numeric_arithmetic(&mut self, op: ArithmeticOp, dst: u16, lhs: u16, rhs: u16) {
        self.emit_movsd_load(0, lhs);
        self.emit_binary_mem(op, 0, rhs);
        self.emit_movsd_store(dst, 0);
    }

    fn emit_slot_copy(&mut self, dst: u16, src: u16) {
        self.emit_movsd_load(0, src);
        self.emit_movsd_store(dst, 0);
    }

    fn emit_numeric_for_loop(&mut self, base: u16) {
        let limit_slot = base + 1;
        let step_slot = base + 2;
        let visible_slot = base + 3;

        self.emit_movsd_load(0, base);
        self.emit_binary_mem(ArithmeticOp::Add, 0, step_slot);

        self.emit_movsd_load(1, step_slot);
        self.emit_xorpd(3, 3);
        self.emit_ucomisd_rr(1, 3);
        let positive_step_jump = self.emit_jcc(0x87);

        self.emit_movsd_load(2, limit_slot);
        self.emit_ucomisd_rr(0, 2);
        let negative_in_range_jump = self.emit_jcc(0x83);
        self.emit_mov_eax_imm32(1);
        self.emit_ret();

        let positive_path = self.position();
        self.patch_rel32(positive_step_jump, positive_path);
        self.emit_movsd_load(2, limit_slot);
        self.emit_ucomisd_rr(0, 2);
        let positive_in_range_jump = self.emit_jcc(0x86);
        self.emit_mov_eax_imm32(1);
        self.emit_ret();

        let in_range = self.position();
        self.patch_rel32(negative_in_range_jump, in_range);
        self.patch_rel32(positive_in_range_jump, in_range);
        self.emit_movsd_store(base, 0);
        self.emit_movsd_store(visible_slot, 0);
        self.emit_xor_eax_eax();
        self.emit_ret();
    }

    fn emit_binary_mem(&mut self, op: ArithmeticOp, dst_xmm: u8, slot: u16) {
        let opcode = match op {
            ArithmeticOp::Add => 0x58,
            ArithmeticOp::Sub => 0x5C,
            ArithmeticOp::Mul => 0x59,
            ArithmeticOp::Div => 0x5E,
            ArithmeticOp::Mod | ArithmeticOp::Pow => unreachable!("filtered before emission"),
        };

        self.bytes.extend_from_slice(&[0xF2, 0x0F, opcode]);
        self.bytes.push(modrm_disp32(dst_xmm, ARG_REG_RDI));
        self.bytes
            .extend_from_slice(&slot_offset(slot).to_le_bytes());
    }

    fn emit_movsd_load(&mut self, dst_xmm: u8, slot: u16) {
        self.bytes.extend_from_slice(&[0xF2, 0x0F, 0x10]);
        self.bytes.push(modrm_disp32(dst_xmm, ARG_REG_RDI));
        self.bytes
            .extend_from_slice(&slot_offset(slot).to_le_bytes());
    }

    fn emit_movsd_store(&mut self, slot: u16, src_xmm: u8) {
        self.bytes.extend_from_slice(&[0xF2, 0x0F, 0x11]);
        self.bytes.push(modrm_disp32(src_xmm, ARG_REG_RDI));
        self.bytes
            .extend_from_slice(&slot_offset(slot).to_le_bytes());
    }

    fn emit_xorpd(&mut self, lhs_xmm: u8, rhs_xmm: u8) {
        self.bytes
            .extend_from_slice(&[0x66, 0x0F, 0x57, modrm_register(lhs_xmm, rhs_xmm)]);
    }

    fn emit_ucomisd_rr(&mut self, lhs_xmm: u8, rhs_xmm: u8) {
        self.bytes
            .extend_from_slice(&[0x66, 0x0F, 0x2E, modrm_register(lhs_xmm, rhs_xmm)]);
    }

    fn emit_mov_eax_imm32(&mut self, value: u32) {
        self.bytes.push(0xB8);
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn emit_xor_eax_eax(&mut self) {
        self.bytes.extend_from_slice(&[0x31, 0xC0]);
    }

    fn emit_ret(&mut self) {
        self.bytes.push(0xC3);
    }

    fn emit_jcc(&mut self, opcode: u8) -> usize {
        self.bytes.extend_from_slice(&[0x0F, opcode, 0, 0, 0, 0]);
        self.bytes.len() - 4
    }

    fn patch_rel32(&mut self, disp_offset: usize, target: usize) {
        let rel = target as isize - (disp_offset as isize + 4);
        let rel = rel as i32;
        self.bytes[disp_offset..disp_offset + 4].copy_from_slice(&rel.to_le_bytes());
    }

    fn position(&self) -> usize {
        self.bytes.len()
    }
}

fn slot_offset(slot: u16) -> i32 {
    i32::from(slot) * 8
}

fn modrm_disp32(reg: u8, rm: u8) -> u8 {
    0b10_000_000 | ((reg & 0b111) << 3) | (rm & 0b111)
}

fn modrm_register(reg: u8, rm: u8) -> u8 {
    0b11_000_000 | ((reg & 0b111) << 3) | (rm & 0b111)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rlua_core::bytecode::Instruction;
    use rlua_core::opcode::Opcode;
    use rlua_ir::{OptimizationReport, TraceInstruction, TraceStep};

    #[test]
    fn compiler_emits_arithmetic_and_loop_control_machine_code() {
        let trace = OptimizedTrace {
            function_id: 0xabc,
            loop_header_pc: 5,
            exit_pc: 5,
            guards: Vec::new(),
            steps: vec![
                TraceStep {
                    source: TraceInstruction {
                        pc: 5,
                        instruction: Instruction::encode_abc(Opcode::Add, 0, 0, 4),
                    },
                    kind: TraceStepKind::Arithmetic {
                        dst: 0,
                        op: ArithmeticOp::Add,
                        lhs: TraceOperand::Slot(0),
                        rhs: TraceOperand::Slot(4),
                    },
                },
                TraceStep {
                    source: TraceInstruction {
                        pc: 6,
                        instruction: Instruction::encode_asbx(Opcode::ForLoop, 1, -2),
                    },
                    kind: TraceStepKind::ForLoop {
                        base: 1,
                        exit_resume_pc: 7,
                    },
                },
            ],
            report: OptimizationReport::default(),
            native_supported: true,
        };

        let encoded = X86_64TraceCompiler::compile(&trace).unwrap();

        assert_eq!(encoded.slot_count, 5);
        assert_eq!(encoded.side_exit_pc, 7);
        assert_eq!(encoded.written_slots, vec![0, 1, 4]);
        assert_eq!(encoded.code.last(), Some(&0xC3));
        assert!(
            encoded
                .code
                .windows(3)
                .any(|window| window == [0xF2, 0x0F, 0x58])
        );
        assert!(
            encoded
                .code
                .windows(3)
                .any(|window| window == [0x66, 0x0F, 0x2E])
        );
    }

    #[test]
    fn compiler_accepts_slot_copy_steps_in_supported_trace() {
        let trace = OptimizedTrace {
            function_id: 0xabc,
            loop_header_pc: 5,
            exit_pc: 5,
            guards: Vec::new(),
            steps: vec![
                TraceStep {
                    source: TraceInstruction {
                        pc: 5,
                        instruction: Instruction::encode_abc(Opcode::Add, 5, 0, 4),
                    },
                    kind: TraceStepKind::Arithmetic {
                        dst: 5,
                        op: ArithmeticOp::Add,
                        lhs: TraceOperand::Slot(0),
                        rhs: TraceOperand::Slot(4),
                    },
                },
                TraceStep {
                    source: TraceInstruction {
                        pc: 6,
                        instruction: Instruction::encode_abc(Opcode::Move, 0, 5, 0),
                    },
                    kind: TraceStepKind::Copy {
                        dst: 0,
                        value: TraceOperand::Slot(5),
                    },
                },
                TraceStep {
                    source: TraceInstruction {
                        pc: 8,
                        instruction: Instruction::encode_asbx(Opcode::ForLoop, 1, -4),
                    },
                    kind: TraceStepKind::ForLoop {
                        base: 1,
                        exit_resume_pc: 9,
                    },
                },
            ],
            report: OptimizationReport::default(),
            native_supported: true,
        };

        let encoded = X86_64TraceCompiler::compile(&trace).unwrap();

        assert_eq!(encoded.slot_count, 6);
        assert_eq!(encoded.written_slots, vec![0, 1, 4, 5]);
        assert_eq!(encoded.side_exit_pc, 9);
        assert_eq!(encoded.code.last(), Some(&0xC3));
    }

    #[test]
    fn compiler_rejects_non_loop_terminated_trace() {
        let trace = OptimizedTrace {
            function_id: 1,
            loop_header_pc: 0,
            exit_pc: 0,
            guards: Vec::new(),
            steps: vec![TraceStep {
                source: TraceInstruction {
                    pc: 0,
                    instruction: Instruction::encode_abc(Opcode::Add, 0, 0, 1),
                },
                kind: TraceStepKind::Arithmetic {
                    dst: 0,
                    op: ArithmeticOp::Add,
                    lhs: TraceOperand::Slot(0),
                    rhs: TraceOperand::Slot(1),
                },
            }],
            report: OptimizationReport::default(),
            native_supported: false,
        };

        assert!(matches!(
            X86_64TraceCompiler::compile(&trace),
            Err(JitError::UnsupportedTrace(_))
        ));
    }
}
