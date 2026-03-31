use std::path::{Path, PathBuf};

use rlua_jit::JitConfig;

#[derive(Clone, Copy)]
enum Surface {
    Parser,
    Compiler,
    Runtime,
}

#[test]
#[ignore = "extended hardening lane"]
fn parser_corpus_replays_without_panics() {
    replay_dir(corpus_dir("parser"), Surface::Parser, false);
}

#[test]
#[ignore = "extended hardening lane"]
fn compiler_corpus_replays_without_panics() {
    replay_dir(corpus_dir("compiler"), Surface::Compiler, true);
}

#[test]
#[ignore = "extended hardening lane"]
fn runtime_corpus_replays_without_panics() {
    replay_dir(corpus_dir("runtime"), Surface::Runtime, true);
}

#[test]
#[ignore = "extended hardening lane"]
fn checked_in_reproducers_replay_by_surface() {
    replay_dir(reproducers_dir().join("parser"), Surface::Parser, false);
    replay_dir(reproducers_dir().join("compiler"), Surface::Compiler, false);
    replay_dir(reproducers_dir().join("runtime"), Surface::Runtime, false);
}

fn replay_dir(dir: PathBuf, surface: Surface, expect_seed_success: bool) {
    for path in lua_files(&dir) {
        let source = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
        replay_surface(&path, &source, surface, expect_seed_success);
        for mutated in deterministic_mutations(&source) {
            replay_surface(&path, &mutated, surface, false);
        }
    }
}

fn replay_surface(path: &Path, source: &str, surface: Surface, expect_success: bool) {
    match surface {
        Surface::Parser => {
            let _ = rlua_parser::parse(source);
        }
        Surface::Compiler => {
            let result = rlua_compiler::compile_named(source, path.to_string_lossy().as_ref());
            if expect_success {
                result.unwrap_or_else(|error| {
                    panic!(
                        "{}: compiler corpus seed should compile successfully: {error}",
                        path.display()
                    )
                });
            }
        }
        Surface::Runtime => {
            let result = rlua_compiler::compile_named(source, path.to_string_lossy().as_ref());
            match (expect_success, result) {
                (true, Ok(proto)) => execute_runtime_seed(path, proto),
                (true, Err(error)) => panic!(
                    "{}: runtime corpus seed should compile successfully: {error}",
                    path.display()
                ),
                (false, Ok(proto)) => {
                    let _ = execute_runtime(proto);
                }
                (false, Err(_)) => {}
            }
        }
    }
}

fn execute_runtime_seed(path: &Path, proto: rlua_core::FunctionProto) {
    execute_runtime(proto).unwrap_or_else(|error| {
        panic!(
            "{}: runtime corpus seed should execute successfully: {error}",
            path.display()
        )
    });
}

fn execute_runtime(proto: rlua_core::FunctionProto) -> Result<Vec<rlua_core::LuaValue>, String> {
    let mut state = rlua_vm::VmState::with_jit_config(JitConfig {
        enabled: false,
        ..JitConfig::default()
    });
    rlua_stdlib::register_stdlib(&mut state);
    rlua_vm::execute(&mut state, proto).map_err(|error| error.to_string())
}

fn deterministic_mutations(source: &str) -> Vec<String> {
    let trimmed = source.trim_end();
    let mut mutations = vec![
        format!("{trimmed}\n"),
        format!("do\n{trimmed}\nend\n"),
        format!("{trimmed}\n-- hardening tail\n"),
    ];
    if !trimmed.is_empty() {
        let split = trimmed.len() / 2;
        mutations.push(trimmed[..split].to_owned());
        mutations.push(format!("{trimmed}\nreturn 0\n"));
    }
    mutations
}

fn corpus_dir(surface: &str) -> PathBuf {
    repo_root().join("tests/fuzz/corpus").join(surface)
}

fn reproducers_dir() -> PathBuf {
    repo_root().join("tests/fuzz/reproducers")
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn lua_files(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", dir.display()))
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            (path.extension().and_then(|ext| ext.to_str()) == Some("lua")).then_some(path)
        })
        .collect();
    files.sort();
    files
}
