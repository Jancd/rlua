## Capability: standard-libraries

String, math, and table library modules registered as global tables with native function fields; includes Lua pattern matching engine for string library.

### Requirement: Module Registration Pattern

Each standard library is registered as a global table with native function values. Libraries are populated at VM startup and accessible as `math.floor`, `string.sub`, etc.

#### Scenario: Module table access

WHEN a Lua program references `math.floor`
THEN it resolves to the native function via `globals["math"]["floor"]`

#### Scenario: String method syntax

WHEN a Lua program calls `s:sub(1, 3)` on a string value
THEN the string metatable's `__index` points to the `string` module table
AND `string.sub(s, 1, 3)` is invoked with `s` as the first argument

### Requirement: Math Standard Library

Implements all standard Lua 5.1 math functions and constants.

Functions: `math.abs`, `math.ceil`, `math.floor`, `math.sqrt`, `math.sin`, `math.cos`, `math.tan`, `math.log`, `math.exp`, `math.max`, `math.min`, `math.random`, `math.randomseed`, `math.huge`, `math.pi`, `math.fmod`, `math.modf`, `math.deg`, `math.rad`.

#### Scenario: Basic math functions

WHEN `math.floor(3.7)` is called
THEN it returns `3`

WHEN `math.ceil(3.2)` is called
THEN it returns `4`

WHEN `math.abs(-5)` is called
THEN it returns `5`

#### Scenario: Trigonometric functions

WHEN `math.sin(0)` is called
THEN it returns `0`

WHEN `math.cos(0)` is called
THEN it returns `1`

#### Scenario: Math constants

WHEN `math.pi` is accessed
THEN it returns the value of pi (3.14159265358979...)

WHEN `math.huge` is accessed
THEN it returns positive infinity

#### Scenario: min/max with multiple arguments

WHEN `math.max(1, 5, 3)` is called
THEN it returns `5`

WHEN `math.min(1, 5, 3)` is called
THEN it returns `1`

#### Scenario: math.random

WHEN `math.random()` is called with no arguments
THEN it returns a float in [0, 1)

WHEN `math.random(n)` is called with one integer argument
THEN it returns an integer in [1, n]

WHEN `math.random(m, n)` is called
THEN it returns an integer in [m, n]

#### Scenario: math.modf

WHEN `math.modf(3.75)` is called
THEN it returns two values: `3` (integer part) and `0.75` (fractional part)

### Requirement: Table Standard Library

Implements Lua 5.1 table manipulation functions.

Functions: `table.insert`, `table.remove`, `table.sort`, `table.concat`.

#### Scenario: table.insert append

WHEN `table.insert(t, v)` is called with two arguments
THEN `v` is appended to the end of table `t`

#### Scenario: table.insert at position

WHEN `table.insert(t, pos, v)` is called with three arguments
THEN `v` is inserted at position `pos`, shifting subsequent elements up

#### Scenario: table.remove

WHEN `table.remove(t, pos)` is called
THEN the element at `pos` is removed, subsequent elements shift down, and the removed value is returned

WHEN `table.remove(t)` is called with one argument
THEN the last element is removed and returned

#### Scenario: table.sort

WHEN `table.sort(t)` is called without a comparator
THEN `t` is sorted in ascending order using `<`

WHEN `table.sort(t, comp)` is called with a comparator function
THEN `t` is sorted using `comp(a, b)` which returns true if `a` should come before `b`

#### Scenario: table.concat

WHEN `table.concat(t, sep)` is called
THEN all string/number elements of `t` are concatenated with `sep` as separator

WHEN `table.concat(t, sep, i, j)` is called
THEN elements from index `i` to `j` are concatenated

### Requirement: String Standard Library

Implements Lua 5.1 string manipulation functions.

Functions: `string.byte`, `string.char`, `string.find`, `string.format`, `string.len`, `string.lower`, `string.upper`, `string.rep`, `string.reverse`, `string.sub`, `string.gmatch`, `string.gsub`, `string.match`.

#### Scenario: string.sub

WHEN `string.sub("hello", 2, 4)` is called
THEN it returns `"ell"`

WHEN `string.sub("hello", -3)` is called
THEN it returns `"llo"` (negative indices count from end)

#### Scenario: string.find

WHEN `string.find("hello world", "world")` is called
THEN it returns `7, 11` (start and end positions, 1-based)

WHEN `string.find("hello", "xyz")` is called
THEN it returns `nil`

#### Scenario: string.format basic specifiers

WHEN `string.format("%d %s", 42, "hi")` is called
THEN it returns `"42 hi"`

Supported specifiers: `%d`, `%s`, `%f`, `%g`, `%x`, `%%`.

#### Scenario: string.byte and string.char

WHEN `string.byte("A")` is called
THEN it returns `65`

WHEN `string.char(65)` is called
THEN it returns `"A"`

#### Scenario: string.rep

WHEN `string.rep("ab", 3)` is called
THEN it returns `"ababab"`

#### Scenario: string.lower and string.upper

WHEN `string.lower("Hello")` is called
THEN it returns `"hello"`

WHEN `string.upper("Hello")` is called
THEN it returns `"HELLO"`

### Requirement: Lua Pattern Matching Engine

A self-contained matching engine for Lua patterns (NOT regular expressions). Supports character classes, captures, anchors, and quantifiers.

#### Scenario: Character classes

WHEN a pattern uses `%a` (letters), `%d` (digits), `%w` (alphanumeric), `%s` (whitespace), `%p` (punctuation)
THEN matching correctly classifies characters
AND uppercase versions (`%A`, `%D`, etc.) match the complement

#### Scenario: Quantifiers

WHEN a pattern uses `*` (0 or more, greedy), `+` (1 or more, greedy), `?` (0 or 1), `-` (0 or more, lazy)
THEN matching applies the correct quantifier semantics

#### Scenario: Captures

WHEN `string.match("2023-01-15", "(%d+)-(%d+)-(%d+)")` is called
THEN it returns `"2023"`, `"01"`, `"15"` as separate capture values

#### Scenario: Anchors

WHEN a pattern starts with `^` or ends with `$`
THEN matching is anchored to the start or end of the string respectively

#### Scenario: string.gmatch iteration

WHEN `for w in string.gmatch("hello world foo", "%a+") do` is used
THEN the iterator yields `"hello"`, `"world"`, `"foo"` in sequence

#### Scenario: string.gsub replacement

WHEN `string.gsub("hello world", "(%w+)", "%1-%1")` is called
THEN it returns `"hello-hello world-world"` with capture substitution

WHEN `string.gsub("hello", "l", "L", 1)` is called
THEN it returns `"heLlo"` (max 1 replacement)

#### Scenario: Character sets

WHEN a pattern uses `[abc]` or `[a-z]` or `[^0-9]`
THEN matching correctly handles character set inclusion/exclusion/ranges
