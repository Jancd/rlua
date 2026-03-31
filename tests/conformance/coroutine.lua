-- coroutine.lua: test coroutine library semantics

assert(coroutine.running() == nil, "main thread is not a coroutine")

local co
co = coroutine.create(function(a, b)
    local running = coroutine.running()
    assert(type(running) == "thread", "running coroutine is a thread")
    assert(running == co, "running coroutine identity")
    assert(coroutine.status(running) == "running", "self status is running")

    local token, final = coroutine.yield(a + b, "yielded")
    assert(token == "resume-token", "resume args reach suspended coroutine")
    return final, a * b
end)

assert(type(co) == "thread", "create returns thread")
assert(coroutine.status(co) == "suspended", "new coroutine is suspended")

local ok, sum, tag = coroutine.resume(co, 2, 5)
assert(ok == true, "first resume succeeds")
assert(sum == 7 and tag == "yielded", "yielded values returned from resume")
assert(coroutine.status(co) == "suspended", "yielded coroutine stays suspended")

ok, final, product = coroutine.resume(co, "resume-token", "done")
assert(ok == true, "second resume succeeds")
assert(final == "done" and product == 10, "returned values propagated from coroutine")
assert(coroutine.status(co) == "dead", "completed coroutine is dead")

ok, local_err = coroutine.resume(co)
assert(ok == false, "dead coroutine cannot resume")
assert(string.find(local_err, "dead coroutine"), "dead coroutine error message")

local yield_ok, yield_err = pcall(coroutine.yield, "boom")
assert(yield_ok == false, "main-thread yield errors")
assert(string.find(yield_err, "outside a coroutine"), "main-thread yield error message")

local parent
local child = coroutine.create(function()
    assert(coroutine.status(parent) == "normal", "resuming parent becomes normal")
    coroutine.yield("child-yield")
    return "child-done"
end)

parent = coroutine.create(function()
    local child_ok, child_value = coroutine.resume(child)
    assert(child_ok == true and child_value == "child-yield", "nested resume succeeds")
    assert(coroutine.status(child) == "suspended", "child is suspended after yield")
end)

ok = coroutine.resume(parent)
assert(ok == true, "parent coroutine resumes")
assert(coroutine.status(parent) == "dead", "parent completes after nested resume")
assert(coroutine.status(child) == "suspended", "child remains suspended")

ok, local_value = coroutine.resume(child)
assert(ok == true and local_value == "child-done", "child resumes after parent returns")
assert(coroutine.status(child) == "dead", "child completes")

local wrapped = coroutine.wrap(function(x)
    local resumed = coroutine.yield(x + 1, "wrapped-yield")
    return resumed * 2, "wrapped-done"
end)

local first, first_tag = wrapped(4)
assert(first == 5 and first_tag == "wrapped-yield", "wrap returns yielded values directly")

local second, second_tag = wrapped(6)
assert(second == 12 and second_tag == "wrapped-done", "wrap returns final values directly")

local wrapped_err = coroutine.wrap(function()
    error("wrap boom")
end)
local wrap_ok, wrap_err = pcall(wrapped_err)
assert(wrap_ok == false, "wrap rethrows coroutine errors")
assert(string.find(wrap_err, "wrap boom"), "wrap error message preserved")

print("coroutine: OK")
