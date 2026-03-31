## Capability: coroutine-support

First-class Lua `thread` values and the Lua 5.1 `coroutine` library for resumable interpreter execution without widening native JIT scope.

### Requirement: Coroutine Library Surface
The runtime MUST provide a global `coroutine` table and first-class `thread` values for the supported Lua 5.1 coroutine APIs.

#### Scenario: Create returns a thread value
- **WHEN** `coroutine.create(f)` is called with a Lua function `f`
- **THEN** it returns a distinct Lua value whose `type()` is `thread`
- **AND** that value can later be passed to `coroutine.resume`, `coroutine.status`, and related coroutine APIs

#### Scenario: Running reports the active coroutine
- **WHEN** `coroutine.running()` is called inside a resumed coroutine
- **THEN** it returns the currently running thread value for that coroutine
- **AND** code executing inside that coroutine can distinguish it from other thread values by identity

### Requirement: Resume and Yield Semantics
The runtime MUST support Lua-visible coroutine suspension and resumption semantics for interpreter execution.

#### Scenario: First resume starts the coroutine
- **WHEN** `coroutine.resume(co, ...)` is called on a newly created coroutine
- **THEN** the coroutine function starts executing with those arguments
- **AND** `coroutine.resume` returns `true` followed by the yielded or returned values produced by that execution slice

#### Scenario: Yield suspends and later resume continues
- **WHEN** a running coroutine calls `coroutine.yield(...)`
- **THEN** the coroutine suspends without becoming dead
- **AND** a later `coroutine.resume(co, ...)` continues execution from the suspension point rather than restarting the function body

#### Scenario: Yield from the main thread fails
- **WHEN** `coroutine.yield(...)` is called outside a running coroutine
- **THEN** the runtime raises a Lua-facing error
- **AND** the root interpreter execution does not suspend

### Requirement: Coroutine Status, Error, and Wrap Behavior
The coroutine library MUST expose Lua 5.1-compatible status inspection, error propagation, and wrapper-based invocation behavior.

#### Scenario: Status reflects lifecycle
- **WHEN** `coroutine.status(co)` is queried for a newly created, running, suspended, or completed coroutine
- **THEN** it reports the corresponding Lua-visible lifecycle state for that thread

#### Scenario: Resume returns false on coroutine error
- **WHEN** a coroutine errors during `coroutine.resume(co, ...)`
- **THEN** `coroutine.resume` returns `false` followed by the error object or message
- **AND** the coroutine does not continue running past the error point

#### Scenario: Wrap resumes and propagates values directly
- **WHEN** `coroutine.wrap(f)` is called and the resulting wrapper function is invoked
- **THEN** the wrapper resumes the underlying coroutine and returns yielded or final values directly
- **AND** coroutine errors are surfaced as ordinary Lua errors from the wrapper call
