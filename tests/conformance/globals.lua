-- globals.lua: test standard library global functions

-- type
assert(type(nil) == "nil", "type nil")
assert(type(true) == "boolean", "type boolean")
assert(type(42) == "number", "type number")
assert(type("hi") == "string", "type string")
assert(type({}) == "table", "type table")
assert(type(print) == "function", "type function")

-- tostring
assert(tostring(42) == "42", "tostring 42")
assert(tostring(nil) == "nil", "tostring nil")
assert(tostring(true) == "true", "tostring true")
assert(tostring(false) == "false", "tostring false")

-- tonumber
assert(tonumber("123") == 123, "tonumber 123")
assert(tonumber("3.14") == 3.14, "tonumber 3.14")
assert(tonumber("abc") == nil, "tonumber invalid")
assert(tonumber(42) == 42, "tonumber passthrough")

-- assert
assert(true, "assert true passes")
assert(1, "assert 1 passes")
assert("yes", "assert string passes")

-- assert returns its arguments
local a, b = assert(1, "msg")
assert(a == 1, "assert returns value")

-- error
local ok, err = pcall(error, "boom")
assert(ok == false, "error makes pcall fail")
assert(err == "boom", "error message propagated")

-- next
local t = {a = 1, b = 2}
local k, v = next(t)
assert(k ~= nil, "next returns key")
assert(v ~= nil, "next returns value")
local k2, v2 = next(t, k)
assert(k2 ~= nil, "next continues")
local k3 = next(t, k2)
assert(k3 == nil, "next exhausted")

-- next on empty table
assert(next({}) == nil, "next empty")

-- rawget / rawset
local t2 = {}
rawset(t2, "x", 99)
assert(rawget(t2, "x") == 99, "rawget/rawset")
assert(rawget(t2, "missing") == nil, "rawget missing")

-- select
assert(select("#", 1, 2, 3) == 3, "select count")
assert(select(2, "a", "b", "c") == "b", "select index")

-- select with negative index
assert(select(-1, "a", "b", "c") == "c", "select -1")
assert(select(-2, "a", "b", "c") == "b", "select -2")
local p1, p2 = select(-2, 10, 20, 30)
assert(p1 == 20 and p2 == 30, "select -2 returns tail")

-- unpack
local x, y, z = unpack({10, 20, 30})
assert(x == 10 and y == 20 and z == 30, "unpack")

print("globals: OK")
