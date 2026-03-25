-- generic_for.lua: test for k, v in pairs/ipairs

-- ipairs iterates array part in order
local t = {10, 20, 30}
local sum = 0
local count = 0
for i, v in ipairs(t) do
    sum = sum + v
    count = count + 1
    assert(t[i] == v, "ipairs index matches")
end
assert(sum == 60, "ipairs sum")
assert(count == 3, "ipairs count")

-- ipairs stops at first nil
local t2 = {1, 2, nil, 4}
count = 0
for i, v in ipairs(t2) do
    count = count + 1
end
assert(count == 2, "ipairs stops at nil")

-- pairs iterates all entries
local t3 = {a = 1, b = 2, c = 3}
local total = 0
local keys = 0
for k, v in pairs(t3) do
    total = total + v
    keys = keys + 1
end
assert(total == 6, "pairs total")
assert(keys == 3, "pairs keys")

-- pairs on array table
local t4 = {10, 20, 30}
sum = 0
for k, v in pairs(t4) do
    sum = sum + v
end
assert(sum == 60, "pairs array sum")

-- Empty table
count = 0
for k, v in pairs({}) do
    count = count + 1
end
assert(count == 0, "pairs empty")

count = 0
for i, v in ipairs({}) do
    count = count + 1
end
assert(count == 0, "ipairs empty")

-- Generic for with break
count = 0
for i, v in ipairs({1, 2, 3, 4, 5}) do
    if i > 3 then break end
    count = count + 1
end
assert(count == 3, "generic for break")

print("generic_for: OK")
