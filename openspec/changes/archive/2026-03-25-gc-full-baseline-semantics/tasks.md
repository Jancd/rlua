## 1. Metatable System (rlua-core + rlua-vm)

- [x] 1.1 Add `metatable: Option<TableRef>` field to `LuaTable` with getter/setter methods
- [x] 1.2 Implement `setmetatable` and `getmetatable` global functions in rlua-stdlib (including `__metatable` protection)
- [x] 1.3 Implement metamethod dispatch helpers: `get_metamethod`, `call_metamethod_binary`, `call_metamethod_unary` in rlua-vm
- [x] 1.4 Add `__index` / `__newindex` dispatch helpers with table-chaining support and 200-level recursion limit
- [x] 1.5 Wire arithmetic opcodes (ADD, SUB, MUL, DIV, MOD, POW, UNM) to fall back to metamethods on failure
- [x] 1.6 Wire comparison opcodes (EQ, LT, LE) to fall back to metamethods; implement `__le` fallback to `not (b < a)`
- [x] 1.7 Wire GETTABLE/SETTABLE to fall back to `__index`/`__newindex`
- [x] 1.8 Wire CONCAT to fall back to `__concat` metamethod
- [x] 1.9 Wire LEN to fall back to `__len` metamethod
- [x] 1.10 Wire CALL to fall back to `__call` metamethod for callable tables
- [x] 1.11 Wire `tostring` to check `__tostring` metamethod before default conversion
- [x] 1.12 Add conformance tests for metatables (arithmetic, comparison, index/newindex, tostring, concat, len, call)

## 2. Math Standard Library (rlua-stdlib)

- [x] 2.1 Create `math.rs` module with math library registration function
- [x] 2.2 Implement math functions: abs, ceil, floor, sqrt, sin, cos, tan, log, exp, fmod, modf, deg, rad
- [x] 2.3 Implement math.max, math.min (vararg)
- [x] 2.4 Implement math.random, math.randomseed (simple PRNG, no external deps)
- [x] 2.5 Register math.pi and math.huge as constant fields on the math table
- [x] 2.6 Add conformance tests for math library

## 3. Table Standard Library (rlua-stdlib)

- [x] 3.1 Create `table_lib.rs` module with table library registration function
- [x] 3.2 Implement table.insert (append and positional)
- [x] 3.3 Implement table.remove (positional and last-element)
- [x] 3.4 Implement table.sort (default and custom comparator via VM callback)
- [x] 3.5 Implement table.concat (with separator, start/end range)
- [x] 3.6 Add conformance tests for table library

## 4. String Standard Library (rlua-stdlib)

- [x] 4.1 Create `string_lib.rs` module with string library registration function
- [x] 4.2 Implement string.byte, string.char, string.len, string.lower, string.upper, string.reverse, string.rep, string.sub
- [x] 4.3 Implement Lua pattern matching engine (character classes, quantifiers, captures, anchors, character sets)
- [x] 4.4 Implement string.find using pattern engine
- [x] 4.5 Implement string.match using pattern engine
- [x] 4.6 Implement string.gmatch returning an iterator closure
- [x] 4.7 Implement string.gsub with string and function replacement, max count
- [x] 4.8 Implement string.format (%d, %s, %f, %g, %x, %%)
- [x] 4.9 Set up shared string metatable with `__index` pointing to string module table
- [x] 4.10 Add conformance tests for string library (basic functions + pattern matching)

## 5. GC Integration (rlua-core + rlua-vm)

- [x] 5.1 Wire `notify_alloc()` calls into table creation (NEWTABLE opcode handler)
- [x] 5.2 Wire `notify_alloc()` into closure creation (CLOSURE opcode handler)
- [x] 5.3 Wire `notify_alloc()` into string allocation (concatenation, string library ops)
- [x] 5.4 Add GC safepoint check at CALL/TAILCALL opcode handlers
- [x] 5.5 Add GC safepoint check at loop back-edges (backward jumps)
- [x] 5.6 Implement transitive marking in `MarkSweepGc::collect` (walk table fields, closure upvalues)
- [x] 5.7 Add tests for GC integration (allocation counting, safepoint triggering, root traversal)

## 6. Error Handling (rlua-vm + rlua-stdlib)

- [x] 6.1 Implement `xpcall` global function with error handler invocation
- [x] 6.2 Support arbitrary error objects (table, number, etc.) through pcall/xpcall
- [x] 6.3 Add source location (file:line) to runtime error messages
- [x] 6.4 Implement `error(msg, level)` level parameter for caller attribution
- [x] 6.5 Add conformance tests for xpcall and error object propagation

## 7. Tail Call Optimization (rlua-vm)

- [x] 7.1 Implement TAILCALL handler that reuses current call frame instead of pushing new
- [x] 7.2 Add conformance test for deep tail recursion (10000+ calls without stack overflow)

## 8. Differential Testing (tests/)

- [x] 8.1 Create `tests/differential/` harness that runs Lua scripts against rlua and reference Lua 5.1
- [x] 8.2 Compare stdout outputs and report diffs on mismatch
- [x] 8.3 Add at least 5 differential test scripts covering arithmetic, strings, tables, metatables, and error handling

## 9. Module Registration and Integration

- [x] 9.1 Create module table registration helper in rlua-stdlib (create table, populate with native fns, set as global)
- [x] 9.2 Wire math, table, and string modules into `register_stdlib` startup
- [x] 9.3 Verify all 4 module tables accessible from Lua (`math`, `table`, `string`, and globals)
- [x] 9.4 Full integration test: compile and run a non-trivial Lua program using metatables + stdlib
