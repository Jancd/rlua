use std::fmt::Write as _;

use rlua_ir::{TraceDeoptExit, TraceDeoptExitKind};
use rlua_jit::{
    ExecutionMode, JitAvailability, JitConfig, NativeArtifactState, TraceExecutionState,
    TraceInvalidationReason, TraceLifecycleState,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputFormat {
    Text,
    Json,
}

#[derive(Debug, Clone)]
struct Options {
    jit: JitConfig,
    format: OutputFormat,
    script: String,
}

fn main() {
    let options = parse_args(std::env::args().skip(1).collect())
        .unwrap_or_else(|err| abort(&format!("trace-inspect: {err}")));
    let source = std::fs::read_to_string(&options.script).unwrap_or_else(|err| {
        abort(&format!(
            "trace-inspect: cannot read {}: {err}",
            options.script
        ))
    });
    let proto = rlua_compiler::compile_named(&source, &options.script)
        .unwrap_or_else(|err| abort(&format!("trace-inspect: {}: {err}", options.script)));

    let mut state = rlua_vm::VmState::with_jit_config(options.jit);
    rlua_stdlib::register_stdlib(&mut state);
    rlua_vm::execute(&mut state, proto)
        .unwrap_or_else(|err| abort(&format!("trace-inspect: {}: {err}", options.script)));

    let debug = state.jit_debug_state();
    match options.format {
        OutputFormat::Text => print!("{}", format_text_summary(&options.script, &debug)),
        OutputFormat::Json => print!("{}", format_json_summary(&options.script, &debug)),
    }
}

fn parse_args(args: Vec<String>) -> Result<Options, String> {
    let mut jit = JitConfig::default();
    let mut format = OutputFormat::Text;
    let mut script = None;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--jit" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--jit requires 'on' or 'off'".to_string())?;
                parse_jit_switch(&mut jit, value)?;
                index += 2;
            }
            option if option.starts_with("--jit=") => {
                parse_jit_switch(&mut jit, &option["--jit=".len()..])?;
                index += 1;
            }
            "--no-jit" => {
                jit.enabled = false;
                index += 1;
            }
            "--hot-threshold" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--hot-threshold requires an integer".to_string())?;
                jit.hot_threshold = parse_positive_u32("--hot-threshold", value)?;
                index += 2;
            }
            option if option.starts_with("--hot-threshold=") => {
                jit.hot_threshold =
                    parse_positive_u32("--hot-threshold", &option["--hot-threshold=".len()..])?;
                index += 1;
            }
            "--side-exit-threshold" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--side-exit-threshold requires an integer".to_string())?;
                jit.side_exit_threshold = parse_positive_u32("--side-exit-threshold", value)?;
                index += 2;
            }
            option if option.starts_with("--side-exit-threshold=") => {
                jit.side_exit_threshold = parse_positive_u32(
                    "--side-exit-threshold",
                    &option["--side-exit-threshold=".len()..],
                )?;
                index += 1;
            }
            "--format" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--format requires 'text' or 'json'".to_string())?;
                format = parse_format(value)?;
                index += 2;
            }
            option if option.starts_with("--format=") => {
                format = parse_format(&option["--format=".len()..])?;
                index += 1;
            }
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            option if option.starts_with('-') => {
                return Err(format!("unexpected option '{option}'"));
            }
            file => {
                if script.is_some() {
                    return Err(format!("unexpected extra argument '{file}'"));
                }
                script = Some(file.to_owned());
                index += 1;
            }
        }
    }

    let Some(script) = script else {
        return Err("missing Lua script path".to_string());
    };

    Ok(Options {
        jit,
        format,
        script,
    })
}

fn parse_jit_switch(config: &mut JitConfig, value: &str) -> Result<(), String> {
    match value {
        "on" => config.enabled = true,
        "off" => config.enabled = false,
        _ => {
            return Err(format!(
                "invalid JIT mode '{value}', expected 'on' or 'off'"
            ));
        }
    }
    Ok(())
}

fn parse_positive_u32(flag: &str, value: &str) -> Result<u32, String> {
    let parsed = value
        .parse::<u32>()
        .map_err(|_| format!("invalid value '{value}' for {flag}"))?;
    if parsed == 0 {
        return Err(format!("{flag} must be greater than zero"));
    }
    Ok(parsed)
}

fn parse_format(value: &str) -> Result<OutputFormat, String> {
    match value {
        "text" => Ok(OutputFormat::Text),
        "json" => Ok(OutputFormat::Json),
        _ => Err(format!(
            "invalid format '{value}', expected 'text' or 'json'"
        )),
    }
}

fn print_help() {
    println!("Usage: cargo run -p rlua-cli --bin trace-inspect -- [options] <script>");
    println!("Options:");
    println!("  --jit on|off              enable or disable JIT recording");
    println!("  --no-jit                  shorthand for '--jit off'");
    println!("  --hot-threshold N         JIT hot-loop threshold");
    println!("  --side-exit-threshold N   trace downgrade/invalidation threshold");
    println!("  --format text|json        output format (default: text)");
    println!("  -h, --help                show this help");
}

fn format_text_summary(script: &str, debug: &rlua_vm::VmJitDebugState) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "trace-inspect summary");
    let _ = writeln!(out, "script: {script}");
    let _ = writeln!(
        out,
        "execution_mode: {}",
        execution_mode_name(debug.execution_mode)
    );
    let _ = writeln!(
        out,
        "availability: {}",
        availability_name(debug.availability)
    );
    let _ = writeln!(
        out,
        "config: enabled={} hot_threshold={} side_exit_threshold={}",
        debug.config.enabled, debug.config.hot_threshold, debug.config.side_exit_threshold
    );
    let _ = writeln!(
        out,
        "stats: trace_count={} hot_loop_triggers={} cache_hits={} native_entries={} replay_entries={} side_exits={} invalidated_bypasses={} native_failures={} trace_downgrades={} trace_invalidations={} trace_recompiles={}",
        debug.trace_count,
        debug.stats.hot_loop_triggers,
        debug.stats.cache_hits,
        debug.stats.native_entries,
        debug.stats.replay_entries,
        debug.stats.side_exits,
        debug.stats.invalidated_bypasses,
        debug.stats.native_failures,
        debug.stats.trace_downgrades,
        debug.stats.trace_invalidations,
        debug.stats.trace_recompiles
    );

    if debug.counters.is_empty() {
        let _ = writeln!(out, "hot_loop_counters: none");
    } else {
        let _ = writeln!(out, "hot_loop_counters:");
        for counter in &debug.counters {
            let _ = writeln!(
                out,
                "  - function=0x{:x} loop_header_pc={} hits={}",
                counter.function, counter.loop_header_pc, counter.hits
            );
        }
    }

    if debug.traces.is_empty() {
        let _ = writeln!(out, "traces: none");
        return out;
    }

    let _ = writeln!(out, "traces:");
    for (index, trace) in debug.traces.iter().enumerate() {
        let _ = writeln!(
            out,
            "  - trace[{index}] function=0x{:x} loop_header_pc={} generation={} optimized={} lifecycle={} native_state={} last_execution={} replay_entries={} native_entries={} side_exits={} invalidated_bypasses={} invalidation={} deopt_exits={}",
            trace.function,
            trace.loop_header_pc,
            trace.generation,
            trace.optimized,
            lifecycle_name(trace.lifecycle_state),
            native_state_name(trace.native_state),
            execution_state_name(trace.last_execution),
            trace.replay_entries,
            trace.native_entries,
            trace.side_exit_count,
            trace.invalidated_bypasses,
            option_name(trace.invalidation_reason.map(invalidation_reason_name)),
            trace.deopt_exits.len()
        );
        let _ = writeln!(
            out,
            "    last_deopt: {}",
            trace
                .last_deopt
                .as_ref()
                .map(format_deopt_text)
                .unwrap_or_else(|| "none".to_string())
        );
    }

    out
}

fn format_json_summary(script: &str, debug: &rlua_vm::VmJitDebugState) -> String {
    let mut out = String::new();
    out.push('{');
    push_json_field_string(&mut out, "script", script);
    out.push(',');
    push_json_field_string(
        &mut out,
        "execution_mode",
        execution_mode_name(debug.execution_mode),
    );
    out.push(',');
    push_json_field_string(
        &mut out,
        "availability",
        availability_name(debug.availability),
    );
    out.push(',');
    out.push_str("\"config\":{");
    push_json_field_bool(&mut out, "enabled", debug.config.enabled);
    out.push(',');
    push_json_field_u64(
        &mut out,
        "hot_threshold",
        u64::from(debug.config.hot_threshold),
    );
    out.push(',');
    push_json_field_u64(
        &mut out,
        "side_exit_threshold",
        u64::from(debug.config.side_exit_threshold),
    );
    out.push('}');
    out.push(',');
    push_json_field_u64(&mut out, "trace_count", debug.trace_count as u64);
    out.push(',');
    out.push_str("\"stats\":{");
    push_json_field_u64(&mut out, "hot_loop_triggers", debug.stats.hot_loop_triggers);
    out.push(',');
    push_json_field_u64(&mut out, "record_attempts", debug.stats.record_attempts);
    out.push(',');
    push_json_field_u64(&mut out, "trace_installs", debug.stats.trace_installs);
    out.push(',');
    push_json_field_u64(&mut out, "cache_hits", debug.stats.cache_hits);
    out.push(',');
    push_json_field_u64(&mut out, "replay_entries", debug.stats.replay_entries);
    out.push(',');
    push_json_field_u64(&mut out, "side_exits", debug.stats.side_exits);
    out.push(',');
    push_json_field_u64(
        &mut out,
        "invalidated_bypasses",
        debug.stats.invalidated_bypasses,
    );
    out.push(',');
    push_json_field_u64(&mut out, "optimize_attempts", debug.stats.optimize_attempts);
    out.push(',');
    push_json_field_u64(&mut out, "optimized_traces", debug.stats.optimized_traces);
    out.push(',');
    push_json_field_u64(
        &mut out,
        "native_compile_attempts",
        debug.stats.native_compile_attempts,
    );
    out.push(',');
    push_json_field_u64(
        &mut out,
        "native_compile_installs",
        debug.stats.native_compile_installs,
    );
    out.push(',');
    push_json_field_u64(
        &mut out,
        "native_compile_skips",
        debug.stats.native_compile_skips,
    );
    out.push(',');
    push_json_field_u64(&mut out, "native_entries", debug.stats.native_entries);
    out.push(',');
    push_json_field_u64(&mut out, "native_failures", debug.stats.native_failures);
    out.push(',');
    push_json_field_u64(&mut out, "trace_downgrades", debug.stats.trace_downgrades);
    out.push(',');
    push_json_field_u64(
        &mut out,
        "trace_invalidations",
        debug.stats.trace_invalidations,
    );
    out.push(',');
    push_json_field_u64(&mut out, "trace_recompiles", debug.stats.trace_recompiles);
    out.push('}');
    out.push(',');
    out.push_str("\"hot_loop_counters\":[");
    for (index, counter) in debug.counters.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        push_json_field_string(&mut out, "function", &format!("0x{:x}", counter.function));
        out.push(',');
        push_json_field_u64(&mut out, "loop_header_pc", counter.loop_header_pc as u64);
        out.push(',');
        push_json_field_u64(&mut out, "hits", u64::from(counter.hits));
        out.push('}');
    }
    out.push(']');
    out.push(',');
    out.push_str("\"traces\":[");
    for (index, trace) in debug.traces.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push('{');
        push_json_field_string(&mut out, "function", &format!("0x{:x}", trace.function));
        out.push(',');
        push_json_field_u64(&mut out, "loop_header_pc", trace.loop_header_pc as u64);
        out.push(',');
        push_json_field_u64(&mut out, "generation", trace.generation);
        out.push(',');
        push_json_field_bool(&mut out, "optimized", trace.optimized);
        out.push(',');
        push_json_field_string(
            &mut out,
            "lifecycle_state",
            lifecycle_name(trace.lifecycle_state),
        );
        out.push(',');
        push_json_field_string(
            &mut out,
            "native_state",
            native_state_name(trace.native_state),
        );
        out.push(',');
        push_json_field_string(
            &mut out,
            "last_execution",
            execution_state_name(trace.last_execution),
        );
        out.push(',');
        push_json_field_u64(&mut out, "replay_entries", trace.replay_entries);
        out.push(',');
        push_json_field_u64(&mut out, "native_entries", trace.native_entries);
        out.push(',');
        push_json_field_u64(&mut out, "side_exit_count", trace.side_exit_count);
        out.push(',');
        push_json_field_u64(&mut out, "invalidated_bypasses", trace.invalidated_bypasses);
        out.push(',');
        push_json_field_u64(&mut out, "deopt_exit_count", trace.deopt_exits.len() as u64);
        out.push(',');
        push_json_field_optional_string(
            &mut out,
            "invalidation_reason",
            trace.invalidation_reason.map(invalidation_reason_name),
        );
        out.push(',');
        out.push_str("\"last_deopt\":");
        match &trace.last_deopt {
            Some(deopt) => push_deopt_json(&mut out, deopt),
            None => out.push_str("null"),
        }
        out.push('}');
    }
    out.push(']');
    out.push('}');
    out.push('\n');
    out
}

fn push_json_field_string(out: &mut String, key: &str, value: &str) {
    push_json_string(out, key);
    out.push(':');
    push_json_string(out, value);
}

fn push_json_field_optional_string(out: &mut String, key: &str, value: Option<&str>) {
    push_json_string(out, key);
    out.push(':');
    match value {
        Some(value) => push_json_string(out, value),
        None => out.push_str("null"),
    }
}

fn push_json_field_bool(out: &mut String, key: &str, value: bool) {
    push_json_string(out, key);
    out.push(':');
    out.push_str(if value { "true" } else { "false" });
}

fn push_json_field_u64(out: &mut String, key: &str, value: u64) {
    push_json_string(out, key);
    out.push(':');
    let _ = write!(out, "{value}");
}

fn push_json_string(out: &mut String, value: &str) {
    out.push('"');
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if ch.is_control() => {
                let _ = write!(out, "\\u{:04x}", ch as u32);
            }
            ch => out.push(ch),
        }
    }
    out.push('"');
}

fn push_deopt_json(out: &mut String, deopt: &TraceDeoptExit) {
    out.push('{');
    push_json_field_string(out, "kind", deopt_kind_name(&deopt.kind));
    out.push(',');
    match deopt.kind {
        TraceDeoptExitKind::Guard { guard_id, slot } => {
            push_json_field_u64(out, "guard_id", u64::from(guard_id));
            out.push(',');
            push_json_field_u64(out, "slot", u64::from(slot));
            out.push(',');
        }
        TraceDeoptExitKind::SideExit { pc } => {
            push_json_field_u64(out, "pc", pc as u64);
            out.push(',');
        }
    }
    push_json_field_u64(out, "resume_pc", deopt.resume_pc as u64);
    out.push(',');
    out.push_str("\"live_in_slots\":[");
    for (index, slot) in deopt.live_in_slots.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        let _ = write!(out, "{slot}");
    }
    out.push(']');
    out.push(',');
    out.push_str("\"materialized_slots\":[");
    for (index, slot) in deopt.materialized_slots.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        let _ = write!(out, "{slot}");
    }
    out.push(']');
    out.push('}');
}

fn format_deopt_text(deopt: &TraceDeoptExit) -> String {
    match deopt.kind {
        TraceDeoptExitKind::Guard { guard_id, slot } => format!(
            "guard(guard_id={guard_id}, slot={slot}, resume_pc={}, live_in={:?}, materialized={:?})",
            deopt.resume_pc, deopt.live_in_slots, deopt.materialized_slots
        ),
        TraceDeoptExitKind::SideExit { pc } => format!(
            "side-exit(pc={pc}, resume_pc={}, live_in={:?}, materialized={:?})",
            deopt.resume_pc, deopt.live_in_slots, deopt.materialized_slots
        ),
    }
}

fn deopt_kind_name(kind: &TraceDeoptExitKind) -> &'static str {
    match kind {
        TraceDeoptExitKind::Guard { .. } => "guard",
        TraceDeoptExitKind::SideExit { .. } => "side-exit",
    }
}

fn execution_mode_name(mode: ExecutionMode) -> &'static str {
    match mode {
        ExecutionMode::InterpreterOnly => "interpreter-only",
        ExecutionMode::JitEnabled => "jit-enabled",
        ExecutionMode::JitUnavailable => "jit-unavailable",
    }
}

fn availability_name(availability: JitAvailability) -> &'static str {
    match availability {
        JitAvailability::Available => "available",
        JitAvailability::UnsupportedArch => "unsupported-arch",
    }
}

fn lifecycle_name(state: TraceLifecycleState) -> &'static str {
    match state {
        TraceLifecycleState::Active => "active",
        TraceLifecycleState::ReplayOnly => "replay-only",
        TraceLifecycleState::Invalidated => "invalidated",
    }
}

fn invalidation_reason_name(reason: TraceInvalidationReason) -> &'static str {
    match reason {
        TraceInvalidationReason::NativeFailure => "native-failure",
        TraceInvalidationReason::SideExitThreshold => "side-exit-threshold",
    }
}

fn native_state_name(state: NativeArtifactState) -> &'static str {
    match state {
        NativeArtifactState::Unavailable => "unavailable",
        NativeArtifactState::UnsupportedArch => "unsupported-arch",
        NativeArtifactState::UnsupportedTrace => "unsupported-trace",
        NativeArtifactState::Installed => "installed",
        NativeArtifactState::CompileFailed => "compile-failed",
    }
}

fn execution_state_name(state: TraceExecutionState) -> &'static str {
    match state {
        TraceExecutionState::None => "none",
        TraceExecutionState::Native => "native",
        TraceExecutionState::Replay => "replay",
        TraceExecutionState::InterpreterFallback => "interpreter-fallback",
    }
}

fn option_name(value: Option<&str>) -> &str {
    value.unwrap_or("none")
}

fn abort(message: &str) -> ! {
    eprintln!("{message}");
    std::process::exit(1);
}
