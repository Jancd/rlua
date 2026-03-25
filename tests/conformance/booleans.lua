-- booleans.lua: test boolean logic, short-circuit, truthiness

-- Truthiness: nil and false are falsy, everything else truthy
assert(not not true == true, "not not true")
assert(not not false == false, "not not false")
assert(not not nil == false, "not not nil")
assert(not not 0 == true, "0 is truthy")
assert(not not "" == true, "empty string is truthy")
assert(not not 1 == true, "1 is truthy")

-- not operator
assert(not true == false, "not true")
assert(not false == true, "not false")
assert(not nil == true, "not nil")

-- and short-circuit
assert((true and "yes") == "yes", "true and x")
assert((false and "yes") == false, "false and x")
assert((nil and "yes") == nil, "nil and x")
assert((1 and 2) == 2, "1 and 2")
assert((1 and false) == false, "1 and false")

-- or short-circuit
assert((true or "no") == true, "true or x")
assert((false or "no") == "no", "false or x")
assert((nil or "fallback") == "fallback", "nil or fallback")
assert((false or false) == false, "false or false")

-- and/or idiom (ternary-like)
assert((true and "yes" or "no") == "yes", "ternary true")
assert((false and "yes" or "no") == "no", "ternary false")

-- Comparisons
assert(1 == 1, "eq num")
assert(1 ~= 2, "neq num")
assert("a" == "a", "eq string")
assert("a" ~= "b", "neq string")
assert(true == true, "eq bool")
assert(true ~= false, "neq bool")
assert(nil == nil, "nil eq nil")
assert(nil ~= false, "nil neq false")
assert(nil ~= 0, "nil neq 0")

-- Numeric comparisons
assert(1 < 2, "lt")
assert(2 > 1, "gt")
assert(1 <= 1, "le eq")
assert(1 <= 2, "le lt")
assert(2 >= 2, "ge eq")
assert(3 >= 2, "ge gt")

print("booleans: OK")
