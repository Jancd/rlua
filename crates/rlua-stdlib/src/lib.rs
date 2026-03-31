mod coroutine_lib;
mod math;
mod string_lib;
mod table_lib;

use std::cell::RefCell;
use std::rc::Rc;

use rlua_core::function::{CallOutcome, LuaFunction, NativeVmContext};
use rlua_core::table::LuaTable;
use rlua_core::value::LuaValue;
use rlua_vm::VmState;

/// Register all standard library functions into the VM's global table.
pub fn register_stdlib(state: &mut VmState) {
    // Global functions
    state.register_global("print", lua_print);
    state.register_global("type", lua_type);
    state.register_global("tostring", lua_tostring);
    state.register_global("tonumber", lua_tonumber);
    state.register_global("error", lua_error);
    state.register_global("assert", lua_assert);
    state.register_global("rawget", lua_rawget);
    state.register_global("rawset", lua_rawset);
    state.register_global("rawequal", lua_rawequal);
    state.register_global("rawlen", lua_rawlen);
    state.register_global("next", lua_next);
    state.register_global("select", lua_select);
    state.register_global("unpack", lua_unpack);
    state.register_global("ipairs", lua_ipairs);
    state.register_global("pairs", lua_pairs);
    state.register_global("pcall", lua_pcall);
    state.register_global("xpcall", lua_xpcall);
    state.register_global("setmetatable", lua_setmetatable);
    state.register_global("getmetatable", lua_getmetatable);

    // Internal iterators for pairs/ipairs
    state.register_global("__ipairs_iter", ipairs_iterator);

    // Register module tables
    register_math_lib(state);
    register_coroutine_lib(state);
    register_table_lib(state);
    register_string_lib(state);
}

// ---------------------------------------------------------------------------
// Module table registration
// ---------------------------------------------------------------------------

fn make_native(name: &'static str, func: rlua_core::NativeFn) -> LuaValue {
    LuaValue::Function(Rc::new(LuaFunction::Native { name, func }))
}

fn register_math_lib(state: &mut VmState) {
    let t = Rc::new(RefCell::new(LuaTable::new()));
    {
        let mut mt = t.borrow_mut();
        mt.rawset(
            LuaValue::from("abs"),
            make_native("math.abs", math::lua_math_abs),
        );
        mt.rawset(
            LuaValue::from("ceil"),
            make_native("math.ceil", math::lua_math_ceil),
        );
        mt.rawset(
            LuaValue::from("floor"),
            make_native("math.floor", math::lua_math_floor),
        );
        mt.rawset(
            LuaValue::from("sqrt"),
            make_native("math.sqrt", math::lua_math_sqrt),
        );
        mt.rawset(
            LuaValue::from("sin"),
            make_native("math.sin", math::lua_math_sin),
        );
        mt.rawset(
            LuaValue::from("cos"),
            make_native("math.cos", math::lua_math_cos),
        );
        mt.rawset(
            LuaValue::from("tan"),
            make_native("math.tan", math::lua_math_tan),
        );
        mt.rawset(
            LuaValue::from("log"),
            make_native("math.log", math::lua_math_log),
        );
        mt.rawset(
            LuaValue::from("exp"),
            make_native("math.exp", math::lua_math_exp),
        );
        mt.rawset(
            LuaValue::from("fmod"),
            make_native("math.fmod", math::lua_math_fmod),
        );
        mt.rawset(
            LuaValue::from("modf"),
            make_native("math.modf", math::lua_math_modf),
        );
        mt.rawset(
            LuaValue::from("deg"),
            make_native("math.deg", math::lua_math_deg),
        );
        mt.rawset(
            LuaValue::from("rad"),
            make_native("math.rad", math::lua_math_rad),
        );
        mt.rawset(
            LuaValue::from("max"),
            make_native("math.max", math::lua_math_max),
        );
        mt.rawset(
            LuaValue::from("min"),
            make_native("math.min", math::lua_math_min),
        );
        mt.rawset(
            LuaValue::from("random"),
            make_native("math.random", math::lua_math_random),
        );
        mt.rawset(
            LuaValue::from("randomseed"),
            make_native("math.randomseed", math::lua_math_randomseed),
        );
        mt.rawset(LuaValue::from("pi"), LuaValue::Number(std::f64::consts::PI));
        mt.rawset(LuaValue::from("huge"), LuaValue::Number(f64::INFINITY));
    }
    state
        .globals()
        .borrow_mut()
        .rawset(LuaValue::from("math"), LuaValue::Table(t));
}

fn register_coroutine_lib(state: &mut VmState) {
    let t = Rc::new(RefCell::new(LuaTable::new()));
    {
        let mut mt = t.borrow_mut();
        mt.rawset(
            LuaValue::from("create"),
            make_native("coroutine.create", coroutine_lib::lua_coroutine_create),
        );
        mt.rawset(
            LuaValue::from("resume"),
            make_native("coroutine.resume", coroutine_lib::lua_coroutine_resume),
        );
        mt.rawset(
            LuaValue::from("yield"),
            make_native("coroutine.yield", coroutine_lib::lua_coroutine_yield),
        );
        mt.rawset(
            LuaValue::from("running"),
            make_native("coroutine.running", coroutine_lib::lua_coroutine_running),
        );
        mt.rawset(
            LuaValue::from("status"),
            make_native("coroutine.status", coroutine_lib::lua_coroutine_status),
        );
        mt.rawset(
            LuaValue::from("wrap"),
            make_native("coroutine.wrap", coroutine_lib::lua_coroutine_wrap),
        );
    }
    state
        .globals()
        .borrow_mut()
        .rawset(LuaValue::from("coroutine"), LuaValue::Table(t));
}

fn register_table_lib(state: &mut VmState) {
    let t = Rc::new(RefCell::new(LuaTable::new()));
    {
        let mut mt = t.borrow_mut();
        mt.rawset(
            LuaValue::from("insert"),
            make_native("table.insert", table_lib::lua_table_insert),
        );
        mt.rawset(
            LuaValue::from("remove"),
            make_native("table.remove", table_lib::lua_table_remove),
        );
        mt.rawset(
            LuaValue::from("sort"),
            make_native("table.sort", table_lib::lua_table_sort),
        );
        mt.rawset(
            LuaValue::from("concat"),
            make_native("table.concat", table_lib::lua_table_concat),
        );
    }
    state
        .globals()
        .borrow_mut()
        .rawset(LuaValue::from("table"), LuaValue::Table(t));
}

fn register_string_lib(state: &mut VmState) {
    let t = Rc::new(RefCell::new(LuaTable::new()));
    {
        let mut mt = t.borrow_mut();
        mt.rawset(
            LuaValue::from("byte"),
            make_native("string.byte", string_lib::lua_string_byte),
        );
        mt.rawset(
            LuaValue::from("char"),
            make_native("string.char", string_lib::lua_string_char),
        );
        mt.rawset(
            LuaValue::from("len"),
            make_native("string.len", string_lib::lua_string_len),
        );
        mt.rawset(
            LuaValue::from("lower"),
            make_native("string.lower", string_lib::lua_string_lower),
        );
        mt.rawset(
            LuaValue::from("upper"),
            make_native("string.upper", string_lib::lua_string_upper),
        );
        mt.rawset(
            LuaValue::from("reverse"),
            make_native("string.reverse", string_lib::lua_string_reverse),
        );
        mt.rawset(
            LuaValue::from("rep"),
            make_native("string.rep", string_lib::lua_string_rep),
        );
        mt.rawset(
            LuaValue::from("sub"),
            make_native("string.sub", string_lib::lua_string_sub),
        );
        mt.rawset(
            LuaValue::from("find"),
            make_native("string.find", string_lib::lua_string_find),
        );
        mt.rawset(
            LuaValue::from("match"),
            make_native("string.match", string_lib::lua_string_match),
        );
        mt.rawset(
            LuaValue::from("gmatch"),
            make_native("string.gmatch", string_lib::lua_string_gmatch),
        );
        mt.rawset(
            LuaValue::from("gsub"),
            make_native("string.gsub", string_lib::lua_string_gsub),
        );
        mt.rawset(
            LuaValue::from("format"),
            make_native("string.format", string_lib::lua_string_format),
        );
    }

    // Set up string metatable: all strings share a metatable with __index = string table
    let string_mt = Rc::new(RefCell::new(LuaTable::new()));
    string_mt
        .borrow_mut()
        .rawset(LuaValue::from("__index"), LuaValue::Table(t.clone()));
    state.set_string_metatable(string_mt);

    state
        .globals()
        .borrow_mut()
        .rawset(LuaValue::from("string"), LuaValue::Table(t));
}

// ---------------------------------------------------------------------------
// Global functions
// ---------------------------------------------------------------------------

fn lua_print(_ctx: &mut dyn NativeVmContext, args: &[LuaValue]) -> Result<CallOutcome, String> {
    let parts: Vec<String> = args.iter().map(|v| v.to_lua_string()).collect();
    println!("{}", parts.join("\t"));
    ret(Vec::new())
}

fn lua_type(_ctx: &mut dyn NativeVmContext, args: &[LuaValue]) -> Result<CallOutcome, String> {
    let val = args.first().unwrap_or(&LuaValue::Nil);
    ret(vec![LuaValue::from(val.type_name())])
}

fn lua_tostring(ctx: &mut dyn NativeVmContext, args: &[LuaValue]) -> Result<CallOutcome, String> {
    let val = args.first().unwrap_or(&LuaValue::Nil);
    if let LuaValue::Table(t) = val
        && let Some(mm) = t.borrow().get_metamethod("__tostring")
    {
        let results = ctx.call_function(&mm, std::slice::from_ref(val))?;
        let out = results.first().cloned().unwrap_or(LuaValue::Nil);
        return match out {
            LuaValue::String(_) | LuaValue::Number(_) => {
                ret(vec![LuaValue::from(out.to_lua_string())])
            }
            _ => Err("'__tostring' must return a string".to_owned()),
        };
    }
    ret(vec![LuaValue::from(val.to_lua_string())])
}

fn lua_tonumber(_ctx: &mut dyn NativeVmContext, args: &[LuaValue]) -> Result<CallOutcome, String> {
    let val = args.first().unwrap_or(&LuaValue::Nil);
    match val.to_number() {
        Some(n) => ret(vec![LuaValue::Number(n)]),
        None => ret(vec![LuaValue::Nil]),
    }
}

fn lua_error(_ctx: &mut dyn NativeVmContext, args: &[LuaValue]) -> Result<CallOutcome, String> {
    let msg = args.first().unwrap_or(&LuaValue::Nil);
    let _level = args.get(1).and_then(|v| v.to_number()).unwrap_or(1.0) as i32;
    Err(msg.to_lua_string())
}

fn lua_assert(_ctx: &mut dyn NativeVmContext, args: &[LuaValue]) -> Result<CallOutcome, String> {
    let val = args.first().unwrap_or(&LuaValue::Nil);
    if val.is_truthy() {
        ret(args.to_vec())
    } else {
        let msg = args
            .get(1)
            .map(|v| v.to_lua_string())
            .unwrap_or_else(|| "assertion failed!".to_owned());
        Err(msg)
    }
}

fn lua_rawget(_ctx: &mut dyn NativeVmContext, args: &[LuaValue]) -> Result<CallOutcome, String> {
    let table = args
        .first()
        .ok_or("bad argument #1 to 'rawget' (table expected)")?;
    let key = args.get(1).unwrap_or(&LuaValue::Nil);
    match table {
        LuaValue::Table(t) => ret(vec![t.borrow().rawget(key)]),
        _ => Err(format!(
            "bad argument #1 to 'rawget' (table expected, got {})",
            table.type_name()
        )),
    }
}

fn lua_rawset(_ctx: &mut dyn NativeVmContext, args: &[LuaValue]) -> Result<CallOutcome, String> {
    let table = args
        .first()
        .ok_or("bad argument #1 to 'rawset' (table expected)")?;
    let key = args.get(1).unwrap_or(&LuaValue::Nil).clone();
    let val = args.get(2).unwrap_or(&LuaValue::Nil).clone();
    match table {
        LuaValue::Table(t) => {
            t.borrow_mut().rawset(key, val);
            ret(vec![table.clone()])
        }
        _ => Err(format!(
            "bad argument #1 to 'rawset' (table expected, got {})",
            table.type_name()
        )),
    }
}

fn lua_rawequal(_ctx: &mut dyn NativeVmContext, args: &[LuaValue]) -> Result<CallOutcome, String> {
    let a = args.first().unwrap_or(&LuaValue::Nil);
    let b = args.get(1).unwrap_or(&LuaValue::Nil);
    ret(vec![LuaValue::Boolean(a == b)])
}

fn lua_rawlen(_ctx: &mut dyn NativeVmContext, args: &[LuaValue]) -> Result<CallOutcome, String> {
    match args.first().unwrap_or(&LuaValue::Nil) {
        LuaValue::Table(t) => ret(vec![LuaValue::Number(t.borrow().len() as f64)]),
        LuaValue::String(s) => ret(vec![LuaValue::Number(s.len() as f64)]),
        other => Err(format!(
            "bad argument #1 to 'rawlen' (table or string expected, got {})",
            other.type_name()
        )),
    }
}

fn lua_next(_ctx: &mut dyn NativeVmContext, args: &[LuaValue]) -> Result<CallOutcome, String> {
    let table = args
        .first()
        .ok_or("bad argument #1 to 'next' (table expected)")?;
    let key = args.get(1).unwrap_or(&LuaValue::Nil);
    match table {
        LuaValue::Table(t) => match t.borrow().next(key) {
            Some((k, v)) => ret(vec![k, v]),
            None => ret(vec![LuaValue::Nil]),
        },
        _ => Err(format!(
            "bad argument #1 to 'next' (table expected, got {})",
            table.type_name()
        )),
    }
}

fn lua_select(_ctx: &mut dyn NativeVmContext, args: &[LuaValue]) -> Result<CallOutcome, String> {
    let index = args.first().unwrap_or(&LuaValue::Nil);
    match index {
        LuaValue::String(s) if s.as_str() == "#" => {
            let count = if args.len() > 1 { args.len() - 1 } else { 0 };
            ret(vec![LuaValue::Number(count as f64)])
        }
        _ => {
            let n = index
                .to_number()
                .ok_or("bad argument #1 to 'select' (number or string expected)")?;
            let n = n as i64;
            let arg_count = (args.len() - 1) as i64;
            let idx = if n < 0 {
                let resolved = arg_count + 1 + n;
                if resolved < 1 {
                    return Err("bad argument #1 to 'select' (index out of range)".to_owned());
                }
                resolved as usize
            } else {
                if n == 0 || n > arg_count {
                    return Err("bad argument #1 to 'select' (index out of range)".to_owned());
                }
                n as usize
            };
            ret(args[idx..].to_vec())
        }
    }
}

fn lua_unpack(_ctx: &mut dyn NativeVmContext, args: &[LuaValue]) -> Result<CallOutcome, String> {
    let table = args
        .first()
        .ok_or("bad argument #1 to 'unpack' (table expected)")?;
    match table {
        LuaValue::Table(t) => {
            let t = t.borrow();
            let len = t.len();
            let mut results = Vec::new();
            for i in 1..=len {
                results.push(t.rawget(&LuaValue::Number(i as f64)));
            }
            ret(results)
        }
        _ => Err(format!(
            "bad argument #1 to 'unpack' (table expected, got {})",
            table.type_name()
        )),
    }
}

fn lua_pairs(_ctx: &mut dyn NativeVmContext, args: &[LuaValue]) -> Result<CallOutcome, String> {
    let table = args
        .first()
        .ok_or("bad argument #1 to 'pairs' (table expected)")?;
    match table {
        LuaValue::Table(_) => ret(vec![
            LuaValue::Function(Rc::new(LuaFunction::Native {
                name: "next",
                func: lua_next,
            })),
            table.clone(),
            LuaValue::Nil,
        ]),
        _ => Err(format!(
            "bad argument #1 to 'pairs' (table expected, got {})",
            table.type_name()
        )),
    }
}

/// pcall is handled specially by the VM's CALL handler.
fn lua_pcall(_ctx: &mut dyn NativeVmContext, _args: &[LuaValue]) -> Result<CallOutcome, String> {
    Err("pcall: internal error — should be handled by VM".to_owned())
}

/// xpcall is handled specially by the VM's CALL handler.
fn lua_xpcall(_ctx: &mut dyn NativeVmContext, _args: &[LuaValue]) -> Result<CallOutcome, String> {
    Err("xpcall: internal error — should be handled by VM".to_owned())
}

fn lua_setmetatable(
    _ctx: &mut dyn NativeVmContext,
    args: &[LuaValue],
) -> Result<CallOutcome, String> {
    let table = args
        .first()
        .ok_or("bad argument #1 to 'setmetatable' (table expected)")?;
    match table {
        LuaValue::Table(t) => {
            // Check for __metatable protection
            if let Some(mm) = t.borrow().get_metamethod("__metatable") {
                let _ = mm;
                return Err("cannot change a protected metatable".to_owned());
            }
            let mt = args.get(1).unwrap_or(&LuaValue::Nil);
            match mt {
                LuaValue::Table(mt_ref) => {
                    t.borrow_mut().set_metatable(Some(mt_ref.clone()));
                }
                LuaValue::Nil => {
                    t.borrow_mut().set_metatable(None);
                }
                _ => {
                    return Err(
                        "bad argument #2 to 'setmetatable' (nil or table expected)".to_owned()
                    );
                }
            }
            ret(vec![table.clone()])
        }
        _ => Err(format!(
            "bad argument #1 to 'setmetatable' (table expected, got {})",
            table.type_name()
        )),
    }
}

fn lua_getmetatable(
    _ctx: &mut dyn NativeVmContext,
    args: &[LuaValue],
) -> Result<CallOutcome, String> {
    let val = args.first().unwrap_or(&LuaValue::Nil);
    match val {
        LuaValue::Table(t) => {
            // Check for __metatable field first
            if let Some(mm) = t.borrow().get_metamethod("__metatable") {
                return ret(vec![mm]);
            }
            match t.borrow().metatable() {
                Some(mt) => ret(vec![LuaValue::Table(mt.clone())]),
                None => ret(vec![LuaValue::Nil]),
            }
        }
        _ => ret(vec![LuaValue::Nil]),
    }
}

fn lua_ipairs(_ctx: &mut dyn NativeVmContext, args: &[LuaValue]) -> Result<CallOutcome, String> {
    let table = args
        .first()
        .ok_or("bad argument #1 to 'ipairs' (table expected)")?;
    match table {
        LuaValue::Table(_) => ret(vec![
            LuaValue::Function(Rc::new(LuaFunction::Native {
                name: "__ipairs_iter",
                func: ipairs_iterator,
            })),
            table.clone(),
            LuaValue::Number(0.0),
        ]),
        _ => Err(format!(
            "bad argument #1 to 'ipairs' (table expected, got {})",
            table.type_name()
        )),
    }
}

fn ipairs_iterator(
    _ctx: &mut dyn NativeVmContext,
    args: &[LuaValue],
) -> Result<CallOutcome, String> {
    let table = args.first().unwrap_or(&LuaValue::Nil);
    let index = args.get(1).and_then(|v| v.to_number()).unwrap_or(0.0);
    let next_index = index + 1.0;
    match table {
        LuaValue::Table(t) => {
            let val = t.borrow().rawget(&LuaValue::Number(next_index));
            if val == LuaValue::Nil {
                ret(vec![LuaValue::Nil])
            } else {
                ret(vec![LuaValue::Number(next_index), val])
            }
        }
        _ => ret(vec![LuaValue::Nil]),
    }
}

fn ret(values: Vec<LuaValue>) -> Result<CallOutcome, String> {
    Ok(CallOutcome::Return(values))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rlua_core::table::LuaTable;
    use std::cell::RefCell;

    fn test_call(func: rlua_core::NativeFn, args: &[LuaValue]) -> Result<Vec<LuaValue>, String> {
        let mut state = VmState::new();
        match func(&mut state, args)? {
            CallOutcome::Return(values) => Ok(values),
            CallOutcome::Yield(_) => Err("unexpected yield in stdlib unit test".to_owned()),
        }
    }

    #[test]
    fn test_type() {
        assert_eq!(
            test_call(lua_type, &[LuaValue::Number(1.0)]).unwrap(),
            vec![LuaValue::from("number")]
        );
        assert_eq!(
            test_call(lua_type, &[LuaValue::Nil]).unwrap(),
            vec![LuaValue::from("nil")]
        );
        assert_eq!(
            test_call(lua_type, &[LuaValue::from("hello")]).unwrap(),
            vec![LuaValue::from("string")]
        );
    }

    #[test]
    fn test_tostring() {
        assert_eq!(
            test_call(lua_tostring, &[LuaValue::Number(42.0)]).unwrap(),
            vec![LuaValue::from("42")]
        );
        assert_eq!(
            test_call(lua_tostring, &[LuaValue::Boolean(true)]).unwrap(),
            vec![LuaValue::from("true")]
        );
    }

    #[test]
    fn test_tonumber() {
        assert_eq!(
            test_call(lua_tonumber, &[LuaValue::from("42")]).unwrap(),
            vec![LuaValue::Number(42.0)]
        );
        assert_eq!(
            test_call(lua_tonumber, &[LuaValue::from("hello")]).unwrap(),
            vec![LuaValue::Nil]
        );
    }

    #[test]
    fn test_assert_success() {
        let result = test_call(
            lua_assert,
            &[LuaValue::Boolean(true), LuaValue::from("msg")],
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_assert_failure() {
        let result = test_call(
            lua_assert,
            &[LuaValue::Boolean(false), LuaValue::from("oops")],
        );
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "oops");
    }

    #[test]
    fn test_error() {
        let result = test_call(lua_error, &[LuaValue::from("boom")]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "boom");
    }

    #[test]
    fn test_select_index() {
        let result = test_call(
            lua_select,
            &[
                LuaValue::Number(2.0),
                LuaValue::from("a"),
                LuaValue::from("b"),
                LuaValue::from("c"),
            ],
        )
        .unwrap();
        assert_eq!(result, vec![LuaValue::from("b"), LuaValue::from("c")]);
    }

    #[test]
    fn test_select_count() {
        let result = test_call(
            lua_select,
            &[
                LuaValue::from("#"),
                LuaValue::Number(1.0),
                LuaValue::Number(2.0),
            ],
        )
        .unwrap();
        assert_eq!(result, vec![LuaValue::Number(2.0)]);
    }

    #[test]
    fn test_rawget_rawset() {
        let t = Rc::new(RefCell::new(LuaTable::new()));
        let table = LuaValue::Table(t);
        test_call(
            lua_rawset,
            &[table.clone(), LuaValue::from("key"), LuaValue::Number(99.0)],
        )
        .unwrap();
        let result = test_call(lua_rawget, &[table, LuaValue::from("key")]).unwrap();
        assert_eq!(result, vec![LuaValue::Number(99.0)]);
    }

    #[test]
    fn test_unpack() {
        let t = Rc::new(RefCell::new(LuaTable::new()));
        t.borrow_mut()
            .rawset(LuaValue::Number(1.0), LuaValue::from("a"));
        t.borrow_mut()
            .rawset(LuaValue::Number(2.0), LuaValue::from("b"));
        t.borrow_mut()
            .rawset(LuaValue::Number(3.0), LuaValue::from("c"));
        let result = test_call(lua_unpack, &[LuaValue::Table(t)]).unwrap();
        assert_eq!(
            result,
            vec![
                LuaValue::from("a"),
                LuaValue::from("b"),
                LuaValue::from("c")
            ]
        );
    }

    #[test]
    fn test_setmetatable_getmetatable() {
        let t = Rc::new(RefCell::new(LuaTable::new()));
        let mt = Rc::new(RefCell::new(LuaTable::new()));
        let table = LuaValue::Table(t);
        let metatable = LuaValue::Table(mt);

        test_call(lua_setmetatable, &[table.clone(), metatable.clone()]).unwrap();
        let result = test_call(lua_getmetatable, &[table]).unwrap();
        assert_eq!(result.len(), 1);
        // Should return the metatable
        assert!(matches!(result[0], LuaValue::Table(_)));
    }

    #[test]
    fn test_setmetatable_protected() {
        let t = Rc::new(RefCell::new(LuaTable::new()));
        let mt = Rc::new(RefCell::new(LuaTable::new()));
        mt.borrow_mut()
            .rawset(LuaValue::from("__metatable"), LuaValue::from("protected"));
        t.borrow_mut().set_metatable(Some(mt));
        let table = LuaValue::Table(t);
        // getmetatable should return __metatable value
        let result = test_call(lua_getmetatable, std::slice::from_ref(&table)).unwrap();
        assert_eq!(result[0], LuaValue::from("protected"));
        // setmetatable should fail
        let new_mt = Rc::new(RefCell::new(LuaTable::new()));
        let result = test_call(lua_setmetatable, &[table, LuaValue::Table(new_mt)]);
        assert!(result.is_err());
    }
}
