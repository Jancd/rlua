use std::rc::Rc;

use rlua_core::value::LuaValue;
use rlua_vm::VmState;

/// Register all M1 standard library functions into the VM's global table.
pub fn register_stdlib(state: &mut VmState) {
    state.register_global("print", lua_print);
    state.register_global("type", lua_type);
    state.register_global("tostring", lua_tostring);
    state.register_global("tonumber", lua_tonumber);
    state.register_global("error", lua_error);
    state.register_global("assert", lua_assert);
    state.register_global("rawget", lua_rawget);
    state.register_global("rawset", lua_rawset);
    state.register_global("next", lua_next);
    state.register_global("select", lua_select);
    state.register_global("unpack", lua_unpack);
    state.register_global("ipairs", lua_ipairs);
    state.register_global("pairs", lua_pairs);
    state.register_global("pcall", lua_pcall);

    // Internal iterators for pairs/ipairs
    state.register_global("__ipairs_iter", ipairs_iterator);
}

fn lua_print(args: &[LuaValue]) -> Result<Vec<LuaValue>, String> {
    let parts: Vec<String> = args.iter().map(|v| v.to_lua_string()).collect();
    println!("{}", parts.join("\t"));
    Ok(Vec::new())
}

fn lua_type(args: &[LuaValue]) -> Result<Vec<LuaValue>, String> {
    let val = args.first().unwrap_or(&LuaValue::Nil);
    Ok(vec![LuaValue::from(val.type_name())])
}

fn lua_tostring(args: &[LuaValue]) -> Result<Vec<LuaValue>, String> {
    let val = args.first().unwrap_or(&LuaValue::Nil);
    Ok(vec![LuaValue::from(val.to_lua_string())])
}

fn lua_tonumber(args: &[LuaValue]) -> Result<Vec<LuaValue>, String> {
    let val = args.first().unwrap_or(&LuaValue::Nil);
    match val.to_number() {
        Some(n) => Ok(vec![LuaValue::Number(n)]),
        None => Ok(vec![LuaValue::Nil]),
    }
}

fn lua_error(args: &[LuaValue]) -> Result<Vec<LuaValue>, String> {
    let msg = args.first().unwrap_or(&LuaValue::Nil).to_lua_string();
    Err(msg)
}

fn lua_assert(args: &[LuaValue]) -> Result<Vec<LuaValue>, String> {
    let val = args.first().unwrap_or(&LuaValue::Nil);
    if val.is_truthy() {
        Ok(args.to_vec())
    } else {
        let msg = args
            .get(1)
            .map(|v| v.to_lua_string())
            .unwrap_or_else(|| "assertion failed!".to_owned());
        Err(msg)
    }
}

fn lua_rawget(args: &[LuaValue]) -> Result<Vec<LuaValue>, String> {
    let table = args
        .first()
        .ok_or("bad argument #1 to 'rawget' (table expected)")?;
    let key = args.get(1).unwrap_or(&LuaValue::Nil);
    match table {
        LuaValue::Table(t) => Ok(vec![t.borrow().rawget(key)]),
        _ => Err(format!(
            "bad argument #1 to 'rawget' (table expected, got {})",
            table.type_name()
        )),
    }
}

fn lua_rawset(args: &[LuaValue]) -> Result<Vec<LuaValue>, String> {
    let table = args
        .first()
        .ok_or("bad argument #1 to 'rawset' (table expected)")?;
    let key = args.get(1).unwrap_or(&LuaValue::Nil).clone();
    let val = args.get(2).unwrap_or(&LuaValue::Nil).clone();
    match table {
        LuaValue::Table(t) => {
            t.borrow_mut().rawset(key, val);
            Ok(vec![table.clone()])
        }
        _ => Err(format!(
            "bad argument #1 to 'rawset' (table expected, got {})",
            table.type_name()
        )),
    }
}

fn lua_next(args: &[LuaValue]) -> Result<Vec<LuaValue>, String> {
    let table = args
        .first()
        .ok_or("bad argument #1 to 'next' (table expected)")?;
    let key = args.get(1).unwrap_or(&LuaValue::Nil);
    match table {
        LuaValue::Table(t) => match t.borrow().next(key) {
            Some((k, v)) => Ok(vec![k, v]),
            None => Ok(vec![LuaValue::Nil]),
        },
        _ => Err(format!(
            "bad argument #1 to 'next' (table expected, got {})",
            table.type_name()
        )),
    }
}

fn lua_select(args: &[LuaValue]) -> Result<Vec<LuaValue>, String> {
    let index = args.first().unwrap_or(&LuaValue::Nil);
    match index {
        LuaValue::String(s) if s.as_str() == "#" => {
            let count = if args.len() > 1 { args.len() - 1 } else { 0 };
            Ok(vec![LuaValue::Number(count as f64)])
        }
        _ => {
            let n = index
                .to_number()
                .ok_or("bad argument #1 to 'select' (number or string expected)")?;
            let n = n as i64;
            let arg_count = (args.len() - 1) as i64; // exclude the index argument itself
            let idx = if n < 0 {
                // Negative index counts from the end
                let resolved = arg_count + 1 + n; // e.g. -1 with 3 args -> 3
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
            Ok(args[idx..].to_vec())
        }
    }
}

fn lua_unpack(args: &[LuaValue]) -> Result<Vec<LuaValue>, String> {
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
            Ok(results)
        }
        _ => Err(format!(
            "bad argument #1 to 'unpack' (table expected, got {})",
            table.type_name()
        )),
    }
}

fn lua_pairs(args: &[LuaValue]) -> Result<Vec<LuaValue>, String> {
    let table = args
        .first()
        .ok_or("bad argument #1 to 'pairs' (table expected)")?;
    match table {
        LuaValue::Table(_) => {
            // Return next, table, nil
            // We need to return the `next` function. Since we can't reference our own
            // registered functions easily, we return a special native function.
            // Actually, for simplicity, we return our lua_next function equivalent.
            // The caller (generic for) will call it as iterator(state, control).
            Ok(vec![
                LuaValue::Function(Rc::new(rlua_core::function::LuaFunction::Native {
                    name: "next",
                    func: lua_next,
                })),
                table.clone(),
                LuaValue::Nil,
            ])
        }
        _ => Err(format!(
            "bad argument #1 to 'pairs' (table expected, got {})",
            table.type_name()
        )),
    }
}

/// pcall is handled specially by the VM's CALL handler. This native function
/// exists only as a placeholder so it can be looked up as a global.
fn lua_pcall(_args: &[LuaValue]) -> Result<Vec<LuaValue>, String> {
    // Should never be called directly — the VM intercepts calls to "pcall"
    Err("pcall: internal error — should be handled by VM".to_owned())
}

fn lua_ipairs(args: &[LuaValue]) -> Result<Vec<LuaValue>, String> {
    let table = args
        .first()
        .ok_or("bad argument #1 to 'ipairs' (table expected)")?;
    match table {
        LuaValue::Table(_) => {
            // Return iterator, table, 0
            Ok(vec![
                LuaValue::Function(Rc::new(rlua_core::function::LuaFunction::Native {
                    name: "__ipairs_iter",
                    func: ipairs_iterator,
                })),
                table.clone(),
                LuaValue::Number(0.0),
            ])
        }
        _ => Err(format!(
            "bad argument #1 to 'ipairs' (table expected, got {})",
            table.type_name()
        )),
    }
}

fn ipairs_iterator(args: &[LuaValue]) -> Result<Vec<LuaValue>, String> {
    let table = args.first().unwrap_or(&LuaValue::Nil);
    let index = args.get(1).and_then(|v| v.to_number()).unwrap_or(0.0);
    let next_index = index + 1.0;

    match table {
        LuaValue::Table(t) => {
            let val = t.borrow().rawget(&LuaValue::Number(next_index));
            if val == LuaValue::Nil {
                Ok(vec![LuaValue::Nil])
            } else {
                Ok(vec![LuaValue::Number(next_index), val])
            }
        }
        _ => Ok(vec![LuaValue::Nil]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rlua_core::table::LuaTable;
    use std::cell::RefCell;

    #[test]
    fn test_type() {
        assert_eq!(
            lua_type(&[LuaValue::Number(1.0)]).unwrap(),
            vec![LuaValue::from("number")]
        );
        assert_eq!(
            lua_type(&[LuaValue::Nil]).unwrap(),
            vec![LuaValue::from("nil")]
        );
        assert_eq!(
            lua_type(&[LuaValue::from("hello")]).unwrap(),
            vec![LuaValue::from("string")]
        );
    }

    #[test]
    fn test_tostring() {
        assert_eq!(
            lua_tostring(&[LuaValue::Number(42.0)]).unwrap(),
            vec![LuaValue::from("42")]
        );
        assert_eq!(
            lua_tostring(&[LuaValue::Boolean(true)]).unwrap(),
            vec![LuaValue::from("true")]
        );
    }

    #[test]
    fn test_tonumber() {
        assert_eq!(
            lua_tonumber(&[LuaValue::from("42")]).unwrap(),
            vec![LuaValue::Number(42.0)]
        );
        assert_eq!(
            lua_tonumber(&[LuaValue::from("hello")]).unwrap(),
            vec![LuaValue::Nil]
        );
    }

    #[test]
    fn test_assert_success() {
        let result = lua_assert(&[LuaValue::Boolean(true), LuaValue::from("msg")]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_assert_failure() {
        let result = lua_assert(&[LuaValue::Boolean(false), LuaValue::from("oops")]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "oops");
    }

    #[test]
    fn test_error() {
        let result = lua_error(&[LuaValue::from("boom")]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "boom");
    }

    #[test]
    fn test_select_index() {
        let result = lua_select(&[
            LuaValue::Number(2.0),
            LuaValue::from("a"),
            LuaValue::from("b"),
            LuaValue::from("c"),
        ])
        .unwrap();
        assert_eq!(result, vec![LuaValue::from("b"), LuaValue::from("c")]);
    }

    #[test]
    fn test_select_count() {
        let result = lua_select(&[
            LuaValue::from("#"),
            LuaValue::Number(1.0),
            LuaValue::Number(2.0),
        ])
        .unwrap();
        assert_eq!(result, vec![LuaValue::Number(2.0)]);
    }

    #[test]
    fn test_rawget_rawset() {
        let t = Rc::new(RefCell::new(LuaTable::new()));
        let table = LuaValue::Table(t);

        lua_rawset(&[table.clone(), LuaValue::from("key"), LuaValue::Number(99.0)]).unwrap();

        let result = lua_rawget(&[table, LuaValue::from("key")]).unwrap();
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

        let result = lua_unpack(&[LuaValue::Table(t)]).unwrap();
        assert_eq!(
            result,
            vec![
                LuaValue::from("a"),
                LuaValue::from("b"),
                LuaValue::from("c")
            ]
        );
    }
}
