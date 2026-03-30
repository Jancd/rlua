local function run(seed, limit)
  local sum = seed

  for i = 1, limit do
    sum = sum + i
  end

  return sum
end

local first = run(0, 5)
local second = run("0", 1)

return first, second
