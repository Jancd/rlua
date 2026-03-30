local function run(seed)
  local sum = seed

  for i = 1, 5 do
    sum = sum + i
  end

  return sum
end

local baseline = run(0)
local exit_one = run("0")
local exit_two = run("0")
local recovered = run(0)

return baseline, exit_one, exit_two, recovered
