-- Differential test: table operations
local t = {1, 2, 3, 4, 5}
print(#t)
print(t[1])
print(t[5])

-- Table with mixed keys
local m = {x = 10, y = 20, [1] = "one"}
print(m.x)
print(m.y)
print(m[1])

-- table.insert
local a = {1, 2, 3}
table.insert(a, 4)
print(#a, a[4])
table.insert(a, 2, 99)
print(a[1], a[2], a[3])

-- table.remove
local b = {10, 20, 30, 40}
local removed = table.remove(b)
print(removed, #b)
removed = table.remove(b, 1)
print(removed, b[1], #b)

-- table.concat
local c = {"a", "b", "c", "d"}
print(table.concat(c, ", "))
print(table.concat(c, "-", 2, 3))

-- table.sort
local d = {5, 3, 1, 4, 2}
table.sort(d)
print(d[1], d[2], d[3], d[4], d[5])

-- pairs iteration (sorted output for determinism)
local keys = {}
local vals = {}
local p = {a = 1, b = 2, c = 3}
for k, v in pairs(p) do
    keys[#keys + 1] = k
end
table.sort(keys)
for i = 1, #keys do
    print(keys[i], p[keys[i]])
end

-- ipairs
local e = {10, 20, 30}
for i, v in ipairs(e) do
    print(i, v)
end

-- Nested tables
local nested = {inner = {1, 2, 3}}
print(nested.inner[2])
