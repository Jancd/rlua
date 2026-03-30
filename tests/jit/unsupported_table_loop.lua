local t = {}
local sum = 0

for i = 1, 6 do
  t[i] = i
  sum = sum + t[i]
end

return sum
