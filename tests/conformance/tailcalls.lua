-- tailcalls.lua: test tail call optimization

-- Basic tail call (should not overflow stack for deep recursion)
local function countdown(n)
    if n <= 0 then return "done" end
    return countdown(n - 1)
end
assert(countdown(10000) == "done", "deep tail recursion")

-- Mutual tail recursion
local function is_even(n)
    if n == 0 then return true end
    return is_odd(n - 1)
end

function is_odd(n)
    if n == 0 then return false end
    return is_even(n - 1)
end

assert(is_even(0) == true, "even 0")
assert(is_even(4) == true, "even 4")
assert(is_odd(3) == true, "odd 3")
assert(is_odd(4) == false, "odd 4")

-- Tail call with multiple arguments
local function sum_tail(n, acc)
    if n <= 0 then return acc end
    return sum_tail(n - 1, acc + n)
end
assert(sum_tail(100, 0) == 5050, "tail call sum 100")

-- Non-tail call (result used in expression) — should still work, just uses stack
local function factorial(n)
    if n <= 1 then return 1 end
    return n * factorial(n - 1)
end
assert(factorial(10) == 3628800, "factorial 10")

-- Tail call in different positions
local function f1(x)
    if x > 0 then
        return f1(x - 1)  -- tail position
    end
    return x
end
assert(f1(100) == 0, "tail call if branch")

local function f2(x)
    if x > 0 then
        local y = x - 1
        return f2(y)  -- tail position after local
    end
    return "end"
end
assert(f2(50) == "end", "tail call after local")

-- Fibonacci with tail call (accumulator pattern)
local function fib_tail(n, a, b)
    if n == 0 then return a end
    if n == 1 then return b end
    return fib_tail(n - 1, b, a + b)
end
assert(fib_tail(10, 0, 1) == 55, "fib tail 10")
assert(fib_tail(20, 0, 1) == 6765, "fib tail 20")

-- Deep tail recursion to verify optimization (would stack overflow without TCO)
local function deep(n)
    if n <= 0 then return 0 end
    return deep(n - 1)
end
assert(deep(50000) == 0, "very deep tail recursion")

print("tailcalls: OK")
