-- if_else.lua: test if/elseif/else, nested conditionals

-- Basic if
local x = 0
if true then x = 1 end
assert(x == 1, "basic if true")

local y = 0
if false then y = 1 end
assert(y == 0, "basic if false")

-- if/else
local a
if true then a = "yes" else a = "no" end
assert(a == "yes", "if true else")

if false then a = "yes" else a = "no" end
assert(a == "no", "if false else")

-- if/elseif/else
local function classify(n)
    if n < 0 then
        return "negative"
    elseif n == 0 then
        return "zero"
    elseif n > 0 then
        return "positive"
    end
end

assert(classify(-5) == "negative", "classify negative")
assert(classify(0) == "zero", "classify zero")
assert(classify(5) == "positive", "classify positive")

-- Nested if
local result = "none"
if true then
    if true then
        result = "both"
    end
end
assert(result == "both", "nested if")

-- Condition with expression
local val = 10
if val > 5 then
    result = "big"
else
    result = "small"
end
assert(result == "big", "conditional expr")

-- Multiple elseif
local function grade(score)
    if score >= 90 then return "A"
    elseif score >= 80 then return "B"
    elseif score >= 70 then return "C"
    elseif score >= 60 then return "D"
    else return "F"
    end
end

assert(grade(95) == "A", "grade A")
assert(grade(85) == "B", "grade B")
assert(grade(75) == "C", "grade C")
assert(grade(65) == "D", "grade D")
assert(grade(55) == "F", "grade F")

print("if_else: OK")
