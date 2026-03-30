local sum = 0

for i = 1, 8 do
  sum = sum + i
end

local tail = sum + 4

return tail, sum, tail - sum
