## Context

M1 delivered a working Lua 5.1 interpreter with 116 tests passing, but it operates at a "raw" level — no metatables, no standard libraries beyond basic globals, and GC exists only as an interface stub with `Rc<RefCell<>>` managing all heap objects. The VM performs direct operations without metamethod fallback, meaning any Lua program that relies on OOP patterns (metatables), string manipulation, or math functions will fail. M2 transforms this into a semantically complete Lua 5.1 interpreter.

Key constraints inherited from M1:
- Zero external dependencies in core crates (std only).
- `NativeFn = fn(args: &[LuaValue]) -> Result<Vec<LuaValue>, String>` is the native function signature.
- `Rc<RefCell<>>` is used for all heap objects (tables, closures, strings).
- 8-crate workspace: rlua-core, rlua-parser, rlua-compiler, rlua-vm, rlua-ir, rlua-jit, rlua-stdlib, rlua-cli.

## Goals / Non-Goals

**Goals:**
- Full metatable system with all Lua 5.1 metamethods dispatched from the VM.
- String, math, and table standard libraries registered as module tables.
- Lua pattern matching engine for string.find/match/gmatch/gsub.
- xpcall with custom error handler.
- GC integration: allocation tracking, mark-phase root traversal, sweep phase reclamation.
- Tail call optimization (TAILCALL reuses frame).
- Differential test harness comparing output against reference Lua 5.1.
- Conformance test coverage for all new features.

**Non-Goals:**
- Full `Rc<RefCell<>>` replacement with GC-managed pointers (too invasive; deferred to post-M2).
- Coroutines (complex stack switching; deferred to M3 or later).
- `io`/`os`/`debug` standard libraries (not needed for semantic confidence).
- `string.format` full printf compatibility (support `%d`, `%s`, `%f`, `%g`, `%x`, `%%`; skip `%e`, `%q`, width/precision modifiers initially).
- Performance optimization of table hash part (linear scan is correct, optimize later).
- `loadstring`/`dofile`/`require` (runtime compilation; deferred).

## Decisions

### Decision 1: Keep Rc<RefCell<>> for M2, deepen GC skeleton

**Choice**: Wire allocation counting and mark-phase root traversal through the existing `MarkSweepGc`, but do NOT replace `Rc<RefCell<>>` with raw GC pointers.

**Rationale**: Replacing Rc with GC pointers requires rewriting every value access across the entire codebase — a multi-thousand-line refactor that risks M2 scope explosion. The existing Rc semantics are correct (objects are reclaimed when unreachable). The GC deepening gives us the infrastructure (allocation tracking, root scanning, phase transitions) that the full GC replacement in a future milestone can build on.

**Alternatives considered**:
- Full GC-managed heap: Correct long-term, but 3-4 week effort that delays all other M2 features. Rc already handles cycle-free object graphs correctly.
- No GC work at all: Violates the M2 spec requirement.

### Decision 2: Metatable field on LuaTable, helper functions for dispatch

**Choice**: Add `metatable: Option<TableRef>` field to `LuaTable`. Extract metamethod dispatch into helper functions (`call_metamethod_binary`, `call_metamethod_unary`, `index_with_metamethod`, `newindex_with_metamethod`) that each opcode handler calls on failure.

**Rationale**: Lua 5.1 metatables are per-table (and per-userdata, but we have no userdata). Inline metamethod checks in every opcode handler would bloat the VM. Helper functions keep the dispatch loop readable while handling the full metamethod resolution chain (__index can be a table or function, chains can be recursive).

**Alternatives considered**:
- Trait-based dispatch: Over-engineered for Lua's dynamic dispatch model.
- Inline in each handler: 15+ opcode handlers would each grow by 20-30 lines.

### Decision 3: Module tables as global tables with function fields

**Choice**: Register `string`, `math`, `table` as global tables. Each contains native function values. Example: `math.floor` is `globals["math"]["floor"] = NativeFn`.

**Rationale**: This matches standard Lua 5.1 behavior. Users access these as `string.sub(s, 1, 3)` or via method syntax `s:sub(1, 3)` (the latter requires string metatable with `__index = string`).

**Implementation**: In `register_stdlib`, create a table for each module, populate with native functions, set as global. For string methods via `:` syntax, set string metatable `__index` to point to the string module table.

**Alternatives considered**:
- Lazy module loading: Unnecessary complexity for built-in modules.

### Decision 4: Lua pattern matching as a standalone engine

**Choice**: Implement Lua patterns as a self-contained matching engine in rlua-stdlib. Lua patterns are NOT regular expressions — they support character classes (`%a`, `%d`, `%w`), captures with `()`, anchors `^$`, and quantifiers `*+?-` but NO alternation or backreferences.

**Rationale**: Lua patterns are simple enough to implement directly (no NFA/DFA needed). A recursive backtracking matcher in ~200-300 lines handles the full Lua 5.1 pattern spec. No external regex crate needed.

### Decision 5: xpcall via VM-level error handler stack

**Choice**: Add an error handler field to the protected call mechanism. When `xpcall(f, handler)` is invoked, the VM stores `handler` and invokes it on error before returning the result.

**Rationale**: Similar to the existing pcall special-case in the CALL opcode. The handler is called with the error object and its return value becomes the error result of xpcall.

### Decision 6: Phased implementation order

**Choice**: Metatables → stdlib (math, table, string) → GC deepening → error handling → tail calls → testing.

**Rationale**: Metatables unlock string method syntax (`s:sub()`), so they must come first. Math/table are quick wins. String library is the largest single effort (pattern matching). GC and error handling are independent. Tail calls are a small optimization. Testing runs throughout.

## Risks / Trade-offs

- [Rc<RefCell<>> cannot collect cycles] → Lua table cycles will leak memory. Mitigation: Lua 5.1 programs rarely create true reference cycles without weak tables. Acceptable for M2; full GC in future milestone resolves this.
- [Pattern matching edge cases] → Lua patterns have subtle behaviors (e.g., `%bxy` balanced match, frontier `%f`). Mitigation: Implement core patterns first, add edge cases driven by conformance test failures.
- [Metamethod infinite recursion] → `__index` returning a table with its own `__index` can recurse. Mitigation: Add a depth limit (Lua 5.1 uses ~200) and raise error on exceed.
- [String metatable global state] → All strings share one metatable. Mitigation: Store string metatable reference in VmState, apply during string operations.
- [VM handler code growth] → Adding metamethod fallbacks to 15+ handlers increases vm/lib.rs significantly. Mitigation: Extract helpers aggressively, keep handlers as thin dispatch + fallback.

## Open Questions

- Should `string.format` support width/precision specifiers (`%5.2f`) in M2 or defer? Leaning defer — most Lua programs use basic `%d`/`%s`/`%f`.
- Should we implement `__gc` metamethod (weak finalization)? Leaning no — it requires real GC integration which we're deferring.
