local wrapped = coroutine.wrap(function(seed)
  coroutine.yield(seed + 1)
  return seed + 2
end)

local first = wrapped(4)
local second = wrapped()
return first, second
