use std::io::{self, BufRead, Read, Write};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        // REPL mode
        run_repl();
        return;
    }

    match args[1].as_str() {
        "-v" | "--version" => {
            println!("rlua 0.1.0 (Lua 5.1 compatible)");
        }
        "-e" => {
            if args.len() < 3 {
                eprintln!("rlua: '-e' needs argument");
                std::process::exit(1);
            }
            let source = &args[2];
            run_source(source, "=(command line)");
        }
        "-h" | "--help" => {
            println!("Usage: rlua [options] [script [args]]");
            println!("Options:");
            println!("  -e stat  execute string 'stat'");
            println!("  -v       show version information");
            println!("  -h       show this help");
            println!("  -        read from stdin");
        }
        "-" => {
            // Read all of stdin
            let mut source = String::new();
            io::stdin().read_to_string(&mut source).unwrap_or_else(|e| {
                eprintln!("rlua: error reading stdin: {e}");
                std::process::exit(1);
            });
            run_source(&source, "=stdin");
        }
        file => {
            let source = match std::fs::read_to_string(file) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("rlua: cannot open {file}: {e}");
                    std::process::exit(1);
                }
            };
            run_source(&source, file);
        }
    }
}

fn run_source(source: &str, name: &str) {
    let proto = match rlua_compiler::compile_named(source, name) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("rlua: {name}: {e}");
            std::process::exit(1);
        }
    };

    let mut state = rlua_vm::VmState::new();
    rlua_stdlib::register_stdlib(&mut state);

    match rlua_vm::execute(&mut state, proto) {
        Ok(_) => {}
        Err(e) => {
            eprintln!("rlua: {name}: {e}");
            std::process::exit(1);
        }
    }
}

fn run_repl() {
    println!("rlua 0.1.0 (Lua 5.1 compatible)");
    let stdin = io::stdin();
    let mut state = rlua_vm::VmState::new();
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
