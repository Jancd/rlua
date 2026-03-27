-- xpcall_errors.lua: test xpcall and enhanced error handling

-- Basic xpcall success
local ok, result = xpcall(function() return 42 end, function(e) return "ERR: " .. e end)
assert(ok == true, "xpcall success ok")
assert(result == 42, "xpcall success result")

-- Basic xpcall failure with handler
ok, result = xpcall(
    function() error("boom") end,
    function(e) return "handled: " .. e end
)
assert(ok == false, "xpcall failure ok")
assert(string.find(result, "handled: ", 1, true), "xpcall handler called")
assert(string.find(result, "boom", 1, true), "xpcall handler called boom")

-- xpcall with arguments
ok, result = xpcall(
    function(a, b) return a + b end,
    function(e) return e end,
    10, 20
)
assert(ok == true, "xpcall args ok")
assert(result == 30, "xpcall args result")

-- xpcall handler receives the error value
ok, result = xpcall(
    function() error(42) end,
    function(e) return e * 2 end
)
assert(ok == false, "xpcall numeric error ok")
assert(result == 84, "xpcall handler transforms error")

-- xpcall with error in handler returns original error
ok, result = xpcall(
    function() error("original") end,
    function(e) error("handler error") end
)
assert(ok == false, "xpcall handler error ok")

-- Nested xpcall
ok, result = xpcall(
    function()
        local ok2, r2 = xpcall(
            function() error("inner") end,
            function(e) return "caught:" .. e end
        )
        assert(ok2 == false, "inner xpcall caught")
        return r2
    end,
    function(e) return "outer:" .. e end
)
assert(ok == true, "nested xpcall outer ok")
assert(string.find(result, "caught:", 1, true), "nested xpcall result caught")
assert(string.find(result, "inner", 1, true), "nested xpcall result inner")

-- pcall inside xpcall
ok, result = xpcall(
    function()
        local ok2, r2 = pcall(function() error("pcall_err") end)
        assert(ok2 == false, "pcall inside xpcall")
        return "recovered"
    end,
    function(e) return "should not reach" end
)
assert(ok == true, "pcall inside xpcall ok")
assert(result == "recovered", "pcall inside xpcall result")

-- xpcall with multiple return values
ok, result = xpcall(
    function() return 1, 2, 3 end,
    function(e) return e end
)
assert(ok == true, "xpcall multi ok")
assert(result == 1, "xpcall multi first result")

-- error with nil (no message)
ok, result = pcall(function() error() end)
assert(ok == false, "error nil ok")

-- assert with custom message
ok, result = pcall(function() assert(false, "custom msg") end)
assert(ok == false, "assert custom ok")
assert(string.find(result, "custom msg", 1, true), "assert custom msg")

-- assert passes through on success
local a, b, c = assert(1, 2, 3)
assert(a == 1 and b == 2 and c == 3, "assert passthrough")

-- Nested error handling
ok, result = pcall(function()
    local ok2 = pcall(function()
        local ok3 = pcall(function()
            error("deep")
        end)
        assert(ok3 == false, "level 3 caught")
        error("level 2")
    end)
    assert(ok2 == false, "level 2 caught")
    return "all caught"
end)
assert(ok == true and result == "all caught", "nested error handling")

print("xpcall_errors: OK")
