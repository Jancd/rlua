-- table_lib.lua: test table standard library

-- table.insert (append)
local t = {1, 2, 3}
table.insert(t, 4)
assert(#t == 4, "insert append length")
assert(t[4] == 4, "insert append value")

-- table.insert (positional)
local t2 = {1, 3, 4}
table.insert(t2, 2, 2)
assert(#t2 == 4, "insert pos length")
assert(t2[1] == 1 and t2[2] == 2 and t2[3] == 3 and t2[4] == 4, "insert pos order")

-- table.insert at beginning
local t3 = {2, 3}
table.insert(t3, 1, 1)
assert(t3[1] == 1 and t3[2] == 2 and t3[3] == 3, "insert at beginning")

-- table.remove (default: last element)
local t4 = {10, 20, 30}
local removed = table.remove(t4)
assert(removed == 30, "remove last value")
assert(#t4 == 2, "remove last length")

-- table.remove (positional)
local t5 = {10, 20, 30, 40}
removed = table.remove(t5, 2)
assert(removed == 20, "remove pos value")
assert(#t5 == 3, "remove pos length")
assert(t5[1] == 10 and t5[2] == 30 and t5[3] == 40, "remove pos shift")

-- table.remove first element
local t6 = {1, 2, 3}
removed = table.remove(t6, 1)
assert(removed == 1, "remove first value")
assert(t6[1] == 2 and t6[2] == 3, "remove first shift")

-- table.sort (default: numeric)
local s1 = {5, 3, 1, 4, 2}
table.sort(s1)
assert(s1[1] == 1 and s1[2] == 2 and s1[3] == 3 and s1[4] == 4 and s1[5] == 5, "sort numeric")

-- table.sort (default: strings)
local s2 = {"banana", "apple", "cherry"}
table.sort(s2)
assert(s2[1] == "apple" and s2[2] == "banana" and s2[3] == "cherry", "sort string")

-- table.sort with Lua comparator
local s5 = {5, 3, 1, 4, 2}
table.sort(s5, function(a, b)
    return a > b
end)
assert(s5[1] == 5 and s5[2] == 4 and s5[3] == 3 and s5[4] == 2 and s5[5] == 1, "sort comparator")

local ok, err = pcall(function()
    table.sort({3, 2, 1}, function(a, b)
        error("cmp boom")
    end)
end)
assert(ok == false, "sort comparator errors propagate")
assert(string.find(err, "cmp boom"), "sort comparator error message")

local yield_ok, yield_err = coroutine.resume(coroutine.create(function()
    local values = {3, 2, 1}
    table.sort(values, function(a, b)
        coroutine.yield("blocked")
        return a < b
    end)
end))
assert(yield_ok == false, "sort comparator yield across native boundary errors")
assert(string.find(yield_err, "native callback boundary"), "sort comparator yield boundary message")

-- table.sort single element
local s3 = {42}
table.sort(s3)
assert(s3[1] == 42, "sort single")

-- table.sort empty
local s4 = {}
table.sort(s4)
assert(#s4 == 0, "sort empty")

-- table.concat
local c1 = {"hello", "world", "!"}
assert(table.concat(c1) == "helloworld!", "concat no sep")
assert(table.concat(c1, ", ") == "hello, world, !", "concat with sep")
assert(table.concat(c1, "-") == "hello-world-!", "concat with dash")

-- table.concat with numbers
local c2 = {1, 2, 3}
assert(table.concat(c2, "+") == "1+2+3", "concat numbers")

-- table.concat with range
local c3 = {"a", "b", "c", "d", "e"}
assert(table.concat(c3, ",", 2, 4) == "b,c,d", "concat range")
assert(table.concat(c3, ",", 3) == "c,d,e", "concat from index")

-- table.concat empty
assert(table.concat({}) == "", "concat empty")

-- table.concat single
assert(table.concat({"only"}, ",") == "only", "concat single")

print("table_lib: OK")
