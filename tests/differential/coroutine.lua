-- Differential test: coroutine library
print(coroutine.running() == nil)

local co
co = coroutine.create(function(a, b)
    local running = coroutine.running()
    print(type(running), running == co, coroutine.status(running))
    local x, y = coroutine.yield(a + b, "yielded")
    print(x, y, coroutine.status(running))
    return "done", a * b
end)

print(type(co), coroutine.status(co))
print(coroutine.resume(co, 2, 5))
print(coroutine.status(co))
print(coroutine.resume(co, "resume-token", "again"))
print(coroutine.status(co))

local dead_ok, dead_err = coroutine.resume(co)
print(dead_ok, string.find(dead_err, "dead coroutine") ~= nil)

local yield_ok, yield_err = pcall(coroutine.yield, "boom")
print(yield_ok, type(yield_err))

local parent
local child = coroutine.create(function()
    print(coroutine.status(parent))
    coroutine.yield("child-yield")
    return "child-done"
end)

parent = coroutine.create(function()
    print(coroutine.resume(child))
    print(coroutine.status(child))
end)

print(coroutine.resume(parent))
print(coroutine.status(parent), coroutine.status(child))
print(coroutine.resume(child))
print(coroutine.status(child))

local wrapped = coroutine.wrap(function(x)
    local resumed = coroutine.yield(x + 1, "wrapped-yield")
    return resumed * 2, "wrapped-done"
end)

print(wrapped(4))
print(wrapped(6))

local wrapped_err = coroutine.wrap(function()
    error("wrap boom")
end)
local wrap_ok, wrap_err = pcall(wrapped_err)
print(wrap_ok, string.find(wrap_err, "wrap boom") ~= nil)
