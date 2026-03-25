-- while.lua: test while loops, break

-- Basic while
local sum = 0
local i = 1
while i <= 10 do
    sum = sum + i
    i = i + 1
end
assert(sum == 55, "while sum 1..10")

-- While with break
local count = 0
local j = 1
while true do
    if j > 5 then break end
    count = count + 1
    j = j + 1
end
assert(count == 5, "while break")

-- While that never executes
local ran = false
while false do
    ran = true
end
assert(ran == false, "while false")

-- Nested while
local total = 0
local a = 1
while a <= 3 do
    local b = 1
    while b <= 3 do
        total = total + 1
        b = b + 1
    end
    a = a + 1
end
assert(total == 9, "nested while")

-- Break from inner loop only
local outer_count = 0
local inner_count = 0
a = 1
while a <= 3 do
    local b = 1
    while b <= 10 do
        if b > 2 then break end
        inner_count = inner_count + 1
        b = b + 1
    end
    outer_count = outer_count + 1
    a = a + 1
end
assert(outer_count == 3, "outer continues after inner break")
assert(inner_count == 6, "inner breaks at 2 each time")

print("while: OK")
