-- metatables.lua: test metatable system and metamethods

-- setmetatable / getmetatable
local t = {}
local mt = {}
assert(setmetatable(t, mt) == t, "setmetatable returns table")
assert(getmetatable(t) == mt, "getmetatable returns metatable")

-- setmetatable(t, nil) removes metatable
setmetatable(t, nil)
assert(getmetatable(t) == nil, "remove metatable")

-- __metatable protection
local t2 = {}
setmetatable(t2, {__metatable = "protected"})
assert(getmetatable(t2) == "protected", "__metatable returned by getmetatable")
local ok, err = pcall(setmetatable, t2, {})
assert(ok == false, "__metatable blocks setmetatable")

-- __tostring metamethod (only works with native functions in current implementation)
-- Lua closure __tostring not supported yet since tostring() is native and can't call Lua functions

-- __index as table
local base = {x = 10, y = 20}
local derived = setmetatable({}, {__index = base})
assert(derived.x == 10, "__index table lookup")
assert(derived.y == 20, "__index table lookup 2")
derived.x = 99
assert(derived.x == 99, "__index own field takes priority")

-- __index chaining
local a = {val = "found"}
local b = setmetatable({}, {__index = a})
local c = setmetatable({}, {__index = b})
assert(c.val == "found", "__index chain")

-- __index as function
local fi = setmetatable({}, {
    __index = function(self, key)
        return key .. "_default"
    end
})
assert(fi.hello == "hello_default", "__index function")
assert(fi.world == "world_default", "__index function 2")

-- __newindex as function
local log = {}
local ni = setmetatable({}, {
    __newindex = function(self, key, value)
        rawset(self, key, value * 2)
    end
})
ni.x = 5
assert(ni.x == 10, "__newindex function")

-- __newindex as table (redirects writes)
local store = {}
local proxy = setmetatable({}, {__newindex = store})
proxy.foo = "bar"
assert(store.foo == "bar", "__newindex table redirect")
assert(rawget(proxy, "foo") == nil, "__newindex didn't write to proxy")

-- Arithmetic metamethods: __add
local Vec = {}
Vec.__index = Vec
function Vec.new(x, y) return setmetatable({x=x, y=y}, Vec) end
Vec.__add = function(a, b) return Vec.new(a.x + b.x, a.y + b.y) end
Vec.__sub = function(a, b) return Vec.new(a.x - b.x, a.y - b.y) end
Vec.__mul = function(a, b)
    if type(a) == "number" then return Vec.new(a * b.x, a * b.y) end
    if type(b) == "number" then return Vec.new(a.x * b, a.y * b) end
    return Vec.new(a.x * b.x, a.y * b.y)
end
Vec.__unm = function(a) return Vec.new(-a.x, -a.y) end

local v1 = Vec.new(1, 2)
local v2 = Vec.new(3, 4)
local v3 = v1 + v2
assert(v3.x == 4 and v3.y == 6, "__add")

local v4 = v2 - v1
assert(v4.x == 2 and v4.y == 2, "__sub")

local v5 = v1 * 3
assert(v5.x == 3 and v5.y == 6, "__mul scalar")

local v6 = -v1
assert(v6.x == -1 and v6.y == -2, "__unm")

-- __eq metamethod
Vec.__eq = function(a, b) return a.x == b.x and a.y == b.y end
local va = Vec.new(1, 2)
local vb = Vec.new(1, 2)
local vc = Vec.new(3, 4)
assert(va == vb, "__eq true")
assert(not (va == vc), "__eq false")

-- __lt and __le
Vec.__lt = function(a, b) return (a.x * a.x + a.y * a.y) < (b.x * b.x + b.y * b.y) end
Vec.__le = function(a, b) return (a.x * a.x + a.y * a.y) <= (b.x * b.x + b.y * b.y) end
assert(va < vc, "__lt")
assert(va <= vb, "__le equal")
assert(va <= vc, "__le less")

-- __len metamethod
local lenobj = setmetatable({}, {__len = function(self) return 42 end})
assert(#lenobj == 42, "__len")

-- __concat metamethod (only tested for table-first case)
-- Note: partial __concat support — see implementation for details

-- __call metamethod
local callable = setmetatable({}, {
    __call = function(self, a, b) return a + b end
})
assert(callable(3, 4) == 7, "__call")

-- rawget bypasses __index
local raw_test = setmetatable({}, {__index = function() return "meta" end})
assert(raw_test.anything == "meta", "metamethod used normally")
assert(rawget(raw_test, "anything") == nil, "rawget bypasses __index")

-- rawset bypasses __newindex
local raw_test2 = setmetatable({}, {__newindex = function() error("should not be called") end})
rawset(raw_test2, "key", "value")
assert(rawget(raw_test2, "key") == "value", "rawset bypasses __newindex")

-- rawequal bypasses __eq
local re1 = setmetatable({}, {__eq = function() return true end})
local re2 = setmetatable({}, {__eq = function() return true end})
assert(rawequal(re1, re1) == true, "rawequal same")
assert(rawequal(re1, re2) == false, "rawequal different")

print("metatables: OK")
