-- pcall.lua: test protected calls and error handling

-- Basic pcall success
local ok, result = pcall(function() return 42 end)
assert(ok == true, "pcall success ok")
assert(result == 42, "pcall success result")

-- Basic pcall failure
ok, result = pcall(function() error("boom") end)
assert(ok == false, "pcall failure ok")
assert(result == "boom", "pcall failure msg")

-- pcall with arguments
ok, result = pcall(function(a, b) return a + b end, 3, 4)
assert(ok == true, "pcall args ok")
assert(result == 7, "pcall args result")

-- pcall with multiple return values
local ok2, a, b, c = pcall(function() return 1, 2, 3 end)
assert(ok2 == true, "pcall multi ok")
assert(a == 1 and b == 2 and c == 3, "pcall multi results")

-- pcall catches runtime errors
ok, result = pcall(function()
    local t = nil
    return t.foo  -- attempt to index nil
end)
assert(ok == false, "pcall runtime error")

-- Nested pcall
ok, result = pcall(function()
    local ok2, r = pcall(function() error("inner") end)
    assert(ok2 == false, "inner pcall caught")
    return "recovered"
end)
assert(ok == true, "outer pcall ok")
assert(result == "recovered", "outer pcall result")

-- error with non-string value
ok, result = pcall(function() error(42) end)
assert(ok == false, "pcall non-string error ok")

-- Assert failure is caught by pcall
ok, result = pcall(function() assert(false, "failed!") end)
assert(ok == false, "pcall assert ok")
assert(result == "failed!", "pcall assert msg")

-- pcall with native function
ok, result = pcall(tonumber, "42")
assert(ok == true, "pcall native ok")
assert(result == 42, "pcall native result")

print("pcall: OK")
