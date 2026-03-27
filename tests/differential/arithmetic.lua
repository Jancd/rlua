-- Differential test: arithmetic operations
print(1 + 2)
print(10 - 3)
print(6 * 7)
print(10 / 3)
print(10 % 3)
print(2 ^ 10)
print(-42)
print(-(3 + 4))

-- Precedence
print(2 + 3 * 4)
print((2 + 3) * 4)
print(2 ^ 3 ^ 2)  -- right-assoc: 2^(3^2) = 2^9 = 512

-- Mixed operations
print(1 + 2 * 3 - 4 / 2)
print(10 % 3 + 2 ^ 3)

-- Number coercion from strings
print("10" + 5)
print("3" * "4")

-- Comparisons
print(1 < 2)
print(2 <= 2)
print(3 > 2)
print(3 >= 3)
print(1 == 1)
print(1 ~= 2)

-- Boolean logic
print(true and false)
print(true or false)
print(not true)
print(not false)
print(nil and 1)
print(nil or 1)
print(false or "hello")
print(1 and 2)
