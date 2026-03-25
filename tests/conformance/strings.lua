-- strings.lua: test string concatenation, length, coercion

-- Concatenation
assert("hello" .. " " .. "world" == "hello world", "concat")
assert("abc" .. "def" == "abcdef", "concat 2")

-- Length operator
assert(#"hello" == 5, "length")
assert(#"" == 0, "empty length")
assert(#"abc" == 3, "length 3")

-- Number to string coercion in concat
assert("val=" .. 42 == "val=42", "number concat")
assert(10 .. 20 == "1020", "number .. number")

-- tostring / tonumber
assert(tostring(42) == "42", "tostring number")
assert(tostring(true) == "true", "tostring bool")
assert(tostring(nil) == "nil", "tostring nil")
assert(tostring("hello") == "hello", "tostring string")

assert(tonumber("42") == 42, "tonumber string")
assert(tonumber("3.14") == 3.14, "tonumber float")
assert(tonumber("hello") == nil, "tonumber invalid")
assert(tonumber(42) == 42, "tonumber number")

print("strings: OK")
