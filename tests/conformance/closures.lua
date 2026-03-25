-- closures.lua: test upvalue capture, shared upvalues

-- Basic closure
local function make_adder(x)
    return function(y) return x + y end
end
local add5 = make_adder(5)
local add10 = make_adder(10)
assert(add5(3) == 8, "closure add5")
assert(add10(3) == 13, "closure add10")

-- Counter closure (shared mutable upvalue)
local function make_counter()
    local n = 0
    return function()
        n = n + 1
        return n
    end
end
local c = make_counter()
assert(c() == 1, "counter 1")
assert(c() == 2, "counter 2")
assert(c() == 3, "counter 3")

-- Two closures sharing one upvalue
local function make_pair()
    local val = 0
    local function get() return val end
    local function set(v) val = v end
    return get, set
end
local get, set = make_pair()
assert(get() == 0, "shared upval initial")
set(42)
assert(get() == 42, "shared upval after set")

-- Closure in loop
local funcs = {}
for i = 1, 5 do
    funcs[i] = function() return i end
end
-- With proper upvalue semantics, each closure captures its own i
assert(funcs[1]() == 1, "loop closure 1")
assert(funcs[3]() == 3, "loop closure 3")
assert(funcs[5]() == 5, "loop closure 5")

-- Nested closures
local function outer()
    local x = 10
    local function middle()
        local y = 20
        local function inner()
            return x + y
        end
        return inner
    end
    return middle
end
assert(outer()()() == 30, "nested closures")

-- Self-recursive local function
local function fib(n)
    if n <= 1 then return n end
    return fib(n - 1) + fib(n - 2)
end
assert(fib(0) == 0, "fib 0")
assert(fib(1) == 1, "fib 1")
assert(fib(10) == 55, "fib 10")

print("closures: OK")
