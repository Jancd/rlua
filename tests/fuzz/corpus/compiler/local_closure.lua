local function make_adder(seed)
  return function(value)
    return seed + value
  end
end

local add = make_adder(4)
return add(5)
