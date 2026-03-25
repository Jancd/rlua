-- recursion.lua: test recursive algorithms

-- Factorial
local function fact(n)
    if n <= 1 then return 1 end
    return n * fact(n - 1)
end
assert(fact(0) == 1, "fact 0")
assert(fact(1) == 1, "fact 1")
assert(fact(10) == 3628800, "fact 10")
assert(fact(12) == 479001600, "fact 12")

-- Fibonacci
local function fib(n)
    if n <= 1 then return n end
    return fib(n - 1) + fib(n - 2)
end
assert(fib(0) == 0, "fib 0")
assert(fib(1) == 1, "fib 1")
assert(fib(10) == 55, "fib 10")
assert(fib(15) == 610, "fib 15")

-- GCD (Euclidean algorithm)
local function gcd(a, b)
    if b == 0 then return a end
    return gcd(b, a % b)
end
assert(gcd(12, 8) == 4, "gcd 12,8")
assert(gcd(100, 75) == 25, "gcd 100,75")
assert(gcd(7, 13) == 1, "gcd 7,13")

-- Power (recursive)
local function power(base, exp)
    if exp == 0 then return 1 end
    return base * power(base, exp - 1)
end
assert(power(2, 10) == 1024, "power 2^10")
assert(power(3, 4) == 81, "power 3^4")

-- Ackermann function (deeply recursive)
local function ack(m, n)
    if m == 0 then return n + 1 end
    if n == 0 then return ack(m - 1, 1) end
    return ack(m - 1, ack(m, n - 1))
end
assert(ack(0, 0) == 1, "ack 0,0")
assert(ack(1, 1) == 3, "ack 1,1")
assert(ack(2, 2) == 7, "ack 2,2")
assert(ack(3, 3) == 61, "ack 3,3")

print("recursion: OK")
