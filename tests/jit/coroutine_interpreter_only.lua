local function runner(limit)
    local sum = 0
    for i = 1, limit do
        sum = sum + i
    end

    local echoed = coroutine.yield(sum)

    for i = 1, limit do
        sum = sum + i
    end

    return sum, echoed
end

local co = coroutine.create(runner)
local ok, first = coroutine.resume(co, 50)
assert(ok == true)

local ok2, total, echoed = coroutine.resume(co, "resume-token")
assert(ok2 == true)

print(first)
print(total, echoed)

return coroutine.status(co)
