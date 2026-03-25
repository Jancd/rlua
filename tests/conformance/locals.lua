-- locals.lua: test local variable declaration, scoping, shadowing

-- Basic declaration
local x = 10
assert(x == 10, "basic local")

-- Multiple assignment
local a, b, c = 1, 2, 3
assert(a == 1 and b == 2 and c == 3, "multi local")

-- Extra values are discarded
local d, e = 1, 2, 3
assert(d == 1 and e == 2, "extra values discarded")

-- Missing values get nil
local f, g, h = 1, 2
assert(f == 1 and g == 2 and h == nil, "missing values nil")

-- Uninitialized local
local uninit
assert(uninit == nil, "uninitialized local")

-- Shadowing
local s = "outer"
do
    local s = "inner"
    assert(s == "inner", "inner shadow")
end
assert(s == "outer", "outer after shadow")

-- Nested scopes
local v = 1
do
    local v = 2
    do
        local v = 3
        assert(v == 3, "nested 3")
    end
    assert(v == 2, "nested 2")
end
assert(v == 1, "nested 1")

-- Local in same scope can reference previous value
local m = 10
local m = m + 5
assert(m == 15, "self-reference reassign")

print("locals: OK")
