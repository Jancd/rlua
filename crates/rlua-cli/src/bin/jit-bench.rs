use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use rlua_core::function::FunctionProto;
use rlua_jit::{JitConfig, TraceLifecycleState};

const DEFAULT_SAMPLES: usize = 10;
const DEFAULT_HOT_THRESHOLD: u32 = 2;
const DEFAULT_SIDE_EXIT_THRESHOLD: u32 = 8;
const TARGET_SPEEDUP: f64 = 2.0;
const WORKLOADS: &[&str] = &[
    "numeric_sum_large.lua",
    "numeric_descending_large.lua",
    "native_side_exit_resume_large.lua",
];

#[derive(Debug, Clone)]
struct BenchCase {
    name: String,
    proto: FunctionProto,
}

#[derive(Debug, Clone)]
struct BenchResult {
    name: String,
    interpreter: Duration,
    jit: Duration,
    speedup: f64,
    debug: rlua_vm::VmJitDebugState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReleaseStatus {
    Pass,
    Investigate,
    Fail,
}

impl ReleaseStatus {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Investigate => "investigate",
            Self::Fail => "fail",
        }
    }
}

fn main() {
    let options = parse_args(std::env::args().skip(1).collect())
        .unwrap_or_else(|err| abort(&format!("jit-bench: {err}")));
    let cases = load_cases(&options.workloads);

    println!("M6 JIT benchmark harness");
    println!(
        "workloads={} samples={} hot_threshold={} side_exit_threshold={}",
        cases.len(),
        options.samples,
        options.hot_threshold,
        options.side_exit_threshold
    );
    println!();
    println!(
        "{:<30} {:>12} {:>12} {:>10}  debug",
        "workload", "interp(ms)", "jit(ms)", "speedup"
    );

    let mut results = Vec::with_capacity(cases.len());
    for case in &cases {
        let interpreter = median_duration((0..options.samples).map(|_| {
            run_case(
                &case.proto,
                JitConfig {
                    enabled: false,
                    hot_threshold: options.hot_threshold,
                    side_exit_threshold: options.side_exit_threshold,
                },
            )
            .0
        }));

        let mut jit_samples = Vec::with_capacity(options.samples);
        let mut jit_debug = None;
        for _ in 0..options.samples {
            let (elapsed, debug) = run_case(
                &case.proto,
                JitConfig {
                    enabled: true,
                    hot_threshold: options.hot_threshold,
                    side_exit_threshold: options.side_exit_threshold,
                },
            );
            jit_samples.push(elapsed);
            jit_debug = Some(debug);
        }
        let jit = median_duration(jit_samples);
        let debug = jit_debug.expect("jit samples produced debug state");
        let speedup = duration_ms(interpreter) / duration_ms(jit).max(f64::MIN_POSITIVE);

        println!(
            "{:<30} {:>12.3} {:>12.3} {:>9.2}x  {}",
            case.name,
            duration_ms(interpreter),
            duration_ms(jit),
            speedup,
            format_debug_summary(&debug)
        );

        results.push(BenchResult {
            name: case.name.clone(),
            interpreter,
            jit,
            speedup,
            debug,
        });
    }

    let median_speedup = median_f64(results.iter().map(|result| result.speedup).collect());
    println!();
    println!("workload set: {}", workload_set(&results));
    println!("median speedup: {:.2}x", median_speedup);

    if median_speedup >= TARGET_SPEEDUP {
        println!("target status: met (>= {:.1}x)", TARGET_SPEEDUP);
    } else {
        println!("target status: below target (< {:.1}x)", TARGET_SPEEDUP);
    }

    let slow_cases: Vec<&BenchResult> = results
        .iter()
        .filter(|result| result.speedup < TARGET_SPEEDUP)
        .collect();
    let release_status = if median_speedup < TARGET_SPEEDUP {
        ReleaseStatus::Fail
    } else if slow_cases.is_empty() {
        ReleaseStatus::Pass
    } else {
        ReleaseStatus::Investigate
    };
    println!("release status: {}", release_status.as_str());

    for result in slow_cases {
        if result.speedup < TARGET_SPEEDUP {
            println!(
                "slow case: {} interp={:.3}ms jit={:.3}ms speedup={:.2}x {}",
                result.name,
                duration_ms(result.interpreter),
                duration_ms(result.jit),
                result.speedup,
                format_debug_summary(&result.debug)
            );
        }
    }
}

#[derive(Debug, Clone)]
struct Options {
    workloads: Vec<String>,
    samples: usize,
    hot_threshold: u32,
    side_exit_threshold: u32,
}

fn parse_args(args: Vec<String>) -> Result<Options, String> {
    let mut workloads = Vec::new();
    let mut samples = DEFAULT_SAMPLES;
    let mut hot_threshold = DEFAULT_HOT_THRESHOLD;
    let mut side_exit_threshold = DEFAULT_SIDE_EXIT_THRESHOLD;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--samples" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--samples requires an integer".to_string())?;
                samples = value
                    .parse::<usize>()
                    .map_err(|_| format!("invalid sample count '{value}'"))?;
                index += 2;
            }
            "--hot-threshold" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--hot-threshold requires an integer".to_string())?;
                hot_threshold = value
                    .parse::<u32>()
                    .map_err(|_| format!("invalid hot threshold '{value}'"))?;
                index += 2;
            }
            "--side-exit-threshold" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--side-exit-threshold requires an integer".to_string())?;
                side_exit_threshold = value
                    .parse::<u32>()
                    .map_err(|_| format!("invalid side-exit threshold '{value}'"))?;
                index += 2;
            }
            "--workload" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--workload requires a benchmark case name".to_string())?;
                workloads.push(value.clone());
                index += 2;
            }
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            other => {
                return Err(format!("unexpected argument '{other}'"));
            }
        }
    }

    if samples == 0 {
        return Err("sample count must be greater than zero".to_string());
    }
    if hot_threshold == 0 {
        return Err("hot threshold must be greater than zero".to_string());
    }
    if side_exit_threshold == 0 {
        return Err("side-exit threshold must be greater than zero".to_string());
    }

    Ok(Options {
        workloads: if workloads.is_empty() {
            WORKLOADS.iter().map(|name| (*name).to_string()).collect()
        } else {
            workloads
        },
        samples,
        hot_threshold,
        side_exit_threshold,
    })
}

fn print_help() {
    println!("Usage: cargo run -p rlua-cli --bin jit-bench -- [options]");
    println!("Options:");
    println!("  --samples N                number of timing samples per mode");
    println!("  --hot-threshold N          JIT hot-loop threshold");
    println!("  --side-exit-threshold N    trace downgrade/invalidation threshold");
    println!("  --workload NAME            benchmark a specific workload (repeatable)");
    println!("  -h, --help                 show this help");
}

fn load_cases(selected: &[String]) -> Vec<BenchCase> {
    selected
        .iter()
        .map(|name| {
            let path = benchmark_path(name);
            let source = std::fs::read_to_string(&path)
                .unwrap_or_else(|err| abort(&format!("cannot read {}: {err}", path.display())));
            let proto = rlua_compiler::compile_named(&source, path.to_str().unwrap())
                .unwrap_or_else(|err| abort(&format!("{}: compile error: {err}", path.display())));

            BenchCase {
                name: trim_lua_suffix(name),
                proto,
            }
        })
        .collect()
}

fn benchmark_path(name: &str) -> PathBuf {
    let manifest = env!("CARGO_MANIFEST_DIR");
    Path::new(manifest)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("benchmarks/jit")
        .join(name)
}

fn trim_lua_suffix(name: &str) -> String {
    name.trim_end_matches(".lua").to_string()
}

fn run_case(proto: &FunctionProto, config: JitConfig) -> (Duration, rlua_vm::VmJitDebugState) {
    let mut state = rlua_vm::VmState::with_jit_config(config);
    rlua_stdlib::register_stdlib(&mut state);

    let start = Instant::now();
    let result = rlua_vm::execute(&mut state, proto.clone());
    let elapsed = start.elapsed();
    result.unwrap_or_else(|err| abort(&format!("benchmark runtime error: {err}")));

    (elapsed, state.jit_debug_state())
}

fn format_debug_summary(debug: &rlua_vm::VmJitDebugState) -> String {
    if debug.traces.is_empty() {
        return format!(
            "traces=0 native={} replay={} exits={} invalidated_bypasses={}",
            debug.stats.native_entries,
            debug.stats.replay_entries,
            debug.stats.side_exits,
            debug.stats.invalidated_bypasses
        );
    }

    let states = debug
        .traces
        .iter()
        .map(|trace| {
            let lifecycle = match trace.lifecycle_state {
                TraceLifecycleState::Active => "active",
                TraceLifecycleState::ReplayOnly => "replay-only",
                TraceLifecycleState::Invalidated => "invalidated",
            };
            let invalidation = trace
                .invalidation_reason
                .map(|reason| format!("{reason:?}"))
                .unwrap_or_else(|| "none".to_string());
            format!(
                "g{}:{}/{}:{}:replay={}:native={}:exits={}:invalidated_bypasses={}",
                trace.generation,
                lifecycle,
                trace.native_state as u8,
                invalidation,
                trace.replay_entries,
                trace.native_entries,
                trace.side_exit_count,
                trace.invalidated_bypasses
            )
        })
        .collect::<Vec<_>>()
        .join(",");

    format!(
        "traces={} native={} replay={} exits={} invalidated_bypasses={} states=[{}]",
        debug.trace_count,
        debug.stats.native_entries,
        debug.stats.replay_entries,
        debug.stats.side_exits,
        debug.stats.invalidated_bypasses,
        states
    )
}

fn workload_set(results: &[BenchResult]) -> String {
    results
        .iter()
        .map(|result| result.name.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

fn median_duration<I>(durations: I) -> Duration
where
    I: IntoIterator<Item = Duration>,
{
    let mut durations: Vec<Duration> = durations.into_iter().collect();
    durations.sort_unstable();
    durations[durations.len() / 2]
}

fn median_f64(mut values: Vec<f64>) -> f64 {
    values.sort_by(|lhs, rhs| lhs.total_cmp(rhs));
    values[values.len() / 2]
}

fn duration_ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

fn abort(message: &str) -> ! {
    eprintln!("{message}");
    std::process::exit(1);
}
