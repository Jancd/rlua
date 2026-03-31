## Why

After M6, the largest remaining compatibility gaps are no longer in trace tooling or release process, but in user-visible Lua semantics: the runtime still lacks coroutine support, `table.sort` cannot use a Lua comparator function, and `tostring()` does not yet honor Lua-closure `__tostring` metamethods. These are high-signal baseline behavior gaps that are easier to justify and validate now than a broader post-v1 JIT expansion.

## What Changes

- Add correctness-first coroutine support for the Lua 5.1 core coroutine APIs, including creation, resume/yield behavior, running/status inspection, and wrapper-based invocation.
- Close the metatable dispatch gap so `tostring()` can invoke Lua-closure `__tostring` metamethods rather than only working in native-only cases.
- Remove the current `table.sort` limitation by allowing Lua comparator functions instead of rejecting them in native stdlib context.
- Expand conformance, differential, and regression coverage around coroutines and the newly closed library/metamethod behavior gaps.
- Keep JIT scope frozen: coroutine execution and newly closed semantic gaps must remain interpreter-correct first, without promising new native trace coverage in this change.

## Capabilities

### New Capabilities
- `coroutine-support`: coroutine object model, `coroutine` library API, and resume/yield execution semantics for the supported Lua 5.1 subset.

### Modified Capabilities
- `standard-libraries`: extend library behavior so `table.sort` supports Lua comparator functions and aligns with Lua-facing expectations around library-driven control flow.
- `metatable-dispatch`: update `__tostring` dispatch so `tostring()` can call Lua closures as metamethods, not only native functions.
- `engineering-quality-gates`: add conformance, differential, and regression coverage for coroutine semantics and the closed library/metamethod compatibility gaps.

## Impact

- Affected code: `crates/rlua-core`, `crates/rlua-vm`, `crates/rlua-stdlib`, and test assets under `tests/conformance/`, `tests/differential/`, and possibly `tests/jit/` for fallback safety.
- Affected systems: call frame management, protected-call/error propagation around resume/yield boundaries, metatable dispatch helpers, and standard library native function shims.
- Dependencies/APIs: adds a new user-visible `coroutine` library surface and removes two documented behavior limitations, but does not expand the native traced JIT subset or non-x86_64 backend support.
