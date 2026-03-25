use std::fmt;
use std::rc::Rc;

use crate::function::LuaFunction;
use crate::table::TableRef;

#[derive(Debug, Clone)]
pub enum LuaValue {
    Nil,
    Boolean(bool),
    Number(f64),
    String(Rc<String>),
    Table(TableRef),
    Function(Rc<LuaFunction>),
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
            Self::String(s) => s.trim().parse::<f64>().ok(),
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
        // Lua uses C's "%.14g" format. Rust doesn't have %g directly,
        // so we use Display which gives a reasonable representation.
        let s = format!("{n}");
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
    }
}
