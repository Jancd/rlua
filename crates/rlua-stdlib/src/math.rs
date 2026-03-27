use rlua_core::value::LuaValue;

pub fn lua_math_abs(args: &[LuaValue]) -> Result<Vec<LuaValue>, String> {
    let n = to_num(args, 1, "abs")?;
    Ok(vec![LuaValue::Number(n.abs())])
}

pub fn lua_math_ceil(args: &[LuaValue]) -> Result<Vec<LuaValue>, String> {
    let n = to_num(args, 1, "ceil")?;
    Ok(vec![LuaValue::Number(n.ceil())])
}

pub fn lua_math_floor(args: &[LuaValue]) -> Result<Vec<LuaValue>, String> {
    let n = to_num(args, 1, "floor")?;
    Ok(vec![LuaValue::Number(n.floor())])
}

pub fn lua_math_sqrt(args: &[LuaValue]) -> Result<Vec<LuaValue>, String> {
    let n = to_num(args, 1, "sqrt")?;
    Ok(vec![LuaValue::Number(n.sqrt())])
}

pub fn lua_math_sin(args: &[LuaValue]) -> Result<Vec<LuaValue>, String> {
    let n = to_num(args, 1, "sin")?;
    Ok(vec![LuaValue::Number(n.sin())])
}

pub fn lua_math_cos(args: &[LuaValue]) -> Result<Vec<LuaValue>, String> {
    let n = to_num(args, 1, "cos")?;
    Ok(vec![LuaValue::Number(n.cos())])
}

pub fn lua_math_tan(args: &[LuaValue]) -> Result<Vec<LuaValue>, String> {
    let n = to_num(args, 1, "tan")?;
    Ok(vec![LuaValue::Number(n.tan())])
}

pub fn lua_math_log(args: &[LuaValue]) -> Result<Vec<LuaValue>, String> {
    let n = to_num(args, 1, "log")?;
    Ok(vec![LuaValue::Number(n.ln())])
}

pub fn lua_math_exp(args: &[LuaValue]) -> Result<Vec<LuaValue>, String> {
    let n = to_num(args, 1, "exp")?;
    Ok(vec![LuaValue::Number(n.exp())])
}

pub fn lua_math_fmod(args: &[LuaValue]) -> Result<Vec<LuaValue>, String> {
    let x = to_num(args, 1, "fmod")?;
    let y = to_num_idx(args, 2, "fmod")?;
    Ok(vec![LuaValue::Number(x % y)])
}

pub fn lua_math_modf(args: &[LuaValue]) -> Result<Vec<LuaValue>, String> {
    let n = to_num(args, 1, "modf")?;
    let trunc = n.trunc();
    let frac = n - trunc;
    Ok(vec![LuaValue::Number(trunc), LuaValue::Number(frac)])
}

pub fn lua_math_deg(args: &[LuaValue]) -> Result<Vec<LuaValue>, String> {
    let n = to_num(args, 1, "deg")?;
    Ok(vec![LuaValue::Number(n.to_degrees())])
}

pub fn lua_math_rad(args: &[LuaValue]) -> Result<Vec<LuaValue>, String> {
    let n = to_num(args, 1, "rad")?;
    Ok(vec![LuaValue::Number(n.to_radians())])
}

pub fn lua_math_max(args: &[LuaValue]) -> Result<Vec<LuaValue>, String> {
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
    Ok(vec![LuaValue::Number(max)])
}

pub fn lua_math_min(args: &[LuaValue]) -> Result<Vec<LuaValue>, String> {
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
    Ok(vec![LuaValue::Number(min)])
}

// Simple PRNG state using a static mutable (not thread-safe, but Lua is single-threaded).
// Uses a linear congruential generator.
use std::sync::atomic::{AtomicU64, Ordering};
static RANDOM_SEED: AtomicU64 = AtomicU64::new(0);

fn next_random() -> f64 {
    let mut seed = RANDOM_SEED.load(Ordering::Relaxed);
    // LCG parameters (same as glibc)
    seed = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    RANDOM_SEED.store(seed, Ordering::Relaxed);
    // Convert to [0, 1)
    (seed >> 11) as f64 / (1u64 << 53) as f64
}

pub fn lua_math_random(args: &[LuaValue]) -> Result<Vec<LuaValue>, String> {
    match args.len() {
        0 => Ok(vec![LuaValue::Number(next_random())]),
        1 => {
            let m = to_num(args, 1, "random")? as i64;
            if m < 1 {
                return Err("bad argument #1 to 'random' (interval is empty)".to_owned());
            }
            let r = (next_random() * m as f64).floor() as i64 + 1;
            Ok(vec![LuaValue::Number(r.min(m) as f64)])
        }
        _ => {
            let m = to_num(args, 1, "random")? as i64;
            let n = to_num_idx(args, 2, "random")? as i64;
            if m > n {
                return Err("bad argument #2 to 'random' (interval is empty)".to_owned());
            }
            let range = (n - m + 1) as f64;
            let r = (next_random() * range).floor() as i64 + m;
            Ok(vec![LuaValue::Number(r.min(n) as f64)])
        }
    }
}

pub fn lua_math_randomseed(args: &[LuaValue]) -> Result<Vec<LuaValue>, String> {
    let n = to_num(args, 1, "randomseed")? as u64;
    RANDOM_SEED.store(n, Ordering::Relaxed);
    Ok(Vec::new())
}

// Helpers
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
