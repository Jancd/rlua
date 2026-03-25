use crate::opcode::Opcode;

// Lua 5.1 instruction bit layout (32 bits total):
//   Bits  0-5:  opcode (6 bits, 0..63)
//   Bits  6-13: A field (8 bits, 0..255)
//   Bits 14-22: C field (9 bits, 0..511)
//   Bits 23-31: B field (9 bits, 0..511)
//
// iABx format: Bx = bits 14..31 (18 bits, 0..262143)
// iAsBx format: sBx = Bx - MAXARG_SBX (signed)

pub const MAXARG_BX: u32 = (1 << 18) - 1; // 262143
pub const MAXARG_SBX: i32 = (MAXARG_BX >> 1) as i32; // 131071
pub const MAXARG_A: u8 = 255;
pub const MAXARG_B: u16 = 511;
pub const MAXARG_C: u16 = 511;

const SIZE_OP: u32 = 6;
const SIZE_A: u32 = 8;
const SIZE_C: u32 = 9;
const SIZE_B: u32 = 9;

const POS_OP: u32 = 0;
const POS_A: u32 = POS_OP + SIZE_OP;
const POS_C: u32 = POS_A + SIZE_A;
const POS_B: u32 = POS_C + SIZE_C;

const MASK_OP: u32 = (1 << SIZE_OP) - 1;
const MASK_A: u32 = (1 << SIZE_A) - 1;
const MASK_C: u32 = (1 << SIZE_C) - 1;
const MASK_B: u32 = (1 << SIZE_B) - 1;
const MASK_BX: u32 = (1 << (SIZE_B + SIZE_C)) - 1;

/// Threshold for RK encoding: values >= this are constant indices (K[val - RK_OFFSET]).
pub const RK_OFFSET: u16 = 256;

/// A single Lua 5.1 bytecode instruction encoded as a 32-bit word.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Instruction(pub u32);

impl Instruction {
    // --- Decoders ---

    #[inline]
    pub const fn opcode(self) -> Opcode {
        match Opcode::from_u8((self.0 >> POS_OP & MASK_OP) as u8) {
            Some(op) => op,
            None => Opcode::Nop, // fallback for unknown opcodes
        }
    }

    #[inline]
    pub const fn a(self) -> u8 {
        ((self.0 >> POS_A) & MASK_A) as u8
    }

    #[inline]
    pub const fn b(self) -> u16 {
        ((self.0 >> POS_B) & MASK_B) as u16
    }

    #[inline]
    pub const fn c(self) -> u16 {
        ((self.0 >> POS_C) & MASK_C) as u16
    }

    #[inline]
    pub const fn bx(self) -> u32 {
        (self.0 >> POS_C) & MASK_BX
    }

    #[inline]
    pub const fn sbx(self) -> i32 {
        self.bx() as i32 - MAXARG_SBX
    }

    // --- Encoders ---

    pub const fn encode_abc(op: Opcode, a: u8, b: u16, c: u16) -> Self {
        let raw = ((op as u32) << POS_OP)
            | ((a as u32) << POS_A)
            | ((b as u32 & MASK_B) << POS_B)
            | ((c as u32 & MASK_C) << POS_C);
        Self(raw)
    }

    pub const fn encode_abx(op: Opcode, a: u8, bx: u32) -> Self {
        let raw = ((op as u32) << POS_OP) | ((a as u32) << POS_A) | ((bx & MASK_BX) << POS_C);
        Self(raw)
    }

    pub const fn encode_asbx(op: Opcode, a: u8, sbx: i32) -> Self {
        let bx = (sbx + MAXARG_SBX) as u32;
        Self::encode_abx(op, a, bx)
    }

    // --- Convenience constructors for backward compatibility ---

    pub const fn nop() -> Self {
        Self::encode_abc(Opcode::Nop, 0, 0, 0)
    }

    pub const fn halt() -> Self {
        Self::encode_abc(Opcode::Halt, 0, 0, 0)
    }

    /// Check if a B or C field value represents a constant index (RK encoding).
    #[inline]
    pub const fn is_constant(val: u16) -> bool {
        val >= RK_OFFSET
    }

    /// Convert a constant pool index to an RK value.
    #[inline]
    pub const fn rk_constant(k: u16) -> u16 {
        k | (RK_OFFSET)
    }
}

impl std::fmt::Debug for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Instruction({:?} A={} B={} C={} Bx={} sBx={})",
            self.opcode(),
            self.a(),
            self.b(),
            self.c(),
            self.bx(),
            self.sbx()
        )
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Chunk {
    code: Vec<Instruction>,
}

impl Chunk {
    pub fn new() -> Self {
        Self { code: Vec::new() }
    }

    pub fn with_code(code: Vec<Instruction>) -> Self {
        Self { code }
    }

    pub fn push(&mut self, instruction: Instruction) {
        self.code.push(instruction);
    }

    pub fn instructions(&self) -> &[Instruction] {
        &self.code
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::opcode::Opcode;

    #[test]
    fn abc_roundtrip() {
        let instr = Instruction::encode_abc(Opcode::Move, 10, 200, 300);
        assert_eq!(instr.opcode(), Opcode::Move);
        assert_eq!(instr.a(), 10);
        assert_eq!(instr.b(), 200);
        assert_eq!(instr.c(), 300);
    }

    #[test]
    fn abx_roundtrip() {
        let instr = Instruction::encode_abx(Opcode::LoadK, 42, 100_000);
        assert_eq!(instr.opcode(), Opcode::LoadK);
        assert_eq!(instr.a(), 42);
        assert_eq!(instr.bx(), 100_000);
    }

    #[test]
    fn asbx_roundtrip_positive() {
        let instr = Instruction::encode_asbx(Opcode::Jmp, 0, 500);
        assert_eq!(instr.opcode(), Opcode::Jmp);
        assert_eq!(instr.sbx(), 500);
    }

    #[test]
    fn asbx_roundtrip_negative() {
        let instr = Instruction::encode_asbx(Opcode::ForLoop, 3, -100);
        assert_eq!(instr.opcode(), Opcode::ForLoop);
        assert_eq!(instr.a(), 3);
        assert_eq!(instr.sbx(), -100);
    }

    #[test]
    fn asbx_roundtrip_zero() {
        let instr = Instruction::encode_asbx(Opcode::Jmp, 0, 0);
        assert_eq!(instr.sbx(), 0);
    }

    #[test]
    fn boundary_values() {
        let instr = Instruction::encode_abc(Opcode::Move, MAXARG_A, MAXARG_B, MAXARG_C);
        assert_eq!(instr.a(), MAXARG_A);
        assert_eq!(instr.b(), MAXARG_B);
        assert_eq!(instr.c(), MAXARG_C);

        let instr = Instruction::encode_abx(Opcode::LoadK, MAXARG_A, MAXARG_BX);
        assert_eq!(instr.a(), MAXARG_A);
        assert_eq!(instr.bx(), MAXARG_BX);

        let instr = Instruction::encode_asbx(Opcode::Jmp, 0, MAXARG_SBX);
        assert_eq!(instr.sbx(), MAXARG_SBX);

        let instr = Instruction::encode_asbx(Opcode::Jmp, 0, -MAXARG_SBX);
        assert_eq!(instr.sbx(), -MAXARG_SBX);
    }

    #[test]
    fn nop_and_halt_backward_compat() {
        let nop = Instruction::nop();
        assert_eq!(nop.opcode(), Opcode::Nop);
        assert_eq!(nop.a(), 0);

        let halt = Instruction::halt();
        assert_eq!(halt.opcode(), Opcode::Halt);
        assert_eq!(halt.a(), 0);
    }

    #[test]
    fn rk_encoding() {
        assert!(!Instruction::is_constant(0));
        assert!(!Instruction::is_constant(255));
        assert!(Instruction::is_constant(256));
        assert!(Instruction::is_constant(511));
        assert_eq!(Instruction::rk_constant(0), 256);
        assert_eq!(Instruction::rk_constant(5), 261);
    }
}
