## Context

M4 established the first end-to-end native trace path for a narrow numeric hot-loop subset: the VM records traces, optimizes them, installs x86_64 native artifacts into the trace cache, and prefers native execution while retaining replay/interpreter fallback. That path is correct enough to demonstrate native execution, but its deoptimization and lifecycle behavior is still intentionally minimal.

Today, deopt state is mostly represented as guard `resume_pc` metadata plus ad hoc slot synchronization from native execution back into the VM. Replay and native execution both rely on the same cached trace entry, but the cache has no invalidation model beyond "installed or not". The VM only suppresses immediate re-entry through a single `pending_side_exit` key, which is enough for M4 correctness smoke tests but not for repeated side exits, stale assumptions, or future expansion of the supported trace subset. The repository also has no dedicated benchmark harness yet, so M5 cannot prove the target median speedup against interpreter baseline.

M5 therefore needs to harden three things together:
- deopt correctness as a first-class runtime contract
- trace invalidation and cache lifecycle stability
- benchmark-driven performance measurement and tuning for the supported traced subset

## Goals / Non-Goals

**Goals:**
- Define a deopt model that reconstructs interpreter-visible state deterministically for every supported native or replay side exit.
- Add explicit trace lifecycle state and invalidation rules so the runtime can retire, bypass, or rebuild stale traces safely.
- Expand diagnostics and debug state so exits, invalidations, and recompilation behavior are observable in tests and benchmarks.
- Add a benchmark harness for the M5-supported hot-loop set and use it to drive tuning toward the `>= 2x` median speedup goal.

**Non-Goals:**
- This design does not expand the supported trace language to tables, calls, metatables, or polymorphic traces.
- This design does not introduce a multithreaded JIT runtime or shared cross-thread trace cache.
- This design does not commit to a sophisticated global optimizer or register allocator; tuning remains targeted to the existing supported subset.
- This design does not require raw machine-code snapshot stability across toolchains or operating systems.

## Decisions

### Decision 1: Deopt state becomes an explicit artifact derived from optimized traces
- Rationale: M4 can only resume through `resume_pc` and a small set of synchronized numeric slots. That is too implicit to scale to repeated exits or richer supported traces. M5 should derive a deopt/resume map from the optimized trace that records which VM-visible slots, loop temporaries, and guard exits must be restored at each supported side exit.
- Design:
  - Extend optimized trace artifacts with deopt metadata keyed by guard/exit site.
  - Treat replay and native execution as two consumers of the same resume contract instead of maintaining separate, loosely equivalent exit behavior.
  - Keep the interpreter as the semantic source of truth; deopt metadata exists to reconstruct interpreter state, not to define new semantics.
- Alternatives:
  - Continue relying only on `resume_pc` plus opportunistic slot writes: too fragile and hard to validate as the subset expands.
  - Introduce per-backend custom deopt schemes: would split correctness logic across replay and native paths.

### Decision 2: Trace cache entries gain explicit lifecycle and invalidation state
- Rationale: the current cache only distinguishes presence/absence and native installation status. M5 needs to model whether a trace is active, exited too often, invalidated by unsupported runtime change, or eligible for recompilation.
- Design:
  - Add per-trace lifecycle metadata such as generation/version, exit counters, invalidation reason, and last execution mode.
  - Route invalidation through `JitRuntime` so the VM does not mutate cache state ad hoc.
  - Prefer soft invalidation first: mark the trace unusable for native entry, fall back to replay/interpreter, and allow future recompilation if the loop becomes hot again.
- Alternatives:
  - Delete traces immediately on any exit anomaly: simple, but destroys observability and makes tuning noisy.
  - Keep traces forever and only bypass them in the VM: stale entries accumulate and diagnostics become misleading.

### Decision 3: Side-exit policy is counter-based, not just single-shot suppression
- Rationale: `pending_side_exit` only prevents immediate re-entry once. It does not tell us whether a trace is unstable, repeatedly deopting, or should be cooled down before recompilation.
- Design:
  - Track per-trace side-exit frequency and recent failure mode in runtime stats/debug state.
  - Introduce policy thresholds for actions such as replay-only downgrade, temporary blacklist, or invalidation.
  - Preserve deterministic interpreter fallback even when a trace is being cooled down or rebuilt.
- Alternatives:
  - Keep one-bit suppression forever: insufficient for robustness and tuning.
  - Permanently blacklist on first side exit: too conservative and would hide recoverable hot loops.

### Decision 4: Benchmarking is a dedicated harness, not implicit timing inside tests
- Rationale: correctness tests should stay deterministic and cheap, while M5 performance work needs repeatable interpreter-vs-JIT comparisons on a known workload set.
- Design:
  - Add a separate benchmark capability and harness for supported hot-loop scripts, with both interpreter-only and JIT-enabled modes.
  - Report median speedup, per-case results, and enough debug counters to explain why a benchmark failed to speed up.
  - Keep benchmark code outside the runtime hot path; use existing host/JIT controls and debug state rather than introducing benchmark-only runtime branches.
- Alternatives:
  - Encode timing assertions inside `cargo test`: likely flaky across CI and developer machines.
  - Tune only from ad hoc local scripts: not auditable and does not satisfy the M5 deliverable.

### Decision 5: Tuning work stays constrained to the existing supported trace subset
- Rationale: M5 needs robustness and measurable speedup, not uncontrolled surface-area expansion. Keeping tuning scoped to the M4 subset makes benchmark data actionable and deopt/invalidation logic easier to validate.
- Design:
  - Optimize trace entry/exit overhead, slot materialization, native/replay dispatch, and invalidation heuristics for the current numeric loop subset.
  - Defer broader trace-language coverage to later milestones once deopt and invalidation contracts are stable.
- Alternatives:
  - Expand trace coverage and tune simultaneously: too many moving parts for one milestone.

## Risks / Trade-offs

- [Deopt metadata duplicates information already present in trace IR] → Keep the deopt map derived, compact, and tied to optimized trace generation rather than introducing a second hand-maintained source of truth.
- [Invalidation policy may become too aggressive and erase speedups] → Start with observable counters and soft invalidation states before introducing permanent eviction.
- [Benchmark results may vary by machine or CI noise] → Use benchmark harnesses primarily for regression detection and developer tuning; keep hard CI pass/fail thresholds narrow and justified.
- [More runtime state increases implementation complexity] → Centralize lifecycle state in `JitRuntime` and expose it through existing debug/diagnostic surfaces instead of scattering flags across VM execution code.

## Migration Plan

1. Extend optimized/native trace artifacts and runtime debug state with explicit deopt and lifecycle metadata.
2. Implement invalidation and side-exit policy handling in `JitRuntime`, then route VM native/replay dispatch through that policy.
3. Add deopt/invalidation regression tests and diagnostics for the supported trace subset.
4. Introduce the benchmark harness and benchmark-oriented reporting for interpreter vs JIT execution.
5. Tune the supported hot-loop path using benchmark feedback, then lock in the updated quality gates and tasks.

Rollback strategy:
- New lifecycle and benchmark code can be disabled without removing M4 native execution.
- If invalidation policy proves unstable, the runtime can temporarily downgrade to replay/interpreter preference while preserving the rest of the M5 scaffolding.

## Open Questions

- Should replay and native execution share one deopt map structure exactly, or should replay keep a thinner adapter over the same underlying metadata?
- What invalidation reasons should be host-visible in debug state versus diagnostics-only?
- Should the benchmark harness live under `bench/`, a new crate, or a test-adjacent developer tool that is excluded from normal CI runs?
