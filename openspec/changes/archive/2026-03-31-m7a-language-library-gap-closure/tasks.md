## 1. Thread Value Model

- [x] 1.1 Add an opaque thread handle type and `LuaValue::Thread` so coroutine objects are first-class Lua values.
- [x] 1.2 Update type reporting, raw equality, table storage/key behavior, debug formatting, and GC/root traversal for thread values.

## 2. VM Call Boundary

- [x] 2.1 Introduce a structured execution outcome that distinguishes normal return, coroutine yield, and runtime error.
- [x] 2.2 Refactor the native function boundary to expose a VM-aware call context that can invoke Lua callbacks from stdlib and metamethod helpers.
- [x] 2.3 Adapt protected-call and top-level dispatch paths so yield and error semantics remain Lua-visible and distinct.

## 3. Coroutine Runtime and Library

- [x] 3.1 Add VM-owned coroutine execution contexts, lifecycle state tracking, and resume/suspend bookkeeping.
- [x] 3.2 Implement `coroutine.create`, `coroutine.resume`, `coroutine.yield`, `coroutine.status`, and `coroutine.running`.
- [x] 3.3 Implement `coroutine.wrap` and register the global `coroutine` library while keeping coroutine execution interpreter-only.

## 4. Library and Metamethod Gap Closure

- [x] 4.1 Route `table.sort` comparator calls through the VM-aware Lua callback path for both native and Lua comparators.
- [x] 4.2 Route `tostring()` `__tostring` dispatch through the same callback path and validate the metamethod result shape.
- [x] 4.3 Ensure comparator and metamethod callback failures propagate as Lua-facing errors, and define the runtime behavior for unsupported yield crossings explicitly.

## 5. Compatibility Coverage and Verification

- [x] 5.1 Add conformance tests for coroutine lifecycle, main-thread yield errors, suspended/dead status, and `coroutine.wrap`.
- [x] 5.2 Add differential and regression coverage for `table.sort` with Lua comparators, Lua-closure `__tostring`, and JIT-enabled safe fallback behavior.
- [x] 5.3 Remove the corresponding known limitations from release-facing docs and update documentation for the new coroutine support boundary.
- [x] 5.4 Run OpenSpec validation plus targeted and workspace-level test verification for M7A.
