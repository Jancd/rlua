-- scoping.lua: test do/end blocks, variable scoping

-- do/end creates a scope
local x = 1
do
    local x = 2
    assert(x == 2, "inner x")
end
assert(x == 1, "outer x restored")

-- Variables from outer scope are visible
local a = 10
do
    assert(a == 10, "outer visible in do")
    a = 20
end
assert(a == 20, "outer modified in do")

-- Nested do blocks
do
    local a = 1
    do
        local b = 2
        do
            local c = 3
            assert(a == 1 and b == 2 and c == 3, "nested do access")
        end
        assert(a == 1 and b == 2, "middle scope")
    end
    assert(a == 1, "outer scope")
end

-- Shadowing across blocks
local val = "original"
do
    local val = "shadow1"
    do
        local val = "shadow2"
        assert(val == "shadow2", "deepest shadow")
    end
    assert(val == "shadow1", "middle shadow")
end
assert(val == "original", "original restored")

-- for loop variable is scoped
local sum = 0
for i = 1, 3 do
    sum = sum + i
end
-- i is not accessible here (would be nil if referenced)

-- Function scope
local function f()
    local inner = 42
    return inner
end
assert(f() == 42, "function scope")

-- Local function in do block
do
    local function g() return "g" end
    assert(g() == "g", "local function in do")
end

print("scoping: OK")
