-- gc_stress.lua: stress test that creates many allocations to trigger GC cycles
-- If GC safepoints or collection have bugs, this will crash or hang

-- Create many tables (triggers notify_alloc on NEWTABLE)
local t = {}
for i = 1, 1000 do
    t[i] = {value = i, name = "item" .. i}
end
assert(#t == 1000, "created 1000 tables")
assert(t[500].value == 500, "table 500 value")

-- Create many closures (triggers notify_alloc on CLOSURE)
local funcs = {}
for i = 1, 500 do
    local x = i
    funcs[i] = function() return x end
end
assert(funcs[1]() == 1, "closure 1")
assert(funcs[500]() == 500, "closure 500")

-- Heavy string concatenation (triggers notify_alloc on CONCAT)
local s = ""
for i = 1, 200 do
    s = s .. "x"
end
assert(string.len(s) == 200, "string concat 200")

-- Nested table creation with metatable chains
local chain = {}
for i = 1, 100 do
    chain = setmetatable({level = i}, {__index = chain})
end
assert(chain.level == 100, "chain level 100")

-- Mixed workload: tables + closures + strings in a loop
local results = {}
for i = 1, 300 do
    local tbl = {i = i}
    local fn_val = function() return tbl.i end
    local str_val = "result_" .. i
    results[i] = {tbl = tbl, fn_val = fn_val, str = str_val}
end
assert(#results == 300, "mixed workload count")
assert(results[150].fn_val() == 150, "mixed workload closure")
assert(results[150].str == "result_150", "mixed workload string")

-- Tight loop with backward jumps (tests GC safepoint at back-edges)
local sum = 0
local j = 0
while j < 10000 do
    sum = sum + 1
    j = j + 1
end
assert(sum == 10000, "tight loop sum")

-- Recursive allocation
local function build_tree(depth)
    if depth <= 0 then return {leaf = true} end
    return {
        left = build_tree(depth - 1),
        right = build_tree(depth - 1),
        depth = depth
    }
end
local tree = build_tree(8)  -- 2^8 = 256 leaf nodes, 511 total nodes
assert(tree.depth == 8, "tree root depth")
assert(tree.left.depth == 7, "tree left depth")
assert(tree.left.left.left.left.left.left.left.left.leaf == true, "tree leaf")

print("gc_stress: OK")
