-- functions.lua: test function declaration, calls, multiple returns, varargs

-- Basic function
local function add(a, b)
    return a + b
end
assert(add(3, 4) == 7, "basic function")

-- Function as expression
local mul = function(a, b) return a * b end
assert(mul(3, 4) == 12, "function expr")

-- Multiple returns
local function multi()
    return 1, 2, 3
end
local a, b, c = multi()
assert(a == 1 and b == 2 and c == 3, "multi return")

-- Extra returns discarded
local x = multi()
assert(x == 1, "extra returns discarded")

-- Missing returns are nil
local function one() return 1 end
local p, q = one()
assert(p == 1 and q == nil, "missing returns nil")

-- No return value
local function noret() end
local r = noret()
assert(r == nil, "no return is nil")

-- Varargs
local function sum(...)
    local total = 0
    local args = {...}
    for i = 1, #args do
        total = total + args[i]
    end
    return total
end
assert(sum(1, 2, 3) == 6, "varargs sum")
assert(sum() == 0, "varargs empty")
assert(sum(10) == 10, "varargs single")

-- select with varargs
local function count(...)
    return select("#", ...)
end
assert(count(1, 2, 3) == 3, "select # varargs")
assert(count() == 0, "select # empty")

-- Passing multiple returns as args
local function double(a, b)
    return a * 2, b * 2
end
local function sum2(a, b)
    return a + b
end
assert(sum2(double(3, 4)) == 14, "multi return as args")

-- Recursive function
local function fact(n)
    if n <= 1 then return 1 end
    return n * fact(n - 1)
end
assert(fact(0) == 1, "fact 0")
assert(fact(1) == 1, "fact 1")
assert(fact(5) == 120, "fact 5")
assert(fact(10) == 3628800, "fact 10")

-- Mutual recursion via globals
function is_even(n)
    if n == 0 then return true end
    return is_odd(n - 1)
end
function is_odd(n)
    if n == 0 then return false end
    return is_even(n - 1)
end
assert(is_even(4) == true, "mutual recursion even")
assert(is_odd(3) == true, "mutual recursion odd")

-- Method call statements preserve self
local obj = { val = 10 }
function obj:get() return self.val end
function obj:set(v) self.val = v end
obj:set(42)
assert(obj:get() == 42, "method call self preserved")

-- Trailing method call expands multiple returns
local collector = {}
function collector:multi() return 1, 2, 3 end
function collector:gather(...) return ... end
local a2, b2, c2 = collector:gather(collector:multi())
assert(a2 == 1 and b2 == 2 and c2 == 3, "trailing method call expands returns")

print("functions: OK")
