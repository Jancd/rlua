-- string_lib.lua: test string standard library

-- string.byte / string.char
assert(string.byte("A") == 65, "byte A")
assert(string.byte("hello", 2) == 101, "byte position")
assert(string.char(65) == "A", "char 65")
assert(string.char(72, 101, 108, 108, 111) == "Hello", "char multi")

-- string.len
assert(string.len("hello") == 5, "len")
assert(string.len("") == 0, "len empty")

-- string.lower / string.upper
assert(string.lower("Hello World") == "hello world", "lower")
assert(string.upper("Hello World") == "HELLO WORLD", "upper")

-- string.reverse
assert(string.reverse("abc") == "cba", "reverse")
assert(string.reverse("") == "", "reverse empty")
assert(string.reverse("a") == "a", "reverse single")

-- string.rep
assert(string.rep("ab", 3) == "ababab", "rep 3")
assert(string.rep("x", 0) == "", "rep 0")
assert(string.rep("x", 1) == "x", "rep 1")

-- string.sub
assert(string.sub("hello", 2, 4) == "ell", "sub range")
assert(string.sub("hello", 2) == "ello", "sub from")
assert(string.sub("hello", -3) == "llo", "sub negative start")
assert(string.sub("hello", 1, -2) == "hell", "sub negative end")

-- Method syntax via string metatable
assert(("hello"):len() == 5, "method len")
assert(("Hello"):lower() == "hello", "method lower")
assert(("hello"):upper() == "HELLO", "method upper")
assert(("abc"):reverse() == "cba", "method reverse")
assert(("hello"):sub(2, 4) == "ell", "method sub")
assert(("ab"):rep(3) == "ababab", "method rep")

-- string.find (plain)
local s, e = string.find("hello world", "world")
assert(s == 7 and e == 11, "find plain")

s, e = string.find("hello world", "xyz")
assert(s == nil, "find not found")

s, e = string.find("hello world", "world", 1, true)
assert(s == 7 and e == 11, "find plain flag")

-- string.find (pattern)
s, e = string.find("hello123", "%d+")
assert(s == 6 and e == 8, "find pattern digits")

s, e = string.find("abc def ghi", "%a+", 5)
assert(s == 5 and e == 7, "find pattern from pos")

-- string.match
assert(string.match("hello123", "%d+") == "123", "match digits")
assert(string.match("hello", "%d+") == nil, "match no digits")

-- string.match with captures
local y, m, d = string.match("2024-01-15", "(%d+)-(%d+)-(%d+)")
assert(y == "2024" and m == "01" and d == "15", "match captures")

-- string.gmatch
local words = {}
for w in string.gmatch("hello world foo", "%a+") do
    words[#words + 1] = w
end
assert(#words == 3, "gmatch count")
assert(words[1] == "hello" and words[2] == "world" and words[3] == "foo", "gmatch words")

-- gmatch with captures
local pairs_found = {}
for k, v in string.gmatch("a=1,b=2,c=3", "(%a)=(%d)") do
    pairs_found[k] = v
end
assert(pairs_found.a == "1" and pairs_found.b == "2" and pairs_found.c == "3", "gmatch captures")

-- string.gsub
local result, count = string.gsub("hello world", "o", "0")
assert(result == "hell0 w0rld", "gsub replace")
assert(count == 2, "gsub count")

-- gsub with max replacements
result, count = string.gsub("aaa", "a", "b", 2)
assert(result == "bba", "gsub max")
assert(count == 2, "gsub max count")

-- gsub with pattern
result = string.gsub("hello 123 world 456", "%d+", "NUM")
assert(result == "hello NUM world NUM", "gsub pattern")

-- gsub with native function replacement
result = string.gsub("hello", "%a", string.upper)
assert(result == "HELLO", "gsub function")

-- gsub with table replacement
local t = {hello = "HI", world = "EARTH"}
result = string.gsub("hello world", "%a+", t)
assert(result == "HI EARTH", "gsub table")

-- string.format
assert(string.format("%d", 42) == "42", "format %d")
assert(string.format("%05d", 42) == "00042", "format %05d")
assert(string.format("%s", "hello") == "hello", "format %s")
assert(string.format("%q", 'he said "hi"') == '"he said \\"hi\\""', "format %q")
assert(string.format("%x", 255) == "ff", "format %x")
assert(string.format("%X", 255) == "FF", "format %X")
assert(string.format("%o", 8) == "10", "format %o")
assert(string.format("%c", 65) == "A", "format %c")
assert(string.format("%%") == "%", "format %%")

-- format with multiple args
assert(string.format("%s = %d", "x", 42) == "x = 42", "format multi")

-- format float
local f = string.format("%.2f", 3.14159)
assert(f == "3.14", "format %.2f")

-- Pattern edge cases
-- Anchored patterns
assert(string.find("hello", "^hello$") == 1, "anchored full match")
assert(string.find("hello world", "^hello$") == nil, "anchored no match")

-- Character classes
assert(string.match("abc123", "%a+") == "abc", "class %a")
assert(string.match("abc123", "%d+") == "123", "class %d")
assert(string.match("  hello  ", "%S+") == "hello", "class %S")

-- Quantifiers
assert(string.match("aabbb", "a+") == "aa", "quantifier +")
assert(string.match("bbb", "a*b+") == "bbb", "quantifier *")
assert(string.match("aabbb", "a?b") == "ab", "quantifier ?")

-- Escape special chars
assert(string.find("hello.world", "%.") == 6, "escape dot")
assert(string.find("(test)", "%(") == 1, "escape paren")

print("string_lib: OK")
