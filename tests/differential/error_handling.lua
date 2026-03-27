-- Differential test: error handling
-- pcall success
local ok, r = pcall(function() return 42 end)
print(ok, r)

-- pcall with error
ok, r = pcall(function() error("boom", 0) end)
print(ok, r)

-- pcall with non-string error (level 0 to avoid location prefix)
ok, r = pcall(function() error(123, 0) end)
print(ok, r)

-- Nested pcall
ok, r = pcall(function()
    local ok2, r2 = pcall(function() error("inner", 0) end)
    return ok2, r2
end)
print(ok, r)

-- xpcall with handler
ok, r = xpcall(
    function() error("fail", 0) end,
    function(e) return "caught: " .. tostring(e) end
)
print(ok, r)

-- xpcall success passes through
ok, r = xpcall(function() return 99 end, function(e) return e end)
print(ok, r)

-- assert success
local a, b, c = assert(1, 2, 3)
print(a, b, c)

-- assert failure caught by pcall
ok, r = pcall(function() assert(false, "assert failed") end)
print(ok)
-- Don't check exact error message since it may have location prefix

-- type()
print(type(nil))
print(type(true))
print(type(42))
print(type("hello"))
print(type({}))
print(type(print))

-- tostring / tonumber
print(tostring(nil))
print(tostring(true))
print(tostring(42))
print(tostring("hello"))
print(tonumber("42"))
print(tonumber("3.14"))
print(tonumber("0xff"))
print(tonumber("abc"))

-- select
print(select("#", 1, 2, 3))
print(select(2, "a", "b", "c"))

-- unpack
print(unpack({10, 20, 30}))
