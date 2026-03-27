-- math_lib.lua: test math standard library

-- Constants
assert(math.pi > 3.14 and math.pi < 3.15, "math.pi")
assert(math.huge > 1e300, "math.huge")

-- abs
assert(math.abs(-5) == 5, "abs negative")
assert(math.abs(5) == 5, "abs positive")
assert(math.abs(0) == 0, "abs zero")

-- ceil / floor
assert(math.ceil(2.3) == 3, "ceil up")
assert(math.ceil(-2.3) == -2, "ceil negative")
assert(math.ceil(5) == 5, "ceil integer")

assert(math.floor(2.7) == 2, "floor down")
assert(math.floor(-2.7) == -3, "floor negative")
assert(math.floor(5) == 5, "floor integer")

-- sqrt
assert(math.sqrt(9) == 3, "sqrt 9")
assert(math.sqrt(0) == 0, "sqrt 0")
assert(math.sqrt(2) > 1.414 and math.sqrt(2) < 1.415, "sqrt 2")

-- Trig functions
assert(math.sin(0) == 0, "sin 0")
assert(math.cos(0) == 1, "cos 0")
assert(math.tan(0) == 0, "tan 0")

-- sin(pi/2) should be ~1
local sp = math.sin(math.pi / 2)
assert(sp > 0.999 and sp < 1.001, "sin pi/2")

-- cos(pi) should be ~-1
local cp = math.cos(math.pi)
assert(cp > -1.001 and cp < -0.999, "cos pi")

-- log (natural log)
assert(math.log(1) == 0, "log 1")
local l2 = math.log(math.exp(1))
assert(l2 > 0.999 and l2 < 1.001, "log(e) = 1")

-- exp
assert(math.exp(0) == 1, "exp 0")
local e1 = math.exp(1)
assert(e1 > 2.718 and e1 < 2.719, "exp 1")

-- fmod
assert(math.fmod(7, 3) == 1, "fmod 7,3")
assert(math.fmod(10, 5) == 0, "fmod 10,5")
assert(math.fmod(-7, 3) == -1, "fmod negative")

-- modf
local i, f = math.modf(3.75)
assert(i == 3, "modf integer part")
assert(f > 0.749 and f < 0.751, "modf fractional part")

local i2, f2 = math.modf(-3.75)
assert(i2 == -3, "modf negative integer")
assert(f2 > -0.751 and f2 < -0.749, "modf negative frac")

-- deg / rad
assert(math.deg(math.pi) == 180, "deg pi")
local r = math.rad(180)
assert(r > 3.141 and r < 3.142, "rad 180")

-- max / min
assert(math.max(1, 2, 3) == 3, "max")
assert(math.max(-1, -5, -2) == -1, "max negative")
assert(math.max(42) == 42, "max single")

assert(math.min(1, 2, 3) == 1, "min")
assert(math.min(-1, -5, -2) == -5, "min negative")
assert(math.min(42) == 42, "min single")

-- random / randomseed
math.randomseed(42)
local r1 = math.random()
assert(r1 >= 0 and r1 < 1, "random [0,1)")

local r2 = math.random(10)
assert(r2 >= 1 and r2 <= 10, "random 1..10")

local r3 = math.random(5, 10)
assert(r3 >= 5 and r3 <= 10, "random 5..10")

-- Determinism: same seed -> same sequence
math.randomseed(123)
local seq1 = math.random(1000)
math.randomseed(123)
local seq2 = math.random(1000)
assert(seq1 == seq2, "randomseed determinism")

print("math_lib: OK")
