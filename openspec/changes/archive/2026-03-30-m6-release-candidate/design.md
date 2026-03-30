## Context

M5 left the project in a strong milestone state: the interpreter passes the existing conformance and differential layers, the tracing JIT has deopt/invalidation hardening, and the benchmark harness demonstrates a clear speedup on the supported numeric subset. That is enough for a milestone deliverable, but not yet enough for a release candidate.

The current repository still has three release-facing gaps. First, there is no single authoritative RC document that explains the supported Lua/JIT subset, architecture constraints, benchmark expectations, and known limitations. Second, developers can inspect JIT state from tests and ad hoc diagnostics, but there is no stable host-facing inspection tool for triaging cache state, invalidation, or fallback behavior from the CLI. Third, the project has strong validation ingredients, but the final release sweep is not yet packaged as one explicit RC contract that ties conformance, regressions, diagnostics, and benchmarks together.

M6 therefore focuses on hardening and packaging the current scope, not expanding it:
- codify what the release candidate supports and what remains out of scope
- expose trace/runtime state through stable tooling rather than only feature-gated logs
- define a final release-validation path that is auditable and repeatable

## Goals / Non-Goals

**Goals:**
- Produce release-facing documentation for the supported interpreter and JIT subset, including documented limitations and validation guidance.
- Add trace inspection tooling that can expose cached trace state, invalidation/fallback behavior, and benchmark-relevant counters without requiring custom test code.
- Tighten quality gates so the RC is evaluated by an explicit final sweep across conformance, regressions, diagnostics, and benchmark validation.
- Use the M6 sweep to fix correctness or stability bugs discovered inside the already-supported scope.

**Non-Goals:**
- M6 does not expand the supported Lua language surface beyond the subset already delivered by M5.
- M6 does not introduce new JIT backends, polymorphic tracing, or broader trace-language coverage.
- M6 does not commit to production packaging beyond an RC-level deliverable and repository documentation.
- M6 does not turn feature-gated diagnostics into always-on runtime behavior.

## Decisions

### Decision 1: Release-candidate documentation is versioned in-repo and treated as a deliverable
- Rationale: the roadmap already calls for a documented RC with known limitations. Today that information is implicit across specs, tests, and code. M6 should gather it into a small set of repository-facing documents that can be reviewed alongside code.
- Design:
  - Add RC documentation covering supported functionality, known limitations, architecture constraints, benchmark expectations, and validation commands.
  - Treat benchmark expectations and diagnostics guidance as release-facing material, not only developer tribal knowledge.
  - Keep the docs tightly scoped to the supported subset rather than writing aspirational documentation for future milestones.
- Alternatives:
  - Keep the information only in OpenSpec artifacts: insufficient for an RC consumer or contributor workflow.
  - Write extensive user docs for unsupported features: misleading and likely to drift from reality.

### Decision 2: Trace inspection becomes a stable CLI surface, separate from feature-gated logs
- Rationale: feature-gated diagnostics are useful for development builds, but RC triage needs a stable way to inspect cached trace state, invalidation reasons, execution mode transitions, and benchmark-relevant counters from normal host tooling.
- Design:
  - Add a CLI-visible inspection path that can run a script and emit structured or human-readable trace summaries derived from existing `VmJitDebugState`.
  - Reuse the runtime’s existing debug-state surfaces instead of introducing a second parallel tracing/inspection stack.
  - Preserve feature-gated diagnostics for fine-grained event logging, but make RC-grade state inspection possible without them.
- Alternatives:
  - Extend debug logging only: hard to consume systematically and poor for release triage.
  - Expose internal Rust-only APIs without CLI tooling: too inconvenient for routine RC validation.

### Decision 3: The release sweep is an explicit contract, not an informal checklist
- Rationale: the repository already has conformance, differential, JIT regression, diagnostics, and benchmark assets. What is missing is an RC contract that says which of those checks define readiness and how failures should be interpreted.
- Design:
  - Define a final validation path that includes conformance/regression coverage, diagnostics/inspection validation, and benchmark validation.
  - Require the release docs to point to the same validation path so the project has one auditable definition of RC readiness.
  - Keep the validation commands simple and repository-native so they can run in CI and on developer machines.
- Alternatives:
  - Rely only on milestone-local commands remembered by contributors: too easy to drift and hard to audit.
  - Make the benchmark gate purely advisory again: inconsistent with an RC deliverable.

### Decision 4: M6 bug-fix work is constrained to hardening inside the supported subset
- Rationale: an RC pass should remove surprises, not add new scope. M6 must stay focused on fixing defects found by the final sweep and on improving debuggability/documentation around the existing supported subset.
- Design:
  - Prioritize bug fixes revealed by conformance, differential, JIT regression, inspection, and benchmark validation.
  - Reject feature work that would materially change the supported subset or require a fresh milestone of semantics work.
  - Use documented limitations for known out-of-scope behavior rather than attempting last-minute expansion.
- Alternatives:
  - Accept opportunistic feature additions during RC hardening: destabilizes the release target.
  - Freeze bug fixes entirely: unrealistic for a meaningful release sweep.

## Risks / Trade-offs

- [RC docs may drift from implementation] → Mitigate by generating them from the current supported subset and tying validation commands directly to repository entrypoints.
- [Trace inspection CLI may overlap with existing diagnostics] → Keep inspection focused on stable state summaries, while feature-gated diagnostics remain event-oriented.
- [Final validation gate may become too heavy for normal iteration] → Keep the RC sweep explicit and documented as a release path, while preserving narrower commands for day-to-day development.
- [Hardening work may reveal issues late] → Treat M6 as the place to fix them, but constrain fixes to the already-supported scope.

## Migration Plan

1. Add RC-oriented documentation and limitation reporting for the supported interpreter/JIT subset.
2. Add trace inspection tooling on top of the existing runtime debug state and benchmark surfaces.
3. Expand specs/tasks/tests/docs so the final validation sweep is explicit and repeatable.
4. Fix any issues discovered by the release sweep inside the supported subset.
5. Run the full RC validation path and freeze the documented limitations for the candidate release.

Rollback strategy:
- Documentation and inspection tooling can be refined or reduced without removing the M5 runtime improvements.
- If a new inspection surface proves too unstable, the project can temporarily fall back to narrower CLI output while preserving the underlying runtime debug-state model.

## Open Questions

- Should the trace inspection tool default to a human-readable summary, structured JSON, or support both from the start?
- Which limitations belong in top-level release docs versus OpenSpec-only notes?
- Should the RC validation path include a dedicated reference-Lua differential rerun for JIT workloads, or keep JIT validation separate from the broader differential suite?
