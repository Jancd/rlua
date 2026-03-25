-- repeat.lua: test repeat/until loops

-- Basic repeat
local sum = 0
local i = 1
repeat
    sum = sum + i
    i = i + 1
until i > 10
assert(sum == 55, "repeat sum 1..10")

-- Repeat always executes at least once
local executed = false
repeat
    executed = true
until true
assert(executed, "repeat at least once")

-- Repeat with break
local count = 0
repeat
    count = count + 1
    if count == 3 then break end
until count > 100
assert(count == 3, "repeat break")

-- Nested repeat
local total = 0
local a = 1
repeat
    local b = 1
    repeat
        total = total + 1
        b = b + 1
    until b > 3
    a = a + 1
until a > 3
assert(total == 9, "nested repeat")

-- Until condition uses loop body locals
local x = 0
repeat
    x = x + 1
    local done = (x >= 5)
until done
assert(x == 5, "until with body local")

print("repeat: OK")
