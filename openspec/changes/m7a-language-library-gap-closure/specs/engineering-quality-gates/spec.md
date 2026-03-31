## Capability: engineering-quality-gates (MODIFIED)

M7A extends the compatibility sweep to cover coroutine semantics and the remaining user-visible library/metamethod behavior gaps that were explicitly left open after M6.

## ADDED Requirements

### Requirement: Coroutine and Library Gap Compatibility Coverage
The project MUST add conformance, differential, and regression coverage for coroutine semantics and the M7A library/metamethod gap closures.

#### Scenario: Conformance suite covers coroutine lifecycle
- **WHEN** the conformance suite is executed
- **THEN** it includes cases for `coroutine.create`, `coroutine.resume`, `coroutine.yield`, `coroutine.status`, `coroutine.running`, and `coroutine.wrap`
- **AND** it verifies main-thread yield errors plus suspended/dead coroutine behavior

#### Scenario: Differential suite covers closed semantic gaps
- **WHEN** differential tests are run against reference Lua 5.1
- **THEN** they include coroutine resume/yield behavior, `table.sort` with Lua comparator functions, and Lua-closure `__tostring` behavior
- **AND** divergent observable results fail the suite

#### Scenario: JIT-enabled execution remains semantically safe
- **WHEN** the newly closed coroutine or library/metamethod paths are exercised with JIT enabled
- **THEN** execution remains interpreter-equivalent for the supported subset
- **AND** unsupported JIT interactions fall back safely without changing Lua-visible behavior
