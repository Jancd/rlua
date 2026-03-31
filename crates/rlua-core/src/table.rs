use std::cell::RefCell;
use std::rc::Rc;

use crate::value::LuaValue;

pub type TableRef = Rc<RefCell<LuaTable>>;

/// A Lua table with an array part and a hash part.
#[derive(Debug, Clone)]
pub struct LuaTable {
    /// 1-based array part. Index 0 is unused; array[i] corresponds to Lua index i.
    array: Vec<LuaValue>,
    /// Hash part: linear scan of key-value pairs.
    hash: Vec<(LuaValue, LuaValue)>,
    /// Optional metatable for metamethod dispatch.
    metatable: Option<TableRef>,
}

impl LuaTable {
    pub fn new() -> Self {
        Self {
            array: Vec::new(),
            hash: Vec::new(),
            metatable: None,
        }
    }

    pub fn with_capacity(array_size: usize, hash_size: usize) -> Self {
        Self {
            array: Vec::with_capacity(array_size),
            hash: Vec::with_capacity(hash_size),
            metatable: None,
        }
    }

    pub fn metatable(&self) -> Option<&TableRef> {
        self.metatable.as_ref()
    }

    pub fn set_metatable(&mut self, mt: Option<TableRef>) {
        self.metatable = mt;
    }

    /// Look up a metamethod by name (e.g., "__add") in this table's metatable.
    pub fn get_metamethod(&self, name: &str) -> Option<LuaValue> {
        let mt = self.metatable.as_ref()?;
        let val = mt.borrow().rawget(&LuaValue::from(name));
        if matches!(val, LuaValue::Nil) {
            None
        } else {
            Some(val)
        }
    }

    pub fn new_ref() -> TableRef {
        Rc::new(RefCell::new(Self::new()))
    }

    /// Get a value by key without metamethods.
    pub fn rawget(&self, key: &LuaValue) -> LuaValue {
        if let Some(idx) = self.array_index(key)
            && idx < self.array.len()
        {
            return self.array[idx].clone();
        }
        for (k, v) in &self.hash {
            if lua_raw_equal(k, key) {
                return v.clone();
            }
        }
        LuaValue::Nil
    }

    /// Set a value by key without metamethods.
    pub fn rawset(&mut self, key: LuaValue, value: LuaValue) {
        if matches!(key, LuaValue::Nil) {
            return; // Cannot use nil as a table key
        }
        if let LuaValue::Number(n) = &key
            && n.is_nan()
        {
            return; // Cannot use NaN as a table key
        }

        // Try array part
        if let Some(idx) = self.array_index(&key) {
            if matches!(value, LuaValue::Nil) {
                // Setting to nil in array part
                if idx < self.array.len() {
                    self.array[idx] = LuaValue::Nil;
                }
                return;
            }
            // Extend array if this is the next sequential index
            if idx == self.array.len() {
                self.array.push(value);
                return;
            }
            if idx < self.array.len() {
                self.array[idx] = value;
                return;
            }
        }

        // Hash part: update existing or insert
        if matches!(value, LuaValue::Nil) {
            self.hash.retain(|(k, _)| !lua_raw_equal(k, &key));
            return;
        }
        for (k, v) in &mut self.hash {
            if lua_raw_equal(k, &key) {
                *v = value;
                return;
            }
        }
        self.hash.push((key, value));
    }

    /// Lua `#` operator: find the boundary of the array part.
    /// Returns n such that t[n] ~= nil and t[n+1] == nil.
    pub fn len(&self) -> usize {
        // Find last non-nil element in array part
        let mut n = self.array.len();
        while n > 0 && matches!(self.array[n - 1], LuaValue::Nil) {
            n -= 1;
        }
        n
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0 && self.hash.is_empty()
    }

    /// Table iteration: returns the next key-value pair after `key`.
    /// Pass `Nil` to get the first pair.
    pub fn next(&self, key: &LuaValue) -> Option<(LuaValue, LuaValue)> {
        if matches!(key, LuaValue::Nil) {
            // Find first non-nil in array part
            for (i, v) in self.array.iter().enumerate() {
                if !matches!(v, LuaValue::Nil) {
                    return Some((LuaValue::Number((i + 1) as f64), v.clone()));
                }
            }
            // Then first in hash part
            return self.hash.first().map(|(k, v)| (k.clone(), v.clone()));
        }

        // Check if key is in array part
        if let Some(idx) = self.array_index(key)
            && idx < self.array.len()
        {
            // Find next non-nil in array part after idx
            for i in (idx + 1)..self.array.len() {
                if !matches!(self.array[i], LuaValue::Nil) {
                    return Some((LuaValue::Number((i + 1) as f64), self.array[i].clone()));
                }
            }
            // Transition to hash part
            return self.hash.first().map(|(k, v)| (k.clone(), v.clone()));
        }

        // Key is in hash part — find it and return the next entry
        for (i, (k, _)) in self.hash.iter().enumerate() {
            if lua_raw_equal(k, key) {
                return self.hash.get(i + 1).map(|(k, v)| (k.clone(), v.clone()));
            }
        }

        None
    }

    /// Iterate all key-value pairs (array + hash) for GC marking.
    pub fn iter_pairs(&self) -> impl Iterator<Item = (LuaValue, LuaValue)> + '_ {
        let array_iter = self
            .array
            .iter()
            .enumerate()
            .filter(|(_, v)| !matches!(v, LuaValue::Nil))
            .map(|(i, v)| (LuaValue::Number((i + 1) as f64), v.clone()));
        let hash_iter = self.hash.iter().map(|(k, v)| (k.clone(), v.clone()));
        array_iter.chain(hash_iter)
    }

    /// Convert a LuaValue key to an array index (0-based) if it represents a positive integer.
    fn array_index(&self, key: &LuaValue) -> Option<usize> {
        if let LuaValue::Number(n) = key {
            let i = *n as usize;
            if i as f64 == *n && i >= 1 {
                return Some(i - 1); // Convert 1-based to 0-based
            }
        }
        None
    }
}

impl Default for LuaTable {
    fn default() -> Self {
        Self::new()
    }
}

/// Raw equality comparison for table keys (no metamethods).
fn lua_raw_equal(a: &LuaValue, b: &LuaValue) -> bool {
    match (a, b) {
        (LuaValue::Nil, LuaValue::Nil) => true,
        (LuaValue::Boolean(a), LuaValue::Boolean(b)) => a == b,
        (LuaValue::Number(a), LuaValue::Number(b)) => a == b,
        (LuaValue::String(a), LuaValue::String(b)) => a == b,
        (LuaValue::Table(a), LuaValue::Table(b)) => Rc::ptr_eq(a, b),
        (LuaValue::Function(a), LuaValue::Function(b)) => Rc::ptr_eq(a, b),
        (LuaValue::Thread(a), LuaValue::Thread(b)) => Rc::ptr_eq(a, b),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::LuaValue;

    fn s(val: &str) -> LuaValue {
        LuaValue::from(val)
    }

    #[test]
    fn rawset_rawget_integer_keys() {
        let mut t = LuaTable::new();
        t.rawset(LuaValue::Number(1.0), s("a"));
        t.rawset(LuaValue::Number(2.0), s("b"));
        t.rawset(LuaValue::Number(3.0), s("c"));

        assert_eq!(t.rawget(&LuaValue::Number(1.0)), s("a"));
        assert_eq!(t.rawget(&LuaValue::Number(2.0)), s("b"));
        assert_eq!(t.rawget(&LuaValue::Number(3.0)), s("c"));
        assert_eq!(t.rawget(&LuaValue::Number(4.0)), LuaValue::Nil);
        assert_eq!(t.len(), 3);
    }

    #[test]
    fn rawset_rawget_string_keys() {
        let mut t = LuaTable::new();
        t.rawset(s("x"), LuaValue::Number(42.0));
        t.rawset(s("y"), LuaValue::Number(99.0));

        assert_eq!(t.rawget(&s("x")), LuaValue::Number(42.0));
        assert_eq!(t.rawget(&s("y")), LuaValue::Number(99.0));
        assert_eq!(t.rawget(&s("z")), LuaValue::Nil);
    }

    #[test]
    fn rawset_nil_removes() {
        let mut t = LuaTable::new();
        t.rawset(s("x"), LuaValue::Number(1.0));
        t.rawset(s("x"), LuaValue::Nil);
        assert_eq!(t.rawget(&s("x")), LuaValue::Nil);
    }

    #[test]
    fn next_iterates_all() {
        let mut t = LuaTable::new();
        t.rawset(LuaValue::Number(1.0), s("a"));
        t.rawset(LuaValue::Number(2.0), s("b"));
        t.rawset(s("k"), LuaValue::Boolean(true));

        let mut count = 0;
        let mut key = LuaValue::Nil;
        while let Some((k, _v)) = t.next(&key) {
            key = k;
            count += 1;
        }
        assert_eq!(count, 3);
    }

    #[test]
    fn nil_and_nan_keys_ignored() {
        let mut t = LuaTable::new();
        t.rawset(LuaValue::Nil, LuaValue::Number(1.0));
        assert_eq!(t.len(), 0);

        t.rawset(LuaValue::Number(f64::NAN), LuaValue::Number(1.0));
        assert_eq!(t.len(), 0);
    }

    #[test]
    fn len_with_holes() {
        let mut t = LuaTable::new();
        t.rawset(LuaValue::Number(1.0), s("a"));
        t.rawset(LuaValue::Number(2.0), s("b"));
        t.rawset(LuaValue::Number(3.0), LuaValue::Nil);
        // array is ["a", "b"] after nil set, len should be 2
        assert_eq!(t.len(), 2);
    }
}
