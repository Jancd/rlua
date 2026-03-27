## Why

M1 delivered a working Lua 5.1 interpreter (parser, compiler, VM, 17 conformance tests), but it lacks metatables, standard libraries (string/math/table), proper GC, and error handling depth. Without these, user programs that rely on core Lua idioms — OOP via metatables, string manipulation, math operations — cannot run. M2 closes these gaps to produce a stable interpreter with semantic confidence, enabling differential testing against reference Lua.

## What Changes

- Implement full metatable system with metamethod dispatch for all Lua 5.1 operators (__index, __newindex, __add, __sub, __mul, __div, __mod, __pow, __unm, __eq, __lt, __le, __tostring, __concat, __len, __call)
- Add `setmetatable`/`getmetatable` global functions
- Implement string standard library (string.byte, string.char, string.find, string.format, string.len, string.lower, string.upper, string.rep, string.reverse, string.sub, string.gmatch, string.gsub, string.match) including Lua pattern matching engine
- Implement math standard library (math.abs, math.ceil, math.floor, math.sqrt, math.sin, math.cos, math.tan, math.log, math.exp, math.max, math.min, math.random, math.randomseed, math.huge, math.pi, math.fmod, math.modf, math.deg, math.rad)
- Implement table standard library (table.insert, table.remove, table.sort, table.concat)
- Add `xpcall` with custom error handler support
- Deepen GC integration: wire allocation tracking through VM, implement mark-phase object traversal, add GC safepoints at calls and loop back-edges
- Optimize tail calls (TAILCALL reuses call frame instead of pushing new one)
- Add differential test harness comparing rlua output against reference Lua 5.1
- Expand conformance test suite to cover metatables, all stdlib functions, and edge cases

## Capabilities

### New Capabilities
- `metatable-dispatch`: Metatable storage on tables, metamethod resolution, and operator fallback dispatch in the VM for all Lua 5.1 metamethods
- `standard-libraries`: String, math, and table library modules registered as global tables with native function fields; includes Lua pattern matching engine for string library
- `gc-integration`: Mark-sweep GC wired into the VM allocation path with root traversal, sweep reclamation, and configurable collection thresholds
- `error-handling-extended`: xpcall with error handler, error object propagation, improved error messages with source locations

### Modified Capabilities
- `lua-runtime-foundation`: GC requirement upgraded from stub to integrated mark-sweep with actual object tracking; metatable field added to table value model; tail call optimization implemented
- `engineering-quality-gates`: Test strategy extended with differential testing layer against reference Lua 5.1

## Impact

- **rlua-core**: LuaTable gains metatable field; GC module upgraded from stub to functional collector; value model may need GC-aware wrappers
- **rlua-vm**: Every arithmetic, comparison, table access, and concatenation opcode handler gains metamethod fallback paths; TAILCALL optimized; GC safepoints inserted
- **rlua-stdlib**: Expands from 15 global functions to ~50+ functions across 4 modules (globals, string, math, table); module table registration pattern needed
- **rlua-compiler**: Minimal changes — may need adjustments for tail call detection
- **rlua-cli**: No changes expected
- **tests/**: New conformance tests for metatables, string/math/table libs, xpcall; new differential test harness in tests/differential/
