## Capability: standard-libraries (MODIFIED)

M7A closes the remaining user-visible table-library gap by letting `table.sort` use Lua comparator functions rather than limiting comparator support to native-only callbacks.

## MODIFIED Requirements

### Requirement: Table Standard Library
The runtime MUST implement the Lua 5.1 table manipulation functions.

Functions: `table.insert`, `table.remove`, `table.sort`, `table.concat`.

#### Scenario: table.insert append
- **WHEN** `table.insert(t, v)` is called with two arguments
- **THEN** `v` is appended to the end of table `t`

#### Scenario: table.insert at position
- **WHEN** `table.insert(t, pos, v)` is called with three arguments
- **THEN** `v` is inserted at position `pos`, shifting subsequent elements up

#### Scenario: table.remove
- **WHEN** `table.remove(t, pos)` is called
- **THEN** the element at `pos` is removed, subsequent elements shift down, and the removed value is returned
- **AND** `table.remove(t)` with one argument removes and returns the last element

#### Scenario: table.sort without comparator
- **WHEN** `table.sort(t)` is called without a comparator
- **THEN** `t` is sorted in ascending order using Lua-visible `<` comparison semantics

#### Scenario: table.sort with Lua or native comparator
- **WHEN** `table.sort(t, comp)` is called with a comparator function that may be native or Lua
- **THEN** `t` is sorted using repeated calls to `comp(a, b)`
- **AND** the comparator's truthiness result determines whether `a` should come before `b`

#### Scenario: Comparator error propagates
- **WHEN** the comparator passed to `table.sort` raises an error
- **THEN** `table.sort` propagates that Lua-facing error instead of silently discarding it

#### Scenario: table.concat
- **WHEN** `table.concat(t, sep)` is called
- **THEN** all string/number elements of `t` are concatenated with `sep` as separator
- **AND** `table.concat(t, sep, i, j)` concatenates elements from index `i` to `j`
