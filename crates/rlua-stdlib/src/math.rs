use rlua_core::function::{CallOutcome, NativeVmContext};
use rlua_core::value::LuaValue;

pub fn lua_math_abs(
    _ctx: &mut dyn NativeVmContext,
    args: &[LuaValue],
) -> Result<CallOutcome, String> {
    let n = to_num(args, 1, "abs")?;
    ret(vec![LuaValue::Number(n.abs())])
}

pub fn lua_math_ceil(
    _ctx: &mut dyn NativeVmContext,
    args: &[LuaValue],
) -> Result<CallOutcome, String> {
    let n = to_num(args, 1, "ceil")?;
    ret(vec![LuaValue::Number(n.ceil())])
}

pub fn lua_math_floor(
    _ctx: &mut dyn NativeVmContext,
    args: &[LuaValue],
) -> Result<CallOutcome, String> {
    let n = to_num(args, 1, "floor")?;
    ret(vec![LuaValue::Number(n.floor())])
}

pub fn lua_math_sqrt(
    _ctx: &mut dyn NativeVmContext,
    args: &[LuaValue],
) -> Result<CallOutcome, String> {
    let n = to_num(args, 1, "sqrt")?;
    ret(vec![LuaValue::Number(n.sqrt())])
}

pub fn lua_math_sin(
    _ctx: &mut dyn NativeVmContext,
    args: &[LuaValue],
) -> Result<CallOutcome, String> {
    let n = to_num(args, 1, "sin")?;
    ret(vec![LuaValue::Number(n.sin())])
}

pub fn lua_math_cos(
    _ctx: &mut dyn NativeVmContext,
    args: &[LuaValue],
) -> Result<CallOutcome, String> {
    let n = to_num(args, 1, "cos")?;
    ret(vec![LuaValue::Number(n.cos())])
}

pub fn lua_math_tan(
    _ctx: &mut dyn NativeVmContext,
    args: &[LuaValue],
) -> Result<CallOutcome, String> {
    let n = to_num(args, 1, "tan")?;
    ret(vec![LuaValue::Number(n.tan())])
}

pub fn lua_math_log(
    _ctx: &mut dyn NativeVmContext,
    args: &[LuaValue],
) -> Result<CallOutcome, String> {
    let n = to_num(args, 1, "log")?;
    ret(vec![LuaValue::Number(n.ln())])
}

pub fn lua_math_exp(
    _ctx: &mut dyn NativeVmContext,
    args: &[LuaValue],
) -> Result<CallOutcome, String> {
    let n = to_num(args, 1, "exp")?;
    ret(vec![LuaValue::Number(n.exp())])
}

pub fn lua_math_fmod(
    _ctx: &mut dyn NativeVmContext,
    args: &[LuaValue],
) -> Result<CallOutcome, String> {
    let x = to_num(args, 1, "fmod")?;
    let y = to_num_idx(args, 2, "fmod")?;
    ret(vec![LuaValue::Number(x % y)])
}

pub fn lua_math_modf(
    _ctx: &mut dyn NativeVmContext,
    args: &[LuaValue],
) -> Result<CallOutcome, String> {
    let n = to_num(args, 1, "modf")?;
    let trunc = n.trunc();
    let frac = n - trunc;
    ret(vec![LuaValue::Number(trunc), LuaValue::Number(frac)])
}

pub fn lua_math_deg(
    _ctx: &mut dyn NativeVmContext,
    args: &[LuaValue],
) -> Result<CallOutcome, String> {
    let n = to_num(args, 1, "deg")?;
    ret(vec![LuaValue::Number(n.to_degrees())])
}

pub fn lua_math_rad(
    _ctx: &mut dyn NativeVmContext,
    args: &[LuaValue],
) -> Result<CallOutcome, String> {
    let n = to_num(args, 1, "rad")?;
    ret(vec![LuaValue::Number(n.to_radians())])
}

pub fn lua_math_max(
    _ctx: &mut dyn NativeVmContext,
    args: &[LuaValue],
) -> Result<CallOutcome, String> {
    if args.is_empty() {
        return Err("bad argument #1 to 'max' (value expected)".to_owned());
    }
    let mut max = to_num(args, 1, "max")?;
    for (i, arg) in args.iter().enumerate().skip(1) {
        let n = arg
            .to_number()
            .ok_or_else(|| format!("bad argument #{} to 'max' (number expected)", i + 1))?;
        if n > max {
            max = n;
        }
    }
    ret(vec![LuaValue::Number(max)])
}

pub fn lua_math_min(
    _ctx: &mut dyn NativeVmContext,
    args: &[LuaValue],
) -> Result<CallOutcome, String> {
    if args.is_empty() {
        return Err("bad argument #1 to 'min' (value expected)".to_owned());
    }
    let mut min = to_num(args, 1, "min")?;
    for (i, arg) in args.iter().enumerate().skip(1) {
        let n = arg
            .to_number()
            .ok_or_else(|| format!("bad argument #{} to 'min' (number expected)", i + 1))?;
        if n < min {
            min = n;
        }
    }
    ret(vec![LuaValue::Number(min)])
}

use std::sync::atomic::{AtomicU64, Ordering};
static RANDOM_SEED: AtomicU64 = AtomicU64::new(0);

fn next_random() -> f64 {
    let mut seed = RANDOM_SEED.load(Ordering::Relaxed);
    seed = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    RANDOM_SEED.store(seed, Ordering::Relaxed);
    (seed >> 11) as f64 / (1u64 << 53) as f64
}

pub fn lua_math_random(
    _ctx: &mut dyn NativeVmContext,
    args: &[LuaValue],
) -> Result<CallOutcome, String> {
    match args.len() {
        0 => ret(vec![LuaValue::Number(next_random())]),
        1 => {
            let m = to_num(args, 1, "random")? as i64;
            if m < 1 {
                return Err("bad argument #1 to 'random' (interval is empty)".to_owned());
            }
            let r = (next_random() * m as f64).floor() as i64 + 1;
            ret(vec![LuaValue::Number(r.min(m) as f64)])
        }
        _ => {
            let m = to_num(args, 1, "random")? as i64;
            let n = to_num_idx(args, 2, "random")? as i64;
            if m > n {
                return Err("bad argument #2 to 'random' (interval is empty)".to_owned());
            }
            let range = (n - m + 1) as f64;
            let r = (next_random() * range).floor() as i64 + m;
            ret(vec![LuaValue::Number(r.min(n) as f64)])
        }
    }
}

pub fn lua_math_randomseed(
    _ctx: &mut dyn NativeVmContext,
    args: &[LuaValue],
) -> Result<CallOutcome, String> {
    let n = to_num(args, 1, "randomseed")? as u64;
    RANDOM_SEED.store(n, Ordering::Relaxed);
    ret(Vec::new())
}

fn ret(values: Vec<LuaValue>) -> Result<CallOutcome, String> {
    Ok(CallOutcome::Return(values))
}

fn to_num(args: &[LuaValue], pos: usize, name: &str) -> Result<f64, String> {
    args.first()
        .and_then(|v| v.to_number())
        .ok_or_else(|| format!("bad argument #{pos} to '{name}' (number expected)"))
}

fn to_num_idx(args: &[LuaValue], pos: usize, name: &str) -> Result<f64, String> {
    args.get(pos - 1)
        .and_then(|v| v.to_number())
        .ok_or_else(|| format!("bad argument #{pos} to '{name}' (number expected)"))
}
