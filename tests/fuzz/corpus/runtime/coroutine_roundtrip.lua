local co = coroutine.create(function(seed)
  coroutine.yield(seed + 1)
  return seed + 2
end)

local ok1, value1 = coroutine.resume(co, 10)
local ok2, value2 = coroutine.resume(co)
return ok1, value1, ok2, value2
