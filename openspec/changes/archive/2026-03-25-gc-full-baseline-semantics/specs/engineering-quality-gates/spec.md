## Capability: engineering-quality-gates (MODIFIED)

Test strategy extended with differential testing layer against reference Lua 5.1.

### Requirement: Differential Test Harness

ADDED: A test harness that runs Lua scripts against both rlua and reference Lua 5.1, comparing their outputs for equivalence.

#### Scenario: Matching output

WHEN a test Lua script is executed by both rlua and reference Lua 5.1
THEN both produce identical stdout output
AND the test passes

#### Scenario: Divergent output

WHEN rlua produces different output than reference Lua 5.1 for a test script
THEN the differential test fails with a clear diff showing expected vs actual

#### Scenario: Harness location

WHEN differential tests are run
THEN they are located in `tests/differential/` and integrated into `cargo test`

### Requirement: Expanded Conformance Test Suite

ADDED: Conformance tests covering all new M2 features — metatables, standard library functions, and edge cases.

#### Scenario: Metatable conformance tests

WHEN `cargo test` is run
THEN tests covering setmetatable/getmetatable, arithmetic metamethods, comparison metamethods, __index/__newindex chaining, __tostring, __concat, __len, __call all pass

#### Scenario: String library conformance tests

WHEN `cargo test` is run
THEN tests covering string.sub, string.find, string.match, string.gmatch, string.gsub, string.format, string.byte, string.char, string.rep, string.reverse, string.lower, string.upper, string.len all pass

#### Scenario: Math library conformance tests

WHEN `cargo test` is run
THEN tests covering math.floor, math.ceil, math.abs, math.sqrt, math.sin, math.cos, math.tan, math.log, math.exp, math.max, math.min, math.random, math.pi, math.huge, math.fmod, math.modf, math.deg, math.rad all pass

#### Scenario: Table library conformance tests

WHEN `cargo test` is run
THEN tests covering table.insert, table.remove, table.sort, table.concat all pass

#### Scenario: Error handling conformance tests

WHEN `cargo test` is run
THEN tests covering xpcall with handler, error object propagation, error with level parameter all pass

#### Scenario: Tail call conformance tests

WHEN `cargo test` is run
THEN tests confirming tail call optimization (deep recursion without stack overflow) pass
