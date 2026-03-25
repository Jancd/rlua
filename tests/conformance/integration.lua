-- integration.lua: non-trivial Lua program using metatables + standard libraries
-- Tests interaction between multiple M2 features in a realistic scenario

-- === Class system using metatables ===
local Class = {}
Class.__index = Class

function Class:new(name)
    return setmetatable({name = name, items = {}}, self)
end

function Class:add(item)
    table.insert(self.items, item)
end

function Class:count()
    return #self.items
end

function Class:sorted_items()
    local copy = {}
    for i = 1, #self.items do
        copy[i] = self.items[i]
    end
    table.sort(copy)
    return copy
end

-- === Use the class ===
local bag = Class:new("inventory")
bag:add("sword")
bag:add("apple")
bag:add("shield")
bag:add("potion")

assert(bag:count() == 4, "class count")
assert(bag.name == "inventory", "class name")

local sorted = bag:sorted_items()
assert(sorted[1] == "apple", "sorted 1")
assert(sorted[2] == "potion", "sorted 2")
assert(sorted[3] == "shield", "sorted 3")
assert(sorted[4] == "sword", "sorted 4")

-- Original is not mutated
assert(bag.items[1] == "sword", "original unchanged")

-- === String processing ===
local csv = "alice,30,engineer;bob,25,designer;carol,35,manager"
local people = {}
for record in string.gmatch(csv, "[^;]+") do
    local name, age, role = string.match(record, "([^,]+),(%d+),([^,]+)")
    table.insert(people, {name = name, age = tonumber(age), role = role})
end

assert(#people == 3, "parsed 3 records")
assert(people[1].name == "alice", "person 1 name")
assert(people[2].age == 25, "person 2 age")
assert(people[3].role == "manager", "person 3 role")

-- Format output
local lines = {}
for i = 1, #people do
    local p = people[i]
    lines[i] = string.format("%s (%d) - %s", p.name, p.age, p.role)
end
assert(lines[1] == "alice (30) - engineer", "formatted line 1")
assert(lines[2] == "bob (25) - designer", "formatted line 2")

-- === Math computations ===
local function distance(x1, y1, x2, y2)
    return math.sqrt((x2-x1)^2 + (y2-y1)^2)
end

local d = distance(0, 0, 3, 4)
assert(d == 5, "distance 3-4-5")

local d2 = distance(1, 1, 4, 5)
assert(d2 == 5, "distance shifted")

-- === Error handling with xpcall ===
local errors = {}
local function safe_divide(a, b)
    return xpcall(function()
        if b == 0 then error("division by zero") end
        return a / b
    end, function(e)
        table.insert(errors, e)
        return "error: " .. e
    end)
end

local ok, result = safe_divide(10, 2)
assert(ok == true and result == 5, "safe divide ok")

ok, result = safe_divide(10, 0)
assert(ok == false, "safe divide error")
assert(#errors == 1, "error logged")
assert(string.find(errors[1], "division by zero") ~= nil, "error message")

-- === Table manipulation ===
local stack = {}
for i = 1, 10 do
    table.insert(stack, i * i)
end
assert(#stack == 10, "stack size")
assert(stack[10] == 100, "stack top")

-- Pop last 3
local popped = {}
for i = 1, 3 do
    table.insert(popped, table.remove(stack))
end
assert(#stack == 7, "stack after pop")
assert(popped[1] == 100 and popped[2] == 81 and popped[3] == 64, "popped values")

-- Concat
assert(table.concat({"a", "b", "c"}, "-") == "a-b-c", "concat join")

-- === Recursive metatable chain ===
local proto = {greet = function(self) return "Hello, " .. self.name end}
local mid = setmetatable({}, {__index = proto})
local obj = setmetatable({name = "world"}, {__index = mid})
assert(obj:greet() == "Hello, world", "metatable chain method")

-- === String method syntax ===
local s = "  Hello, World!  "
local trimmed = s:match("^%s*(.-)%s*$")
assert(trimmed == "Hello, World!", "string match trim")

local upper = ("hello"):upper()
assert(upper == "HELLO", "method upper")

local replaced = ("banana"):gsub("a", "o")
assert(replaced == "bonono", "method gsub")

-- === Protected call with non-string error ===
-- Note: table errors are converted to strings in current implementation
ok, result = pcall(function()
    error(42)
end)
assert(ok == false, "numeric error caught")
assert(result == "42", "numeric error as string")

-- === Math edge cases ===
assert(math.max(1, 2, 3, 4, 5) == 5, "max of 5")
assert(math.min(5, 4, 3, 2, 1) == 1, "min of 5")
assert(math.abs(-math.pi) == math.pi, "abs pi")
assert(math.floor(math.pi) == 3, "floor pi")
assert(math.ceil(math.pi) == 4, "ceil pi")

print("integration: OK")
