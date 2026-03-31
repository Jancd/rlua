## Capability: metatable-dispatch (MODIFIED)

M7A removes the remaining `__tostring` dispatch gap by requiring `tostring()` to honor Lua-closure metamethods, not only native callbacks.

## MODIFIED Requirements

### Requirement: String and Miscellaneous Metamethods
The runtime MUST support additional metamethods for string operations, length, callable tables, and string conversion.

Metamethods: `__tostring`, `__concat`, `__len`, `__call`.

#### Scenario: __tostring via native or Lua closure
- **WHEN** `tostring(v)` is called and `v` has a `__tostring` metamethod that may be native or Lua
- **THEN** the runtime invokes that metamethod through ordinary Lua call semantics
- **AND** the metamethod result is used as the string conversion result

#### Scenario: __tostring invalid result errors
- **WHEN** a `__tostring` metamethod returns a non-string-like result that is invalid for `tostring()`
- **THEN** the runtime raises a Lua-facing error instead of silently formatting the original value

#### Scenario: __concat
- **WHEN** `a .. b` is executed and concatenation fails due to operand types
- **THEN** the runtime looks up `__concat` on the left operand and then the right operand
- **AND** it invokes the discovered metamethod when present

#### Scenario: __len
- **WHEN** `#t` is executed on a table with `__len` metamethod
- **THEN** the runtime calls `__len(t)` and returns the result instead of the raw length

#### Scenario: __call
- **WHEN** `t(args...)` is executed and `t` is a table with `__call` metamethod
- **THEN** the runtime invokes `__call(t, args...)`
- **AND** the table behaves as a callable Lua value for that operation
