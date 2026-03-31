use std::rc::Rc;

use rlua_core::function::{CallOutcome, LuaFunction, NativeVmContext};
use rlua_core::value::LuaValue;

pub fn lua_coroutine_create(
    ctx: &mut dyn NativeVmContext,
    args: &[LuaValue],
) -> Result<CallOutcome, String> {
    let func = args
        .first()
        .ok_or("bad argument #1 to 'create' (function expected)")?;
    let thread = ctx.create_coroutine(func)?;
    ret(vec![thread])
}

pub fn lua_coroutine_resume(
    ctx: &mut dyn NativeVmContext,
    args: &[LuaValue],
) -> Result<CallOutcome, String> {
    let thread = args
        .first()
        .ok_or("bad argument #1 to 'resume' (thread expected)")?;
    let resume_args = if args.len() > 1 { &args[1..] } else { &[] };
    ret(ctx.resume_coroutine(thread, resume_args)?)
}

pub fn lua_coroutine_yield(
    ctx: &mut dyn NativeVmContext,
    args: &[LuaValue],
) -> Result<CallOutcome, String> {
    ctx.yield_current(args)
}

pub fn lua_coroutine_running(
    ctx: &mut dyn NativeVmContext,
    _args: &[LuaValue],
) -> Result<CallOutcome, String> {
    ret(vec![ctx.running_coroutine().unwrap_or(LuaValue::Nil)])
}

pub fn lua_coroutine_status(
    ctx: &mut dyn NativeVmContext,
    args: &[LuaValue],
) -> Result<CallOutcome, String> {
    let thread = args
        .first()
        .ok_or("bad argument #1 to 'status' (thread expected)")?;
    ret(vec![LuaValue::from(ctx.coroutine_status(thread)?)])
}

pub fn lua_coroutine_wrap(
    ctx: &mut dyn NativeVmContext,
    args: &[LuaValue],
) -> Result<CallOutcome, String> {
    let func = args
        .first()
        .ok_or("bad argument #1 to 'wrap' (function expected)")?;
    let thread = ctx.create_coroutine(func)?;
    let LuaValue::Thread(thread) = thread else {
        return Err("coroutine.wrap internal error".to_owned());
    };
    ret(vec![LuaValue::Function(Rc::new(
        LuaFunction::WrappedCoroutine { thread },
    ))])
}

fn ret(values: Vec<LuaValue>) -> Result<CallOutcome, String> {
    Ok(CallOutcome::Return(values))
}
