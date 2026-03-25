-- numeric_for.lua: test for i = start, limit, step

-- Basic counting
local sum = 0
for i = 1, 10 do
    sum = sum + i
end
assert(sum == 55, "for 1..10")

-- Counting down
local result = ""
for i = 3, 1, -1 do
    result = result .. i
end
assert(result == "321", "for 3..1 step -1")

-- Step of 2
sum = 0
for i = 1, 10, 2 do
    sum = sum + i
end
assert(sum == 25, "for step 2: 1+3+5+7+9")

-- Empty range (start > limit with positive step)
local ran = false
for i = 10, 1 do
    ran = true
end
assert(ran == false, "empty range")

-- Empty range (start < limit with negative step)
ran = false
for i = 1, 10, -1 do
    ran = true
end
assert(ran == false, "empty range negative")

-- Single iteration
local count = 0
for i = 5, 5 do
    count = count + 1
end
assert(count == 1, "single iteration")

-- Loop variable is local
local outer_i = "unchanged"
for i = 1, 3 do
    -- i is local to the loop
end
assert(outer_i == "unchanged", "for var is local")

-- Nested numeric for
local total = 0
for i = 1, 3 do
    for j = 1, 3 do
        total = total + 1
    end
end
assert(total == 9, "nested for")

-- Break in numeric for
sum = 0
for i = 1, 100 do
    if i > 5 then break end
    sum = sum + i
end
assert(sum == 15, "for break: 1+2+3+4+5")

-- Fractional step
count = 0
for i = 0, 1, 0.25 do
    count = count + 1
end
assert(count == 5, "fractional step 0, 0.25, 0.5, 0.75, 1.0")

print("numeric_for: OK")
