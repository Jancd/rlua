use std::path::{Path, PathBuf};
use std::process::Command;

/// Find the rlua binary path (built by cargo).
fn rlua_binary() -> PathBuf {
    let manifest = env!("CARGO_MANIFEST_DIR");
    // The binary is at target/debug/rlua
    Path::new(manifest)
        .parent()
        .unwrap() // crates/
        .parent()
        .unwrap() // rlua/
        .join("target/debug/rlua-cli")
}

/// Find a reference Lua 5.1-compatible interpreter on the system.
fn find_reference_lua() -> Option<String> {
    for cmd in &["lua5.1", "luajit"] {
        if let Ok(output) = Command::new(cmd).arg("-v").output() {
            let combined = String::from_utf8_lossy(&output.stdout).to_string()
                + &String::from_utf8_lossy(&output.stderr);
            if combined.contains("5.1") || combined.contains("LuaJIT") {
                return Some(cmd.to_string());
            }
        }
    }
    None
}

fn differential_dir() -> PathBuf {
    let manifest = env!("CARGO_MANIFEST_DIR");
    Path::new(manifest)
        .parent()
        .unwrap() // crates/
        .parent()
        .unwrap() // rlua/
        .join("tests/differential")
}

fn run_command(cmd: &str, script_path: &Path) -> Result<String, String> {
    let output = Command::new(cmd)
        .arg(script_path)
        .output()
        .map_err(|e| format!("failed to run {cmd}: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("{cmd} failed: {stderr}"));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn run_differential_test(name: &str) {
    let lua_cmd = match find_reference_lua() {
        Some(cmd) => cmd,
        None => {
            eprintln!(
                "SKIPPED {name}: no Lua 5.1 reference interpreter found (install lua5.1 or luajit)"
            );
            return;
        }
    };

    let rlua_bin = rlua_binary();
    if !rlua_bin.exists() {
        panic!(
            "rlua binary not found at {}. Run `cargo build` first.",
            rlua_bin.display()
        );
    }

    let script_path = differential_dir().join(name);

    // Run through rlua
    let rlua_output = run_command(rlua_bin.to_str().unwrap(), &script_path)
        .unwrap_or_else(|e| panic!("rlua failed on {name}: {e}"));

    // Run through reference interpreter
    let ref_output = run_command(&lua_cmd, &script_path)
        .unwrap_or_else(|e| panic!("{lua_cmd} failed on {name}: {e}"));

    // Compare outputs line by line
    if rlua_output != ref_output {
        let rlua_lines: Vec<&str> = rlua_output.lines().collect();
        let ref_lines: Vec<&str> = ref_output.lines().collect();

        let mut diffs = Vec::new();
        let max_lines = rlua_lines.len().max(ref_lines.len());
        for i in 0..max_lines {
            let rlua_line = rlua_lines.get(i).copied().unwrap_or("<missing>");
            let ref_line = ref_lines.get(i).copied().unwrap_or("<missing>");
            if rlua_line != ref_line {
                diffs.push(format!(
                    "  line {}: rlua={:?} ref={:?}",
                    i + 1,
                    rlua_line,
                    ref_line
                ));
            }
        }

        panic!(
            "Output mismatch in {name} (ref={lua_cmd}):\n{}\n\n--- rlua output ---\n{}\n--- ref output ---\n{}",
            diffs.join("\n"),
            rlua_output,
            ref_output
        );
    }
}

#[test]
fn diff_arithmetic() {
    run_differential_test("arithmetic.lua");
}

#[test]
fn diff_strings() {
    run_differential_test("strings.lua");
}

#[test]
fn diff_tables() {
    run_differential_test("tables.lua");
}

#[test]
fn diff_metatables() {
    run_differential_test("metatables.lua");
}

#[test]
fn diff_error_handling() {
    run_differential_test("error_handling.lua");
}

#[test]
fn diff_coroutine() {
    run_differential_test("coroutine.lua");
}
