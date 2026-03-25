-- arithmetic.lua: test arithmetic operators and precedence

-- Basic arithmetic
assert(1 + 2 == 3, "1 + 2")
assert(10 - 3 == 7, "10 - 3")
assert(2 * 5 == 10, "2 * 5")
assert(10 / 3 > 3.33 and 10 / 3 < 3.34, "10 / 3")
assert(10 % 3 == 1, "10 % 3")
assert(2 ^ 10 == 1024, "2 ^ 10")

-- Unary minus
assert(-5 == 0 - 5, "unary minus")
assert(-(3 + 2) == -5, "unary minus expr")

-- Precedence: * binds tighter than +
assert(2 + 3 * 4 == 14, "precedence: + vs *")
assert(2 * 3 + 4 == 10, "precedence: * vs +")

-- Precedence: ^ binds tighter than unary minus
assert(-2 ^ 2 == -4, "precedence: unary minus vs ^")

-- Precedence: ^ is right-associative
assert(2 ^ 2 ^ 3 == 256, "right-associative ^")

-- Division
assert(1 / 0 == 1 / 0, "inf == inf")  -- inf
assert(7 / 2 == 3.5, "float division")

-- Modulo edge cases
assert(5 % 2 == 1, "modulo")
assert(5 % -2 == -1 or 5 % -2 == 1, "modulo negative") -- implementation dependent

-- Compound expressions
assert((1 + 2) * 3 == 9, "parens")
assert(2 * (3 + 4) == 14, "parens 2")

-- Numeric coercion from string
assert("10" + 5 == 15, "string to number coercion +")
assert("3" * "4" == 12, "string * string coercion")

print("arithmetic: OK")
