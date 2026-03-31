use std::fmt;
use std::rc::Rc;

use crate::function::LuaFunction;
use crate::table::TableRef;

#[derive(Debug, Default)]
pub struct LuaThread;

pub type ThreadRef = Rc<LuaThread>;

#[derive(Debug, Clone)]
pub enum LuaValue {
    Nil,
    Boolean(bool),
    Number(f64),
    String(Rc<String>),
    Table(TableRef),
    Function(Rc<LuaFunction>),
    Thread(ThreadRef),
}

impl LuaValue {
    pub const fn type_name(&self) -> &'static str {
        match self {
            Self::Nil => "nil",
            Self::Boolean(_) => "boolean",
            Self::Number(_) => "number",
            Self::String(_) => "string",
            Self::Table(_) => "table",
            Self::Function(_) => "function",
            Self::Thread(_) => "thread",
        }
    }

    /// Lua truthiness: everything except nil and false is truthy.
    pub fn is_truthy(&self) -> bool {
        !matches!(self, Self::Nil | Self::Boolean(false))
    }

    /// Attempt to convert the value to a number (Lua string-to-number coercion).
    pub fn to_number(&self) -> Option<f64> {
        match self {
            Self::Number(n) => Some(*n),
            Self::String(s) => {
                let trimmed = s.trim();
                // Handle hex literals (0x or 0X prefix)
                if trimmed.starts_with("0x") || trimmed.starts_with("0X") {
                    u64::from_str_radix(&trimmed[2..], 16)
                        .ok()
                        .map(|n| n as f64)
                } else {
                    trimmed.parse::<f64>().ok()
                }
            }
            _ => None,
        }
    }

    /// Lua `tostring` representation.
    pub fn to_lua_string(&self) -> String {
        match self {
            Self::Nil => "nil".into(),
            Self::Boolean(b) => b.to_string(),
            Self::Number(n) => lua_number_to_string(*n),
            Self::String(s) => (**s).clone(),
            Self::Table(t) => format!("table: {:p}", Rc::as_ptr(t)),
            Self::Function(f) => format!("{f:?}"),
            Self::Thread(thread) => format!("thread: {:p}", Rc::as_ptr(thread)),
        }
    }
}

fn lua_number_to_string(n: f64) -> String {
    if n.is_infinite() {
        if n > 0.0 { "inf".into() } else { "-inf".into() }
    } else if n.is_nan() {
        "-nan".into()
    } else if n == 0.0 && n.is_sign_negative() {
        "0".into()
    } else if n.fract() == 0.0 && n.abs() < i64::MAX as f64 {
        format!("{}", n as i64)
    } else {
        // Lua 5.1 uses C's "%.14g" format.
        // %g uses the shorter of %e and %f, stripping trailing zeros.
        lua_format_g(n, 14)
    }
}

/// Emulate C's `%.*g` format for a float.
fn lua_format_g(n: f64, precision: usize) -> String {
    // %g uses %e if the exponent < -4 or >= precision, else %f.
    // The precision in %g means "significant digits", not decimal places.
    if n == 0.0 {
        return "0".into();
    }
    let exp = n.abs().log10().floor() as i32;
    let s = if exp < -4 || exp >= precision as i32 {
        // Use scientific notation matching C's %e format
        let prec = precision - 1;
        let rust_sci = format!("{n:.prec$e}");
        // Rust outputs "1.23e-5", C outputs "1.23e-05" (min 2-digit exponent with sign)
        // Parse and reformat the exponent part
        if let Some(epos) = rust_sci.find('e') {
            let mantissa = &rust_sci[..epos];
            let exp_str = &rust_sci[epos + 1..];
            let exp_val: i32 = exp_str.parse().unwrap_or(0);
            format!(
                "{mantissa}e{}{:02}",
                if exp_val >= 0 { "+" } else { "-" },
                exp_val.unsigned_abs()
            )
        } else {
            rust_sci
        }
    } else {
        // Use fixed-point notation; decimal places = precision - 1 - exp
        let decimal_places = if precision as i32 - 1 - exp > 0 {
            (precision as i32 - 1 - exp) as usize
        } else {
            0
        };
        format!("{n:.decimal_places$}")
    };
    // Strip trailing zeros after decimal point (but not in exponent part)
    if let Some(epos) = s.find('e') {
        let (mantissa, exp_part) = s.split_at(epos);
        if mantissa.contains('.') {
            let trimmed = mantissa.trim_end_matches('0').trim_end_matches('.');
            format!("{trimmed}{exp_part}")
        } else {
            s
        }
    } else if s.contains('.') {
        let s = s.trim_end_matches('0');
        let s = s.trim_end_matches('.');
        s.to_string()
    } else {
        s
    }
}

impl PartialEq for LuaValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Nil, Self::Nil) => true,
            (Self::Boolean(a), Self::Boolean(b)) => a == b,
            (Self::Number(a), Self::Number(b)) => a == b,
            (Self::String(a), Self::String(b)) => a == b,
            (Self::Table(a), Self::Table(b)) => Rc::ptr_eq(a, b),
            (Self::Function(a), Self::Function(b)) => Rc::ptr_eq(a, b),
            (Self::Thread(a), Self::Thread(b)) => Rc::ptr_eq(a, b),
            _ => false,
        }
    }
}

impl fmt::Display for LuaValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_lua_string())
    }
}

impl From<f64> for LuaValue {
    fn from(n: f64) -> Self {
        Self::Number(n)
    }
}

impl From<bool> for LuaValue {
    fn from(b: bool) -> Self {
        Self::Boolean(b)
    }
}

impl From<&str> for LuaValue {
    fn from(s: &str) -> Self {
        Self::String(Rc::new(s.to_owned()))
    }
}

impl From<String> for LuaValue {
    fn from(s: String) -> Self {
        Self::String(Rc::new(s))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truthiness() {
        assert!(!LuaValue::Nil.is_truthy());
        assert!(!LuaValue::Boolean(false).is_truthy());
        assert!(LuaValue::Boolean(true).is_truthy());
        assert!(LuaValue::Number(0.0).is_truthy()); // Lua: 0 is truthy!
        assert!(LuaValue::from("").is_truthy()); // Lua: "" is truthy!
        assert!(LuaValue::Number(42.0).is_truthy());
    }

    #[test]
    #[allow(clippy::approx_constant)]
    fn to_number_coercion() {
        assert_eq!(LuaValue::Number(42.0).to_number(), Some(42.0));
        assert_eq!(LuaValue::from("42").to_number(), Some(42.0));
        assert_eq!(LuaValue::from(" 3.14 ").to_number(), Some(3.14));
        assert_eq!(LuaValue::from("hello").to_number(), None);
        assert_eq!(LuaValue::Nil.to_number(), None);
    }

    #[test]
    fn type_names() {
        assert_eq!(LuaValue::Nil.type_name(), "nil");
        assert_eq!(LuaValue::Boolean(true).type_name(), "boolean");
        assert_eq!(LuaValue::Number(1.0).type_name(), "number");
        assert_eq!(LuaValue::from("s").type_name(), "string");
        assert_eq!(LuaValue::Thread(Rc::new(LuaThread)).type_name(), "thread");
    }

    #[test]
    #[allow(clippy::approx_constant)]
    fn display_numbers() {
        assert_eq!(format!("{}", LuaValue::Number(42.0)), "42");
        assert_eq!(format!("{}", LuaValue::Number(3.14)), "3.14");
        assert_eq!(format!("{}", LuaValue::Number(0.0)), "0");
    }

    #[test]
    fn equality() {
        assert_eq!(LuaValue::Nil, LuaValue::Nil);
        assert_eq!(LuaValue::Number(1.0), LuaValue::Number(1.0));
        assert_eq!(LuaValue::from("a"), LuaValue::from("a"));
        assert_ne!(LuaValue::Number(1.0), LuaValue::from("1"));
        let thread = Rc::new(LuaThread);
        assert_eq!(LuaValue::Thread(thread.clone()), LuaValue::Thread(thread));
    }
}
