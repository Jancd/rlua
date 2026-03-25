-- tables.lua: test table construction, indexing, field access, length, nesting

-- Array construction
local t = {1, 2, 3}
assert(t[1] == 1, "array index 1")
assert(t[2] == 2, "array index 2")
assert(t[3] == 3, "array index 3")
assert(#t == 3, "array length")

-- Named field construction
local t2 = {x = 10, y = 20}
assert(t2.x == 10, "named field x")
assert(t2.y == 20, "named field y")
assert(t2["x"] == 10, "bracket access named")

-- Mixed construction
local t3 = {1, 2, x = "hello", 3}
assert(t3[1] == 1, "mixed array 1")
assert(t3[2] == 2, "mixed array 2")
assert(t3[3] == 3, "mixed array 3")
assert(t3.x == "hello", "mixed named")

-- Assignment
local t4 = {}
t4[1] = "a"
t4[2] = "b"
t4.name = "test"
assert(t4[1] == "a", "assign array")
assert(t4.name == "test", "assign named")

-- Length of table
assert(#{1, 2, 3, 4, 5} == 5, "length 5")
assert(#{} == 0, "length 0")

-- Nested tables
local nested = {inner = {val = 42}}
assert(nested.inner.val == 42, "nested access")

-- Table as value
local t5 = {}
t5.sub = {1, 2, 3}
assert(t5.sub[2] == 2, "table value access")

-- Explicit index construction
local t6 = {[1] = "one", [2] = "two", ["key"] = "val"}
assert(t6[1] == "one", "explicit index 1")
assert(t6[2] == "two", "explicit index 2")
assert(t6.key == "val", "explicit string key")

-- nil removes entry (rawset with nil)
local t7 = {a = 1, b = 2}
t7.a = nil
assert(t7.a == nil, "nil removes")

-- rawget / rawset
local t8 = {}
rawset(t8, "key", 99)
assert(rawget(t8, "key") == 99, "rawget/rawset")

print("tables: OK")
