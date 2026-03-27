## Capability: metatable-dispatch

Metatable storage on tables, metamethod resolution, and operator fallback dispatch in the VM for all Lua 5.1 metamethods.

### Requirement: Metatable Storage on Tables

Every `LuaTable` carries an optional metatable reference (`Option<TableRef>`). The metatable is itself a regular Lua table whose string-keyed entries (e.g., `"__add"`) define metamethods.

#### Scenario: Set and retrieve a metatable

WHEN `setmetatable(t, mt)` is called with a table `t` and a table `mt`
THEN `t`'s metatable is set to `mt` and `setmetatable` returns `t`
AND `getmetatable(t)` returns `mt`

#### Scenario: Clear a metatable

WHEN `setmetatable(t, nil)` is called
THEN `t`'s metatable is removed
AND `getmetatable(t)` returns `nil`

#### Scenario: Metatable on non-table value

WHEN `setmetatable(v, mt)` is called where `v` is not a table
THEN a runtime error is raised

#### Scenario: __metatable protection

WHEN a table has a metatable with a `__metatable` field
THEN `getmetatable(t)` returns the `__metatable` field value instead of the actual metatable
AND `setmetatable(t, mt)` raises an error ("cannot change a protected metatable")

### Requirement: Arithmetic Metamethod Dispatch

When an arithmetic operation fails (operand is not a number and cannot be coerced), the VM looks up the corresponding metamethod on the left operand's metatable, then the right operand's metatable.

Metamethods: `__add`, `__sub`, `__mul`, `__div`, `__mod`, `__pow`, `__unm` (unary minus).

#### Scenario: Binary arithmetic with metamethod

WHEN `a + b` is executed and `a` is a table with `__add` metamethod
THEN the VM calls `__add(a, b)` and uses the return value as the result

#### Scenario: Binary arithmetic fallback to right operand

WHEN `a + b` is executed and `a` has no `__add` but `b` is a table with `__add`
THEN the VM calls `__add(a, b)` from `b`'s metatable

#### Scenario: No metamethod found

WHEN an arithmetic operation fails and neither operand has the corresponding metamethod
THEN a runtime error is raised ("attempt to perform arithmetic on a <type> value")

#### Scenario: Unary minus metamethod

WHEN `-a` is executed and `a` is a table with `__unm` metamethod
THEN the VM calls `__unm(a)` and uses the return value

### Requirement: Comparison Metamethod Dispatch

Comparison operators fall back to metamethods when operands are not directly comparable.

Metamethods: `__eq`, `__lt`, `__le`.

#### Scenario: Equality with __eq

WHEN `a == b` is executed where both are tables with the same `__eq` metamethod
THEN the VM calls `__eq(a, b)` and uses the boolean result

#### Scenario: Less-than with __lt

WHEN `a < b` is executed where both are tables with the same `__lt` metamethod
THEN the VM calls `__lt(a, b)` and uses the boolean result

#### Scenario: Less-or-equal fallback

WHEN `a <= b` is executed and `__le` is not defined but `__lt` is
THEN the VM evaluates `not (b < a)` as a fallback (Lua 5.1 behavior)

### Requirement: Table Access Metamethods

`__index` and `__newindex` control field lookup and assignment for missing keys.

#### Scenario: __index as function

WHEN `t[k]` is accessed and `k` is not present in `t` and `t` has `__index` as a function
THEN the VM calls `__index(t, k)` and returns the result

#### Scenario: __index as table (chaining)

WHEN `t[k]` is accessed and `k` is not present in `t` and `t` has `__index` as a table `mt`
THEN the VM looks up `k` in `mt` (recursively, following `mt`'s own `__index`)

#### Scenario: __newindex as function

WHEN `t[k] = v` is executed and `k` is not present in `t` and `t` has `__newindex` as a function
THEN the VM calls `__newindex(t, k, v)` instead of performing the raw set

#### Scenario: __newindex as table

WHEN `t[k] = v` is executed and `k` is not present in `t` and `t` has `__newindex` as a table `mt`
THEN the VM performs the set on `mt` instead (following `mt`'s own `__newindex` if `k` absent there too)

#### Scenario: Metamethod recursion depth limit

WHEN `__index` chains recurse beyond 200 levels
THEN a runtime error is raised ("'__index' chain too long; possible loop")

### Requirement: String and Miscellaneous Metamethods

Additional metamethods for string operations, length, and callable tables.

Metamethods: `__tostring`, `__concat`, `__len`, `__call`.

#### Scenario: __tostring

WHEN `tostring(t)` is called and `t` has a `__tostring` metamethod
THEN the VM calls `__tostring(t)` and returns the string result

#### Scenario: __concat

WHEN `a .. b` is executed and concatenation fails (non-string/number operand)
THEN the VM looks up `__concat` on the left operand, then right operand, and calls it

#### Scenario: __len

WHEN `#t` is executed on a table with `__len` metamethod
THEN the VM calls `__len(t)` and returns the result instead of the raw length

#### Scenario: __call

WHEN `t(args...)` is executed and `t` is a table with `__call` metamethod
THEN the VM calls `__call(t, args...)` treating the table as callable

### Requirement: Metamethod Dispatch Helpers

Metamethod dispatch is extracted into reusable helper functions to avoid bloating individual opcode handlers.

#### Scenario: Helper function organization

WHEN the VM encounters an operation that may need metamethod fallback
THEN it calls a helper function (e.g., `call_metamethod_binary`, `index_with_metamethod`) that handles the full resolution chain
AND individual opcode handlers remain concise (thin dispatch + fallback call)
