use std::cell::RefCell;
use std::rc::Rc;

use rlua_core::table::LuaTable;
use rlua_core::value::LuaValue;

pub fn lua_table_insert(args: &[LuaValue]) -> Result<Vec<LuaValue>, String> {
    let table = get_table(args, "insert")?;
    let mut t = table.borrow_mut();
    match args.len() {
        2 => {
            // table.insert(t, value) — append
            let val = args[1].clone();
            let pos = t.len() + 1;
            t.rawset(LuaValue::Number(pos as f64), val);
        }
        3 => {
            // table.insert(t, pos, value) — insert at pos
            let pos = args[1]
                .to_number()
                .ok_or("bad argument #2 to 'insert' (number expected)")?
                as usize;
            let val = args[2].clone();
            let len = t.len();
            // Shift elements up
            for i in (pos..=len).rev() {
                let v = t.rawget(&LuaValue::Number(i as f64));
                t.rawset(LuaValue::Number((i + 1) as f64), v);
            }
            t.rawset(LuaValue::Number(pos as f64), val);
        }
        _ => {
            return Err("wrong number of arguments to 'insert'".to_owned());
        }
    }
    Ok(Vec::new())
}

pub fn lua_table_remove(args: &[LuaValue]) -> Result<Vec<LuaValue>, String> {
    let table = get_table(args, "remove")?;
    let mut t = table.borrow_mut();
    let len = t.len();
    let pos = if args.len() >= 2 {
        args[1]
            .to_number()
            .ok_or("bad argument #2 to 'remove' (number expected)")? as usize
    } else {
        len
    };
    if pos < 1 || pos > len {
        return Ok(vec![LuaValue::Nil]);
    }
    let removed = t.rawget(&LuaValue::Number(pos as f64));
    // Shift elements down
    for i in pos..len {
        let v = t.rawget(&LuaValue::Number((i + 1) as f64));
        t.rawset(LuaValue::Number(i as f64), v);
    }
    t.rawset(LuaValue::Number(len as f64), LuaValue::Nil);
    Ok(vec![removed])
}

pub fn lua_table_sort(args: &[LuaValue]) -> Result<Vec<LuaValue>, String> {
    let table = get_table(args, "sort")?;
    let comp = args.get(1).cloned();
    let len = table.borrow().len();
    if len <= 1 {
        return Ok(Vec::new());
    }

    // Extract array elements
    let mut elems: Vec<LuaValue> = Vec::with_capacity(len);
    for i in 1..=len {
        elems.push(table.borrow().rawget(&LuaValue::Number(i as f64)));
    }

    // Sort with simple insertion sort (no VM callback needed for default comparator,
    // but we need to handle custom comparator without VM access).
    // For custom comparator: NativeFn can be called directly.
    if let Some(comp_fn) = &comp {
        // Custom comparator — call it for each comparison
        // Since NativeFn = fn(&[LuaValue]) -> Result<Vec<LuaValue>, String>,
        // we can call it if it's a native function
        match comp_fn {
            LuaValue::Function(f) => {
                match f.as_ref() {
                    rlua_core::function::LuaFunction::Native { func, .. } => {
                        // Use insertion sort for stability and simplicity
                        let func = *func;
                        let mut err: Option<String> = None;
                        elems.sort_by(|a, b| {
                            if err.is_some() {
                                return std::cmp::Ordering::Equal;
                            }
                            match func(&[a.clone(), b.clone()]) {
                                Ok(res) => {
                                    let r = res.first().map(|v| v.is_truthy()).unwrap_or(false);
                                    if r {
                                        std::cmp::Ordering::Less
                                    } else {
                                        std::cmp::Ordering::Greater
                                    }
                                }
                                Err(e) => {
                                    err = Some(e);
                                    std::cmp::Ordering::Equal
                                }
                            }
                        });
                        if let Some(e) = err {
                            return Err(e);
                        }
                    }
                    rlua_core::function::LuaFunction::Lua(_) => {
                        return Err(
                            "table.sort with Lua comparator not supported in native context"
                                .to_owned(),
                        );
                    }
                }
            }
            _ => {
                return Err("bad argument #2 to 'sort' (function expected)".to_owned());
            }
        }
    } else {
        // Default sort: numbers by value, strings lexicographic
        let mut err: Option<String> = None;
        elems.sort_by(|a, b| {
            if err.is_some() {
                return std::cmp::Ordering::Equal;
            }
            match (a, b) {
                (LuaValue::Number(a), LuaValue::Number(b)) => {
                    a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
                }
                (LuaValue::String(a), LuaValue::String(b)) => a.cmp(b),
                _ => {
                    err = Some(format!(
                        "attempt to compare {} with {}",
                        a.type_name(),
                        b.type_name()
                    ));
                    std::cmp::Ordering::Equal
                }
            }
        });
        if let Some(e) = err {
            return Err(e);
        }
    }

    // Write back
    let mut t = table.borrow_mut();
    for (i, val) in elems.into_iter().enumerate() {
        t.rawset(LuaValue::Number((i + 1) as f64), val);
    }
    Ok(Vec::new())
}

pub fn lua_table_concat(args: &[LuaValue]) -> Result<Vec<LuaValue>, String> {
    let table = get_table(args, "concat")?;
    let sep = args
        .get(1)
        .map(|v| match v {
            LuaValue::String(s) => s.as_str().to_owned(),
            _ => v.to_lua_string(),
        })
        .unwrap_or_default();
    let t = table.borrow();
    let len = t.len();
    let i = args
        .get(2)
        .and_then(|v| v.to_number())
        .map(|n| n as usize)
        .unwrap_or(1);
    let j = args
        .get(3)
        .and_then(|v| v.to_number())
        .map(|n| n as usize)
        .unwrap_or(len);

    let mut parts = Vec::new();
    for idx in i..=j {
        let val = t.rawget(&LuaValue::Number(idx as f64));
        match &val {
            LuaValue::String(s) => parts.push((**s).clone()),
            LuaValue::Number(_) => parts.push(val.to_lua_string()),
            _ => {
                return Err(format!(
                    "invalid value ({}) at index {} in table for 'concat'",
                    val.type_name(),
                    idx
                ));
            }
        }
    }
    Ok(vec![LuaValue::from(parts.join(&sep))])
}

fn get_table(args: &[LuaValue], name: &str) -> Result<Rc<RefCell<LuaTable>>, String> {
    match args.first() {
        Some(LuaValue::Table(t)) => Ok(t.clone()),
        Some(other) => Err(format!(
            "bad argument #1 to '{}' (table expected, got {})",
            name,
            other.type_name()
        )),
        None => Err(format!("bad argument #1 to '{}' (table expected)", name)),
    }
}
