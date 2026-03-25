use std::path::Path;

fn run_lua_file(path: &str) {
    let source =
        std::fs::read_to_string(path).unwrap_or_else(|e| panic!("cannot read {path}: {e}"));
    let proto = rlua_compiler::compile_source(&source)
        .unwrap_or_else(|e| panic!("{path}: compile error: {e}"));
    let mut state = rlua_vm::VmState::new();
    rlua_stdlib::register_stdlib(&mut state);
    rlua_vm::execute(&mut state, proto).unwrap_or_else(|e| panic!("{path}: runtime error: {e}"));
}

fn conformance_path(name: &str) -> String {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let p = Path::new(manifest)
        .parent()
        .unwrap() // crates/
        .parent()
        .unwrap() // rlua/
        .join("tests/conformance")
        .join(name);
    p.to_string_lossy().to_string()
}

#[test]
fn arithmetic() {
    run_lua_file(&conformance_path("arithmetic.lua"));
}
#[test]
fn booleans() {
    run_lua_file(&conformance_path("booleans.lua"));
}
#[test]
fn closures() {
    run_lua_file(&conformance_path("closures.lua"));
}
#[test]
fn functions() {
    run_lua_file(&conformance_path("functions.lua"));
}
#[test]
fn generic_for() {
    run_lua_file(&conformance_path("generic_for.lua"));
}
#[test]
fn globals() {
    run_lua_file(&conformance_path("globals.lua"));
}
#[test]
fn if_else() {
    run_lua_file(&conformance_path("if_else.lua"));
}
#[test]
fn locals() {
    run_lua_file(&conformance_path("locals.lua"));
}
#[test]
fn multireturn() {
    run_lua_file(&conformance_path("multireturn.lua"));
}
#[test]
fn numeric_for() {
    run_lua_file(&conformance_path("numeric_for.lua"));
}
#[test]
fn pcall() {
    run_lua_file(&conformance_path("pcall.lua"));
}
#[test]
fn recursion() {
    run_lua_file(&conformance_path("recursion.lua"));
}
#[test]
fn repeat() {
    run_lua_file(&conformance_path("repeat.lua"));
}
#[test]
fn scoping() {
    run_lua_file(&conformance_path("scoping.lua"));
}
#[test]
fn strings() {
    run_lua_file(&conformance_path("strings.lua"));
}
#[test]
fn tables() {
    run_lua_file(&conformance_path("tables.lua"));
}
#[test]
fn while_loop() {
    run_lua_file(&conformance_path("while.lua"));
}
