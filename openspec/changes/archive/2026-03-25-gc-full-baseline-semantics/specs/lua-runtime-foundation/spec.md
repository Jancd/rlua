## Capability: lua-runtime-foundation (MODIFIED)

GC requirement upgraded from stub to integrated mark-sweep with actual object tracking; metatable field added to table value model; tail call optimization implemented.

### Requirement: Metatable Field on LuaTable

MODIFIED: `LuaTable` gains an `Option<TableRef>` field to store its metatable.

#### Scenario: Table created without metatable

WHEN a new table is created
THEN its metatable field is `None`

#### Scenario: Table metatable storage

WHEN `setmetatable(t, mt)` sets a metatable on table `t`
THEN `t`'s internal metatable field holds a reference to `mt`
AND the metatable is accessible for metamethod lookups during operations

### Requirement: GC Upgraded from Stub to Functional

MODIFIED: The `MarkSweepGc` in `rlua-core` is upgraded from a phase-transition-only stub to a functional collector with allocation tracking, root scanning through `GcRootProvider`, and sweep-phase statistics. Actual memory reclamation is still handled by `Rc<RefCell<>>` — the GC infrastructure enables future replacement.

#### Scenario: VM integrates with GC

WHEN the VM runs
THEN it implements `GcRootProvider` to expose stack, globals, and open upvalues as roots
AND allocation operations notify the GC
AND collection cycles run at safepoints when thresholds are exceeded

### Requirement: Tail Call Optimization

The `TAILCALL` opcode reuses the current call frame instead of pushing a new one, preventing stack growth for tail-recursive functions.

#### Scenario: Tail call reuses frame

WHEN a function ends with `return f(args)` compiled as TAILCALL
THEN the VM reuses the current call frame (same base register, same frame slot)
AND the call stack does not grow

#### Scenario: Deep tail recursion

WHEN a tail-recursive function is called 10000+ times
THEN execution completes without stack overflow
AND only one call frame is used for the recursive chain

#### Scenario: Non-tail calls unaffected

WHEN a function call is NOT in tail position (e.g., `local x = f(); return x`)
THEN a normal CALL is used and a new call frame is pushed

### Requirement: String Metatable

MODIFIED: All string values share a single metatable whose `__index` points to the `string` module table. This enables method syntax like `s:sub(1, 3)`.

#### Scenario: String method syntax works

WHEN `("hello"):sub(1, 3)` or `s:upper()` is called on a string
THEN the string metatable's `__index` resolves to the `string` module table
AND the corresponding string function is called with the string as the first argument
