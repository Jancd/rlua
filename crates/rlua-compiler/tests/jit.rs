use std::path::{Path, PathBuf};

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

fn run_jit_case(name: &str, enabled: bool) -> (Vec<String>, Vec<String>, rlua_vm::VmJitDebugState) {
    let path = jit_path(name);
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    let proto = rlua_compiler::compile_named(&source, path.to_str().unwrap())
        .unwrap_or_else(|e| panic!("{}: compile error: {e}", path.display()));

    let mut state = rlua_vm::VmState::with_jit_config(rlua_jit::JitConfig {
        enabled,
        hot_threshold: 2,
    });
    rlua_stdlib::register_stdlib(&mut state);

    let results = rlua_vm::execute(&mut state, proto)
        .unwrap_or_else(|e| panic!("{}: runtime error: {e}", path.display()));
    let result_strings = results.iter().map(|value| value.to_lua_string()).collect();
    let output = state.get_output().to_vec();

    (result_strings, output, state.jit_debug_state())
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
    assert!(
        jit_debug.stats.replay_entries >= 1,
        "expected replay activity for {name}"
    );
    assert_eq!(
        interp_debug.trace_count, 0,
        "interpreter-only run should not cache traces"
    );
}

#[test]
fn jit_numeric_sum_matches_interpreter() {
    assert_jit_matches_interpreter("numeric_sum.lua");
}

#[test]
fn jit_numeric_descending_matches_interpreter() {
    assert_jit_matches_interpreter("numeric_descending.lua");
}
