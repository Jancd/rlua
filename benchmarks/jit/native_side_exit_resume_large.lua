local sum = 0

for i = 1, 200000 do
  sum = sum + i
end

local tail = sum + 123

return tail, sum, tail - sum
