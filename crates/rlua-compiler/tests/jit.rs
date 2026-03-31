use std::path::{Path, PathBuf};

use rlua_ir::TraceDeoptExitKind;
use rlua_jit::{ExecutionMode, NativeArtifactState};

fn jit_path(name: &str) -> PathBuf {
    let manifest = env!("CARGO_MANIFEST_DIR");
    Path::new(manifest)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests/jit")
        .join(name)
}

fn run_jit_case_with_config(
    name: &str,
    config: rlua_jit::JitConfig,
) -> (Vec<String>, Vec<String>, rlua_vm::VmJitDebugState) {
    let path = jit_path(name);
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    let proto = rlua_compiler::compile_named(&source, path.to_str().unwrap())
        .unwrap_or_else(|e| panic!("{}: compile error: {e}", path.display()));

    let mut state = rlua_vm::VmState::with_jit_config(config);
    rlua_stdlib::register_stdlib(&mut state);

    let results = rlua_vm::execute(&mut state, proto)
        .unwrap_or_else(|e| panic!("{}: runtime error: {e}", path.display()));
    let result_strings = results.iter().map(|value| value.to_lua_string()).collect();
    let output = state.get_output().to_vec();

    (result_strings, output, state.jit_debug_state())
}

fn run_jit_case(name: &str, enabled: bool) -> (Vec<String>, Vec<String>, rlua_vm::VmJitDebugState) {
    run_jit_case_with_config(
        name,
        rlua_jit::JitConfig {
            enabled,
            hot_threshold: 2,
            ..rlua_jit::JitConfig::default()
        },
    )
}

fn assert_jit_matches_interpreter(name: &str) {
    let (jit_results, jit_output, jit_debug) = run_jit_case(name, true);
    let (interp_results, interp_output, interp_debug) = run_jit_case(name, false);

    assert_eq!(jit_results, interp_results, "result mismatch for {name}");
    assert_eq!(jit_output, interp_output, "stdout mismatch for {name}");
    assert!(
        jit_debug.trace_count >= 1,
        "expected at least one cached trace for {name}"
    );
    assert_eq!(
        interp_debug.trace_count, 0,
        "interpreter-only run should not cache traces"
    );
    assert_eq!(jit_debug.traces.len(), jit_debug.trace_count);
    assert_eq!(interp_debug.traces.len(), 0);
}

fn assert_supported_trace_backend_state(name: &str) {
    let (_, _, jit_debug) = run_jit_case(name, true);

    if cfg!(target_arch = "x86_64") {
        assert_eq!(jit_debug.execution_mode, ExecutionMode::JitEnabled);
        assert!(
            jit_debug.stats.native_compile_installs >= 1,
            "expected native trace installation for {name}"
        );
        assert!(
            jit_debug.stats.native_entries >= 1,
            "expected native execution for {name}"
        );
        assert!(
            jit_debug
                .traces
                .iter()
                .any(|trace| trace.native_state == NativeArtifactState::Installed)
        );
    } else {
        assert_eq!(jit_debug.execution_mode, ExecutionMode::JitUnavailable);
        assert_eq!(jit_debug.stats.native_entries, 0);
        assert!(
            jit_debug
                .traces
                .iter()
                .all(|trace| trace.native_state == NativeArtifactState::UnsupportedArch)
        );
        assert!(
            jit_debug.stats.replay_entries >= 1,
            "expected replay activity for {name}"
        );
    }
}

fn assert_fallback_trace_backend_state(name: &str) {
    let (jit_results, jit_output, jit_debug) = run_jit_case(name, true);
    let (interp_results, interp_output, interp_debug) = run_jit_case(name, false);

    assert_eq!(jit_results, interp_results, "result mismatch for {name}");
    assert_eq!(jit_output, interp_output, "stdout mismatch for {name}");
    assert!(
        jit_debug.trace_count >= 1,
        "expected at least one cached trace for {name}"
    );
    assert_eq!(interp_debug.trace_count, 0);
    assert_eq!(jit_debug.stats.native_entries, 0);
    assert!(
        jit_debug.stats.side_exits >= 1 || jit_debug.stats.replay_entries >= 1,
        "expected replay or side-exit fallback activity for {name}"
    );

    let expected = if cfg!(target_arch = "x86_64") {
        NativeArtifactState::UnsupportedTrace
    } else {
        NativeArtifactState::UnsupportedArch
    };
    assert!(
        jit_debug
            .traces
            .iter()
            .all(|trace| trace.native_state == expected)
    );
}

fn assert_has_guard_deopt(name: &str, jit_debug: &rlua_vm::VmJitDebugState) {
    assert!(
        jit_debug.traces.iter().any(|trace| {
            matches!(
                trace.last_deopt.as_ref().map(|deopt| deopt.kind),
                Some(TraceDeoptExitKind::Guard { .. })
            )
        }),
        "expected guard deopt metadata for {name}"
    );
}

fn assert_has_side_exit_deopt(name: &str, jit_debug: &rlua_vm::VmJitDebugState) {
    assert!(
        jit_debug.traces.iter().any(|trace| {
            matches!(
                trace.last_deopt.as_ref().map(|deopt| deopt.kind),
                Some(TraceDeoptExitKind::SideExit { .. })
            )
        }),
        "expected side-exit deopt metadata for {name}"
    );
}

fn assert_trace_debug_is_observable(name: &str, jit_debug: &rlua_vm::VmJitDebugState) {
    assert!(
        jit_debug.trace_count >= 1,
        "expected cached trace debug state for {name}"
    );
    assert_eq!(jit_debug.traces.len(), jit_debug.trace_count);
    assert!(
        jit_debug.stats.replay_entries + jit_debug.stats.native_entries >= 1,
        "expected trace activity counters for {name}"
    );
    assert!(
        jit_debug.traces.iter().any(|trace| {
            trace.last_execution != rlua_jit::TraceExecutionState::None
                || trace.replay_entries > 0
                || trace.native_entries > 0
                || trace.side_exit_count > 0
                || trace.invalidated_bypasses > 0
        }),
        "expected trace execution metadata for {name}"
    );
}

#[test]
fn jit_numeric_sum_matches_interpreter() {
    assert_jit_matches_interpreter("numeric_sum.lua");
    assert_supported_trace_backend_state("numeric_sum.lua");
}

#[test]
fn jit_numeric_descending_matches_interpreter() {
    assert_jit_matches_interpreter("numeric_descending.lua");
    assert_supported_trace_backend_state("numeric_descending.lua");
}

#[test]
fn jit_unsupported_table_loop_falls_back_without_drift() {
    assert_fallback_trace_backend_state("unsupported_table_loop.lua");
}

#[test]
fn jit_native_side_exit_resume_matches_interpreter() {
    let (jit_results, jit_output, jit_debug) = run_jit_case("native_side_exit_resume.lua", true);
    let (interp_results, interp_output, _) = run_jit_case("native_side_exit_resume.lua", false);

    assert_eq!(jit_results, interp_results);
    assert_eq!(jit_output, interp_output);
    assert_supported_trace_backend_state("native_side_exit_resume.lua");
    assert_trace_debug_is_observable("native_side_exit_resume.lua", &jit_debug);
    assert!(jit_debug.stats.side_exits >= 1);
    assert_has_side_exit_deopt("native_side_exit_resume.lua", &jit_debug);
}

#[test]
fn jit_guard_exit_resumes_without_drift() {
    let (jit_results, jit_output, jit_debug) = run_jit_case("guard_string_seed.lua", true);
    let (interp_results, interp_output, _) = run_jit_case("guard_string_seed.lua", false);

    assert_eq!(jit_results, interp_results);
    assert_eq!(jit_output, interp_output);
    assert_supported_trace_backend_state("guard_string_seed.lua");
    assert_trace_debug_is_observable("guard_string_seed.lua", &jit_debug);
    assert!(jit_debug.stats.side_exits >= 1);
    assert_has_guard_deopt("guard_string_seed.lua", &jit_debug);
}

#[test]
fn jit_invalidated_trace_can_recompile_without_drift() {
    let config = rlua_jit::JitConfig {
        enabled: true,
        hot_threshold: 2,
        side_exit_threshold: 1,
    };
    let (jit_results, jit_output, jit_debug) =
        run_jit_case_with_config("guard_invalidation_recovery.lua", config);
    let (interp_results, interp_output, _) = run_jit_case("guard_invalidation_recovery.lua", false);

    assert_eq!(jit_results, interp_results);
    assert_eq!(jit_output, interp_output);
    assert_trace_debug_is_observable("guard_invalidation_recovery.lua", &jit_debug);
    assert!(jit_debug.stats.side_exits >= 2);
    assert!(jit_debug.stats.trace_invalidations >= 1);
    assert!(jit_debug.stats.trace_recompiles >= 1);
    let replacement = jit_debug
        .traces
        .iter()
        .find(|trace| trace.generation >= 1)
        .expect("expected a replacement trace generation after invalidation");
    assert!(replacement.side_exit_count <= u64::from(config.side_exit_threshold));
    assert!(replacement.replay_entries + replacement.native_entries >= 1);
}

#[test]
fn jit_coroutine_execution_stays_interpreter_only() {
    let (jit_results, jit_output, jit_debug) = run_jit_case("coroutine_interpreter_only.lua", true);
    let (interp_results, interp_output, interp_debug) =
        run_jit_case("coroutine_interpreter_only.lua", false);

    assert_eq!(jit_results, interp_results);
    assert_eq!(jit_output, interp_output);
    assert_eq!(jit_debug.trace_count, 0);
    assert_eq!(jit_debug.stats.native_entries, 0);
    assert_eq!(interp_debug.trace_count, 0);
}
