use std::fmt::Write;

use crate::bytecode::{Instruction, RK_OFFSET};
use crate::function::FunctionProto;
use crate::opcode::{OpFormat, Opcode};
use crate::value::LuaValue;

/// Disassemble a FunctionProto into a human-readable string (`luac -l` style).
pub fn disassemble(proto: &FunctionProto) -> String {
    let mut out = String::new();
    disassemble_proto(&mut out, proto, 0);
    out
}

fn disassemble_proto(out: &mut String, proto: &FunctionProto, level: usize) {
    // Header
    let kind = if level == 0 { "main" } else { "function" };
    let _ = writeln!(
        out,
        "{kind} <{}:{}> ({} instructions)",
        if proto.source_name.is_empty() {
            "?"
        } else {
            &proto.source_name
        },
        proto.line_defined,
        proto.code.len(),
    );
    let _ = writeln!(
        out,
        "{} params, {} slots, {} upvalues, {} locals, {} constants, {} functions",
        proto.num_params,
        proto.max_stack_size,
        proto.upvalue_descs.len(),
        proto.local_vars.len(),
        proto.constants.len(),
        proto.prototypes.len(),
    );

    // Instructions
    for (i, &instr) in proto.code.iter().enumerate() {
        let pc = i + 1; // 1-based like luac
        let line = proto
            .line_info
            .get(i)
            .copied()
            .map(|l| format!("{l}"))
            .unwrap_or_else(|| "-".to_owned());
        let _ = write!(out, "\t{pc}\t[{line}]\t");
        disassemble_instr(out, instr, &proto.constants);
        let _ = writeln!(out);
    }

    // Constants
    let _ = writeln!(out, "constants ({}):", proto.constants.len());
    for (i, k) in proto.constants.iter().enumerate() {
        let _ = writeln!(out, "\t{}\t{}", i + 1, format_constant(k));
    }

    // Locals
    let _ = writeln!(out, "locals ({}):", proto.local_vars.len());
    for (i, lv) in proto.local_vars.iter().enumerate() {
        let _ = writeln!(
            out,
            "\t{}\t{}\t{}\t{}",
            i,
            lv.name,
            lv.start_pc + 1,
            lv.end_pc + 1
        );
    }

    // Upvalues
    let _ = writeln!(out, "upvalues ({}):", proto.upvalue_descs.len());
    for (i, uv) in proto.upvalue_descs.iter().enumerate() {
        let name = proto.upvalue_names.get(i).map(|s| s.as_str()).unwrap_or("");
        let _ = writeln!(
            out,
            "\t{}\t{}\t{}\t{}",
            i,
            name,
            u8::from(uv.in_stack),
            uv.index
        );
    }

    // Nested prototypes
    for child in &proto.prototypes {
        let _ = writeln!(out);
        disassemble_proto(out, child, level + 1);
    }
}

fn disassemble_instr(out: &mut String, instr: Instruction, constants: &[LuaValue]) {
    let op = instr.opcode();
    let a = instr.a();

    // Opcode name, left-aligned in 10 chars
    let _ = write!(out, "{:<10}", op.name());

    match op.format() {
        OpFormat::ABC => {
            let b = instr.b();
            let c = instr.c();
            match op {
                Opcode::Move => {
                    let _ = write!(out, "{a} {b}");
                }
                Opcode::LoadBool => {
                    let _ = write!(out, "{a} {b} {c}");
                    if c != 0 {
                        let _ = write!(out, "\t; skip next");
                    }
                }
                Opcode::LoadNil => {
                    let _ = write!(out, "{a} {b}");
                }
                Opcode::GetUpval | Opcode::SetUpval => {
                    let _ = write!(out, "{a} {b}");
                }
                Opcode::GetTable => {
                    let _ = write!(out, "{a} {b} {}", rk_display(c, constants));
                }
                Opcode::SetTable => {
                    let _ = write!(
                        out,
                        "{a} {} {}",
                        rk_display(b, constants),
                        rk_display(c, constants)
                    );
                }
                Opcode::NewTable => {
                    let _ = write!(out, "{a} {b} {c}");
                }
                Opcode::OpSelf => {
                    let _ = write!(out, "{a} {b} {}", rk_display(c, constants));
                }
                Opcode::Add
                | Opcode::Sub
                | Opcode::Mul
                | Opcode::Div
                | Opcode::Mod
                | Opcode::Pow => {
                    let _ = write!(
                        out,
                        "{a} {} {}",
                        rk_display(b, constants),
                        rk_display(c, constants)
                    );
                }
                Opcode::Unm | Opcode::Not | Opcode::Len => {
                    let _ = write!(out, "{a} {b}");
                }
                Opcode::Concat => {
                    let _ = write!(out, "{a} {b} {c}");
                }
                Opcode::Eq | Opcode::Lt | Opcode::Le => {
                    let _ = write!(
                        out,
                        "{a} {} {}",
                        rk_display(b, constants),
                        rk_display(c, constants)
                    );
                }
                Opcode::Test => {
                    let _ = write!(out, "{a} {c}");
                }
                Opcode::TestSet => {
                    let _ = write!(out, "{a} {b} {c}");
                }
                Opcode::Call | Opcode::TailCall => {
                    let _ = write!(out, "{a} {b} {c}");
                }
                Opcode::Return => {
                    let _ = write!(out, "{a} {b}");
                }
                Opcode::TForLoop => {
                    let _ = write!(out, "{a} {c}");
                }
                Opcode::SetList => {
                    let _ = write!(out, "{a} {b} {c}");
                }
                Opcode::Close => {
                    let _ = write!(out, "{a}");
                }
                Opcode::Vararg => {
                    let _ = write!(out, "{a} {b}");
                }
                Opcode::Nop | Opcode::Halt => {
                    // no operands
                }
                // Remaining ABC opcodes: generic fallback
                _ => {
                    let _ = write!(out, "{a} {b} {c}");
                }
            }
        }
        OpFormat::ABx => {
            let bx = instr.bx();
            match op {
                Opcode::LoadK => {
                    let _ = write!(out, "{a} {bx}");
                    if let Some(k) = constants.get(bx as usize) {
                        let _ = write!(out, "\t; {}", format_constant(k));
                    }
                }
                Opcode::GetGlobal | Opcode::SetGlobal => {
                    let _ = write!(out, "{a} {bx}");
                    if let Some(k) = constants.get(bx as usize) {
                        let _ = write!(out, "\t; {}", format_constant(k));
                    }
                }
                Opcode::Closure => {
                    let _ = write!(out, "{a} {bx}");
                }
                _ => {
                    let _ = write!(out, "{a} {bx}");
                }
            }
        }
        OpFormat::AsBx => {
            let sbx = instr.sbx();
            match op {
                Opcode::Jmp => {
                    let _ = write!(out, "{sbx}");
                    // Show target PC (1-based)
                    // We don't have PC here, but the caller could add it
                }
                Opcode::ForLoop | Opcode::ForPrep => {
                    let _ = write!(out, "{a} {sbx}");
                }
                _ => {
                    let _ = write!(out, "{a} {sbx}");
                }
            }
        }
    }
}

/// Format an RK value: if it's a constant index, show the constant inline.
fn rk_display(val: u16, constants: &[LuaValue]) -> String {
    if val >= RK_OFFSET {
        let idx = (val - RK_OFFSET) as usize;
        if let Some(k) = constants.get(idx) {
            format!("-{idx}\t; {}", format_constant(k))
        } else {
            format!("-{idx}")
        }
    } else {
        format!("{val}")
    }
}

/// Format a constant for display.
fn format_constant(val: &LuaValue) -> String {
    match val {
        LuaValue::Nil => "nil".to_owned(),
        LuaValue::Boolean(b) => b.to_string(),
        LuaValue::Number(n) => format!("{n}"),
        LuaValue::String(s) => format!("{s:?}"),
        LuaValue::Table(_) => "table".to_owned(),
        LuaValue::Function(_) => "function".to_owned(),
        LuaValue::Thread(_) => "thread".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::Instruction;
    use crate::function::FunctionProto;
    use crate::opcode::Opcode;
    use crate::value::LuaValue;

    fn make_proto(code: Vec<Instruction>, constants: Vec<LuaValue>) -> FunctionProto {
        FunctionProto {
            source_name: "test".to_owned(),
            code,
            constants,
            ..FunctionProto::new()
        }
    }

    #[test]
    fn disasm_loadk() {
        let proto = make_proto(
            vec![Instruction::encode_abx(Opcode::LoadK, 0, 0)],
            vec![LuaValue::Number(42.0)],
        );
        let out = disassemble(&proto);
        assert!(out.contains("LoadK"));
        assert!(out.contains("42"));
    }

    #[test]
    fn disasm_move() {
        let proto = make_proto(vec![Instruction::encode_abc(Opcode::Move, 1, 0, 0)], vec![]);
        let out = disassemble(&proto);
        assert!(out.contains("Move"));
        assert!(out.contains("1 0"));
    }

    #[test]
    fn disasm_add_with_rk() {
        let proto = make_proto(
            vec![Instruction::encode_abc(
                Opcode::Add,
                2,
                0,
                Instruction::rk_constant(0),
            )],
            vec![LuaValue::Number(10.0)],
        );
        let out = disassemble(&proto);
        assert!(out.contains("Add"));
        assert!(out.contains("10"));
    }

    #[test]
    fn disasm_jmp() {
        let proto = make_proto(vec![Instruction::encode_asbx(Opcode::Jmp, 0, 5)], vec![]);
        let out = disassemble(&proto);
        assert!(out.contains("Jmp"));
        assert!(out.contains("5"));
    }

    #[test]
    fn disasm_constants_section() {
        let proto = make_proto(
            vec![Instruction::encode_abx(Opcode::LoadK, 0, 0)],
            vec![LuaValue::from("hello")],
        );
        let out = disassemble(&proto);
        assert!(out.contains("constants (1):"));
        assert!(out.contains("\"hello\""));
    }

    #[test]
    fn disasm_getglobal() {
        let proto = make_proto(
            vec![Instruction::encode_abx(Opcode::GetGlobal, 0, 0)],
            vec![LuaValue::from("print")],
        );
        let out = disassemble(&proto);
        assert!(out.contains("GetGlobal"));
        assert!(out.contains("\"print\""));
    }

    #[test]
    fn disasm_header() {
        let proto = make_proto(vec![], vec![]);
        let out = disassemble(&proto);
        assert!(out.contains("main <test:0>"));
        assert!(out.contains("0 instructions"));
    }

    #[test]
    fn disasm_return() {
        let proto = make_proto(
            vec![Instruction::encode_abc(Opcode::Return, 0, 1, 0)],
            vec![],
        );
        let out = disassemble(&proto);
        assert!(out.contains("Return"));
        assert!(out.contains("0 1"));
    }
}
