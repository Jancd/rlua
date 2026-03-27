## Capability: error-handling-extended

xpcall with error handler, error object propagation, improved error messages with source locations.

### Requirement: xpcall with Custom Error Handler

`xpcall(f, handler, ...)` calls `f` in protected mode. If `f` raises an error, `handler` is called with the error object before the error propagates.

#### Scenario: Successful xpcall

WHEN `xpcall(f, handler)` is called and `f` succeeds
THEN it returns `true` followed by `f`'s return values
AND `handler` is never called

#### Scenario: xpcall with error

WHEN `xpcall(f, handler)` is called and `f` raises an error `e`
THEN `handler(e)` is called
AND `xpcall` returns `false` followed by `handler`'s return value

#### Scenario: xpcall passes arguments to f

WHEN `xpcall(f, handler, a, b, c)` is called
THEN `f(a, b, c)` is invoked with the extra arguments

#### Scenario: Error in handler itself

WHEN `xpcall(f, handler)` is called and `f` errors and `handler` also errors
THEN `xpcall` returns `false` and an error message about the handler failure

### Requirement: Error Object Propagation

Errors are not limited to strings — any Lua value can be used as an error object. The error object is passed through pcall/xpcall intact.

#### Scenario: Table as error object

WHEN `error({code = 404, msg = "not found"})` is called inside `pcall`
THEN the second return value of `pcall` is the table `{code = 404, msg = "not found"}`

#### Scenario: Number as error object

WHEN `error(42)` is called inside `pcall`
THEN the second return value of `pcall` is the number `42`

### Requirement: Improved Error Messages

Runtime errors include source location information (file name and line number) when available.

#### Scenario: Error with source location

WHEN a runtime error occurs (e.g., arithmetic on nil)
THEN the error message includes the source file and line number (e.g., `"test.lua:5: attempt to perform arithmetic on a nil value"`)

#### Scenario: error() with level parameter

WHEN `error(msg, level)` is called with `level = 2`
THEN the error message references the caller's location, not the location of the `error()` call itself

WHEN `error(msg, 0)` is called
THEN no location information is prepended to the error message
