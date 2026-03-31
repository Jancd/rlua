## Context

M6 closed the release-candidate loop around the existing supported subset, but it also made the remaining semantic gaps much more obvious. The missing pieces are not primarily new optimization work. They are baseline Lua behaviors that users reasonably expect from a 5.1-compatible runtime:

- there is no `coroutine` library or first-class `thread` value
- `tostring()` only honors `__tostring` when the metamethod is native, not when it is a Lua closure
- `table.sort` rejects Lua comparator functions because the current stdlib/native-call path cannot re-enter Lua from native library code

These gaps all share one architectural pressure point: the current `NativeFn` boundary is intentionally simple, but too weak for stateful library operations that need VM cooperation. Coroutine resume/yield, Lua-comparator sorting, and Lua-closure `__tostring` all require native code to call back into Lua or to return a non-standard control-flow outcome.

The design therefore focuses on closing these semantic gaps without expanding JIT scope:
- keep coroutine execution correctness-first
- avoid ad hoc name-based interception for every new library corner case
- preserve the existing interpreter and trace architecture for non-coroutine code

## Goals / Non-Goals

**Goals:**
- Add first-class coroutine objects and the Lua 5.1 `coroutine` library surface.
- Support `resume` / `yield` / `running` / `status` / `wrap` semantics in a way that preserves Lua-visible behavior.
- Let library and metamethod helpers invoke Lua closures from native code where required, specifically for `table.sort` comparators and `__tostring`.
- Keep the change compatible with the current interpreter-first architecture and existing protected-call/error propagation model.
- Add tests that prove these newly closed gaps match reference-Lua-visible behavior where applicable.

**Non-Goals:**
- M7A does not widen the native traced JIT subset.
- M7A does not add coroutine-aware native trace execution, coroutine scheduling primitives beyond the Lua 5.1 library, or cross-thread JIT migration.
- M7A does not redesign the GC into a new ownership model; it only extends the existing model enough to represent coroutine handles safely.
- M7A does not attempt to solve every native-library reentrancy case at once; it solves the VM/native boundary needed for the targeted semantic gaps.

## Decisions

### Decision 1: Coroutines are first-class `thread` values backed by VM-owned execution contexts
- Rationale: Lua-visible coroutine objects must behave like ordinary values (`type(co) == "thread"`, pointer identity, table storage), but the resumable stack/frames/open-upvalues state is VM-specific and should not move into `rlua-core` wholesale.
- Design:
  - Add a new `LuaValue::Thread(...)` variant for user-visible coroutine handles.
  - Keep the actual resumable execution context in the VM layer as a VM-owned structure containing stack, frames, open upvalues, pending-side-exit state, and coroutine lifecycle state.
  - Represent the core-level thread handle as an opaque identity object so tables, equality, and `type()` can treat it as a first-class Lua value without giving `rlua-core` direct knowledge of VM call frames.
  - Update raw equality, table keys, GC root scanning, and debug formatting to recognize thread values.
- Alternatives:
  - Store full frame stacks inside `rlua-core`: rejected because it would drag VM-private execution details into the core crate and increase coupling.
  - Represent coroutines as tables/userdata-like proxies: rejected because it would not match Lua's `thread` type semantics cleanly.

### Decision 2: Introduce a VM-aware native-call context instead of adding more VM call-site special cases
- Rationale: `table.sort` with a Lua comparator and Lua-closure `__tostring` both fail today because `NativeFn` only sees raw arguments and cannot invoke Lua code. Coroutine APIs add the same pressure in a more severe form because `yield` must signal structured control flow rather than just returning values or errors.
- Design:
  - Replace the current purely stateless native-call contract with a VM-aware context abstraction implemented by the VM layer.
  - The context provides the minimum operations native helpers need:
    - invoke an arbitrary Lua value as a function
    - inspect or update source-location-sensitive behavior when needed
    - signal coroutine yield/resume control flow
  - Keep simple math/string/table helpers thin by treating context access as optional in the common case, but make it available uniformly.
  - Preserve the existing pcall/xpcall/error interception model only where it is semantically special, not as a template for every new library feature.
- Alternatives:
  - Add more hard-coded name checks in the VM for `tostring`, `table.sort`, `coroutine.resume`, and `coroutine.yield`: rejected because it scales poorly and keeps library semantics fragmented between stdlib and VM opcode handlers.
  - Change stdlib APIs to avoid Lua re-entry: rejected because it would preserve the current compatibility gaps instead of closing them.

### Decision 3: Execution returns a structured outcome so `yield` is not encoded as an error
- Rationale: Lua `yield` is not an error path. Treating it as one would entangle coroutine control flow with `pcall`/`xpcall` and distort Lua-visible semantics.
- Design:
  - Introduce an execution/call outcome type that can represent:
    - normal return values
    - yielded values
    - runtime error
  - Thread resume logic consumes yielded outcomes by suspending the current coroutine context and returning `(true, ...)` from `coroutine.resume`.
  - Attempts to yield across unsupported boundaries remain explicit runtime errors, but ordinary coroutine suspension does not flow through `LuaError`.
  - Reuse this outcome type at the interpreter/native boundary so native helpers that need to yield or re-enter Lua can do so without inventing sentinel values.
- Alternatives:
  - Encode yield as a special string or tagged `LuaError`: rejected because it makes protected calls and handler logic brittle and obscures the distinction between suspension and failure.

### Decision 4: Coroutine execution is interpreter-only in M7A
- Rationale: the proposal explicitly freezes JIT scope. Coroutines would otherwise force immediate decisions about trace recording across resume points, side exits into suspended contexts, and deopt maps spanning yielded frames.
- Design:
  - Mark coroutine-owned execution contexts as JIT-ineligible in M7A.
  - Keep the existing JIT behavior unchanged for the main interpreter path and existing supported workloads.
  - If coroutine code executes hot loops, it remains interpreter-correct rather than trace-optimized.
  - Document this as an implementation boundary, not as a silent semantic difference.
- Alternatives:
  - Allow tracing inside coroutines immediately: rejected because it turns a semantic compatibility change into a combined coroutine + deopt/JIT architecture project.
  - Disable JIT globally whenever coroutines exist: rejected because it would unnecessarily regress existing supported workloads.

### Decision 5: `table.sort` comparator and `__tostring` closure support use the same Lua-callback path
- Rationale: these two gaps are symptoms of the same architectural problem. Solving them separately would likely duplicate bespoke “call Lua from native” plumbing.
- Design:
  - Route `table.sort` comparator invocation through the new VM-aware native-call context so comparator functions may be native or Lua closures.
  - Route `tostring()` metamethod dispatch through the same path so `__tostring` may be native or Lua.
  - Keep result-shape validation local to each feature:
    - `table.sort` interprets comparator truthiness the Lua way
    - `tostring()` requires a string-like result and reports a Lua-facing runtime error when the metamethod violates expectations
- Alternatives:
  - Keep `tostring()` and `table.sort` as separate ad hoc exceptions: rejected because it would bypass the reusable boundary this change needs anyway.

## Risks / Trade-offs

- [Coroutine handle vs VM context split may complicate ownership] -> Mitigate by keeping the handle opaque and centralizing context lookup/lifecycle transitions in the VM.
- [Refactoring the native-call boundary touches many stdlib functions] -> Mitigate by introducing a narrow context abstraction and preserving simple adapters for stateless helpers.
- [Yield semantics may interact subtly with `pcall`/`xpcall` and error handlers] -> Mitigate by using a structured execution outcome and adding targeted protected-call coroutine tests.
- [Interpreter-only coroutine execution may surprise users expecting JIT speedups] -> Mitigate by documenting the boundary explicitly and preserving correct fallback semantics rather than partial unsupported tracing.
- [Adding `thread` as a new value kind expands raw equality, table-key, and GC surfaces] -> Mitigate by auditing all `LuaValue` matches and adding focused unit tests for thread identity, storage, and root scanning.

## Migration Plan

1. Add the `thread` value kind and opaque coroutine handle support in `rlua-core`, including equality, type reporting, table storage, and GC-marking integration.
2. Extract or introduce the VM-owned execution-context abstraction needed to suspend and resume non-root execution contexts safely.
3. Replace the native-call boundary with the new VM-aware call context and structured execution outcome.
4. Implement the `coroutine` standard library on top of the new thread/context model, keeping coroutine execution interpreter-only.
5. Update `tostring()` and `table.sort` to use the shared Lua-callback path.
6. Expand conformance, differential, and regression coverage, then remove the corresponding documented limitations from release-facing docs.

Rollback strategy:
- The coroutine library can be withheld while retaining the VM-aware native-call refactor if the semantic surface proves too unstable.
- `table.sort` and `__tostring` closure support can land ahead of full coroutine support because they depend on the same call-context foundation but do not require suspended execution contexts.

## Open Questions

- Should the new VM-aware native-call context live as a trait in `rlua-core`, or should the project move `LuaFunction::Native` out of `rlua-core` to avoid trait-based indirection?
- Do we want `coroutine.wrap` to be implemented directly as a thread-backed wrapper closure in M7A, or as a thin library helper layered on `create`/`resume` after the core API works?
- Should yielding across metamethod or comparator callbacks be forbidden initially, or supported wherever the call stack is coroutine-owned and interpreter-only?
