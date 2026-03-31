local total = 0
for i = 1, 4 do
  if i % 2 == 0 then
    total = total + i
  else
    total = total - i
  end
end
return total
