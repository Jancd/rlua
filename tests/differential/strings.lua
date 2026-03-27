-- Differential test: string operations
print(#"hello")
print("hello" .. " " .. "world")
print("abc" .. 123)
print(42 .. "!")

-- string library basics
print(string.len("hello"))
print(string.upper("hello"))
print(string.lower("HELLO"))
print(string.rep("ab", 3))
print(string.reverse("hello"))
print(string.sub("hello", 2, 4))
print(string.sub("hello", -3))
print(string.byte("A"))
print(string.char(65))

-- string.find
print(string.find("hello world", "world"))
print(string.find("hello", "xyz"))

-- string.format
print(string.format("%d", 42))
print(string.format("%s", "hello"))
print(string.format("%05d", 42))
print(string.format("%.2f", 3.14159))
print(string.format("%x", 255))
print(string.format("%%"))

-- string.match
print(string.match("hello123", "%d+"))
print(string.match("2024-01-15", "(%d+)-(%d+)-(%d+)"))

-- string.gsub
print(string.gsub("hello world", "o", "0"))
print(string.gsub("aaa", "a", "bb", 2))

-- Method syntax
print(("hello"):upper())
print(("hello"):len())
