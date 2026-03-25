#[macro_export]
macro_rules! define_opcodes {
    ($( $name:ident = $value:expr ),+ $(,)?) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        #[repr(u8)]
        pub enum Opcode {
            $( $name = $value, )+
        }

        impl Opcode {
            pub const fn name(self) -> &'static str {
                match self {
                    $( Self::$name => stringify!($name), )+
                }
            }

            pub const fn all() -> &'static [Self] {
                &[
                    $( Self::$name, )+
                ]
            }

            pub const fn from_u8(raw: u8) -> Option<Self> {
                match raw {
                    $( $value => Some(Self::$name), )+
                    _ => None,
                }
            }
        }
    };
}

// Lua 5.1 canonical opcode numbering (0-37) plus internal opcodes (38-39)
define_opcodes! {
    Move = 0,
    LoadK = 1,
    LoadBool = 2,
    LoadNil = 3,
    GetUpval = 4,
    GetGlobal = 5,
    GetTable = 6,
    SetGlobal = 7,
    SetUpval = 8,
    SetTable = 9,
    NewTable = 10,
    OpSelf = 11,
    Add = 12,
    Sub = 13,
    Mul = 14,
    Div = 15,
    Mod = 16,
    Pow = 17,
    Unm = 18,
    Not = 19,
    Len = 20,
    Concat = 21,
    Jmp = 22,
    Eq = 23,
    Lt = 24,
    Le = 25,
    Test = 26,
    TestSet = 27,
    Call = 28,
    TailCall = 29,
    Return = 30,
    ForLoop = 31,
    ForPrep = 32,
    TForLoop = 33,
    SetList = 34,
    Close = 35,
    Closure = 36,
    Vararg = 37,
    // Internal opcodes (not part of Lua 5.1 spec)
    Nop = 38,
    Halt = 39,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpFormat {
    ABC,
    ABx,
    AsBx,
}

impl Opcode {
    pub const fn format(self) -> OpFormat {
        match self {
            Self::LoadK | Self::GetGlobal | Self::SetGlobal | Self::Closure => OpFormat::ABx,
            Self::Jmp | Self::ForLoop | Self::ForPrep => OpFormat::AsBx,
            _ => OpFormat::ABC,
        }
    }

    /// Returns true for opcodes that perform a conditional test and skip the next instruction.
    pub const fn is_test(self) -> bool {
        matches!(
            self,
            Self::Eq | Self::Lt | Self::Le | Self::Test | Self::TestSet
        )
    }

    /// Returns true if this opcode writes to register A.
    pub const fn sets_register_a(self) -> bool {
        matches!(
            self,
            Self::Move
                | Self::LoadK
                | Self::LoadBool
                | Self::LoadNil
                | Self::GetUpval
                | Self::GetGlobal
                | Self::GetTable
                | Self::NewTable
                | Self::OpSelf
                | Self::Add
                | Self::Sub
                | Self::Mul
                | Self::Div
                | Self::Mod
                | Self::Pow
                | Self::Unm
                | Self::Not
                | Self::Len
                | Self::Concat
                | Self::TestSet
                | Self::Closure
                | Self::Vararg
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{OpFormat, Opcode};

    #[test]
    fn opcode_roundtrip_and_names() {
        for opcode in Opcode::all() {
            let roundtrip = Opcode::from_u8(*opcode as u8);
            assert_eq!(roundtrip, Some(*opcode));
            assert!(!opcode.name().is_empty());
        }
    }

    #[test]
    fn all_lua51_opcodes_present() {
        // Lua 5.1 has opcodes 0..37, plus our internal 38..39
        assert_eq!(Opcode::all().len(), 40);
        assert_eq!(Opcode::Move as u8, 0);
        assert_eq!(Opcode::Vararg as u8, 37);
        assert_eq!(Opcode::Nop as u8, 38);
        assert_eq!(Opcode::Halt as u8, 39);
    }

    #[test]
    fn format_classification() {
        assert_eq!(Opcode::Add.format(), OpFormat::ABC);
        assert_eq!(Opcode::LoadK.format(), OpFormat::ABx);
        assert_eq!(Opcode::Jmp.format(), OpFormat::AsBx);
        assert_eq!(Opcode::ForLoop.format(), OpFormat::AsBx);
        assert_eq!(Opcode::Closure.format(), OpFormat::ABx);
    }

    #[test]
    fn test_opcodes_identified() {
        assert!(Opcode::Eq.is_test());
        assert!(Opcode::Lt.is_test());
        assert!(Opcode::Le.is_test());
        assert!(Opcode::Test.is_test());
        assert!(Opcode::TestSet.is_test());
        assert!(!Opcode::Add.is_test());
        assert!(!Opcode::Jmp.is_test());
    }
}
