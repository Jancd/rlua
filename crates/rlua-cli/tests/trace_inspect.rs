use std::path::{Path, PathBuf};
use std::process::Command;

fn trace_inspect_binary() -> &'static str {
    env!("CARGO_BIN_EXE_trace-inspect")
}

fn jit_script(name: &str) -> PathBuf {
    let manifest = env!("CARGO_MANIFEST_DIR");
    Path::new(manifest)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests/jit")
        .join(name)
}

#[test]
fn trace_inspect_text_summary_runs_without_optional_diagnostics() {
    let output = Command::new(trace_inspect_binary())
        .arg("--hot-threshold")
        .arg("2")
        .arg(jit_script("native_side_exit_resume.lua"))
        .output()
        .expect("trace-inspect should run");

    assert!(
        output.status.success(),
        "trace-inspect failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("trace-inspect summary"));
    assert!(stdout.contains("execution_mode:"));
    assert!(stdout.contains("stats: trace_count="));
    assert!(stdout.contains("invalidated_bypasses="));
    assert!(stdout.contains("traces:"));
}

#[test]
fn trace_inspect_json_output_includes_lifecycle_and_fallback_fields() {
    let output = Command::new(trace_inspect_binary())
        .arg("--format")
        .arg("json")
        .arg("--hot-threshold")
        .arg("2")
        .arg("--side-exit-threshold")
        .arg("1")
        .arg(jit_script("guard_invalidation_recovery.lua"))
        .output()
        .expect("trace-inspect should run");

    assert!(
        output.status.success(),
        "trace-inspect failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\"script\":"));
    assert!(stdout.contains("\"trace_count\":"));
    assert!(stdout.contains("\"trace_invalidations\":"));
    assert!(stdout.contains("\"invalidated_bypasses\":"));
    assert!(stdout.contains("\"traces\":["));
    assert!(stdout.contains("\"lifecycle_state\":"));
    assert!(stdout.contains("\"last_execution\":"));
    assert!(stdout.contains("\"replay_entries\":"));
}
