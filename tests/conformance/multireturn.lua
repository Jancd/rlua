-- multireturn.lua: test multiple return values in various contexts

-- Multiple returns from function
local function two() return 1, 2 end
local function three() return "a", "b", "c" end

-- In local assignment
local a, b = two()
assert(a == 1 and b == 2, "multi assign")

-- In table constructor (last position expands)
local t = {three()}
assert(t[1] == "a" and t[2] == "b" and t[3] == "c", "multi in table")

-- NOT last position: only first value
local t2 = {two(), "end"}
assert(t2[1] == 1 and t2[2] == "end", "multi not last truncated")

-- Select
assert(select(1, "a", "b", "c") == "a", "select 1")
assert(select(2, "a", "b", "c") == "b", "select 2")
assert(select("#", "a", "b", "c") == 3, "select #")

-- Unpack
local t3 = {10, 20, 30}
local x, y, z = unpack(t3)
assert(x == 10 and y == 20 and z == 30, "unpack")

-- Unpack empty
local nothing = unpack({})
assert(nothing == nil, "unpack empty")

-- Multiple returns passed as args to function
local function sum(a, b, c) return a + b + c end
assert(sum(three() == "a" and 1 or 1, 2, 3) == 6, "multi return as arg")

-- Multiple returns at end of arg list
local function add(a, b) return a + b end
assert(add(two()) == 3, "multi return fills args")

print("multireturn: OK")
