use std::io::{self, BufRead, Read, Write};

use rlua_jit::JitConfig;

enum InputMode {
    Repl,
    Version,
    Help,
    Eval(String),
    Stdin,
    File(String),
}

struct CliOptions {
    jit: JitConfig,
    input: InputMode,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let options = parse_args(&args).unwrap_or_else(|err| {
        eprintln!("rlua: {err}");
        std::process::exit(1);
    });

    match options.input {
        InputMode::Version => println!("rlua 0.1.0 (Lua 5.1 compatible)"),
        InputMode::Help => print_help(),
        InputMode::Eval(source) => run_source(&source, "=(command line)", options.jit),
        InputMode::Stdin => {
            let mut source = String::new();
            io::stdin().read_to_string(&mut source).unwrap_or_else(|e| {
                eprintln!("rlua: error reading stdin: {e}");
                std::process::exit(1);
            });
            run_source(&source, "=stdin", options.jit);
        }
        InputMode::File(file) => {
            let source = match std::fs::read_to_string(&file) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("rlua: cannot open {file}: {e}");
                    std::process::exit(1);
                }
            };
            run_source(&source, &file, options.jit);
        }
        InputMode::Repl => run_repl(options.jit),
    }
}

fn parse_args(args: &[String]) -> Result<CliOptions, String> {
    let mut jit = JitConfig::default();
    let mut input = None;
    let mut index = 1;

    while index < args.len() {
        let arg = &args[index];
        match arg.as_str() {
            "-v" | "--version" => {
                input = Some(InputMode::Version);
                break;
            }
            "-h" | "--help" => {
                input = Some(InputMode::Help);
                break;
            }
            "--no-jit" => {
                jit.enabled = false;
                index += 1;
            }
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
            "--hot-threshold" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--hot-threshold requires an integer".to_string())?;
                jit.hot_threshold = parse_hot_threshold(value)?;
                index += 2;
            }
            option if option.starts_with("--hot-threshold=") => {
                jit.hot_threshold = parse_hot_threshold(&option["--hot-threshold=".len()..])?;
                index += 1;
            }
            "-e" => {
                let source = args
                    .get(index + 1)
                    .ok_or_else(|| "'-e' needs argument".to_string())?;
                input = Some(InputMode::Eval(source.clone()));
                break;
            }
            "-" => {
                input = Some(InputMode::Stdin);
                break;
            }
            file => {
                input = Some(InputMode::File(file.to_owned()));
                break;
            }
        }
    }

    Ok(CliOptions {
        jit,
        input: input.unwrap_or(InputMode::Repl),
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

fn parse_hot_threshold(value: &str) -> Result<u32, String> {
    let threshold = value
        .parse::<u32>()
        .map_err(|_| format!("invalid hot threshold '{value}'"))?;
    if threshold == 0 {
        return Err("hot threshold must be greater than zero".to_string());
    }
    Ok(threshold)
}

fn print_help() {
    println!("Usage: rlua [options] [script [args]]");
    println!("Options:");
    println!("  -e stat              execute string 'stat'");
    println!("  --jit on|off         enable or disable JIT recording");
    println!("  --no-jit             shorthand for '--jit off'");
    println!("  --hot-threshold N    set loop hotness threshold");
    println!("  -v, --version        show version information");
    println!("  -h, --help           show this help");
    println!("  -                    read from stdin");
}

fn run_source(source: &str, name: &str, jit: JitConfig) {
    let proto = match rlua_compiler::compile_named(source, name) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("rlua: {name}: {e}");
            std::process::exit(1);
        }
    };

    let mut state = rlua_vm::VmState::with_jit_config(jit);
    rlua_stdlib::register_stdlib(&mut state);

    match rlua_vm::execute(&mut state, proto) {
        Ok(_) => {}
        Err(e) => {
            eprintln!("rlua: {name}: {e}");
            std::process::exit(1);
        }
    }
}

fn run_repl(jit: JitConfig) {
    println!("rlua 0.1.0 (Lua 5.1 compatible)");
    let stdin = io::stdin();
    let mut state = rlua_vm::VmState::with_jit_config(jit);
    rlua_stdlib::register_stdlib(&mut state);

    loop {
        print!("> ");
        io::stdout().flush().unwrap();

        let mut line = String::new();
        match stdin.lock().read_line(&mut line) {
            Ok(0) => break, // EOF
            Ok(_) => {}
            Err(e) => {
                eprintln!("rlua: {e}");
                break;
            }
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Try as expression first (prefix with "return")
        let source = if !trimmed.starts_with("return")
            && !trimmed.contains('=')
            && !trimmed.starts_with("if")
            && !trimmed.starts_with("for")
            && !trimmed.starts_with("while")
            && !trimmed.starts_with("repeat")
            && !trimmed.starts_with("do")
            && !trimmed.starts_with("local")
            && !trimmed.starts_with("function")
        {
            format!("return {trimmed}")
        } else {
            trimmed.to_owned()
        };

        let proto = match rlua_compiler::compile_named(&source, "stdin") {
            Ok(p) => p,
            Err(_) => {
                // Try original if return-prefix failed
                match rlua_compiler::compile_named(trimmed, "stdin") {
                    Ok(p) => p,
                    Err(e) => {
                        eprintln!("  {e}");
                        continue;
                    }
                }
            }
        };

        match rlua_vm::execute(&mut state, proto) {
            Ok(results) => {
                if !results.is_empty() {
                    let parts: Vec<String> = results.iter().map(|v| v.to_lua_string()).collect();
                    println!("{}", parts.join("\t"));
                }
            }
            Err(e) => {
                eprintln!("  {e}");
            }
        }
    }
}
