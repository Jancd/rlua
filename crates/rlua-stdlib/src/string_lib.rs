use std::cell::RefCell;
use std::rc::Rc;

use rlua_core::function::LuaFunction;
use rlua_core::value::LuaValue;

// ---------------------------------------------------------------------------
// Basic string functions
// ---------------------------------------------------------------------------

pub fn lua_string_byte(args: &[LuaValue]) -> Result<Vec<LuaValue>, String> {
    let s = get_str(args, "byte")?;
    let i = args.get(1).and_then(|v| v.to_number()).unwrap_or(1.0) as i64;
    let j = args.get(2).and_then(|v| v.to_number()).unwrap_or(i as f64) as i64;
    let bytes = s.as_bytes();
    let len = bytes.len() as i64;
    let i = lua_index(i, len);
    let j = lua_index(j, len);
    let mut results = Vec::new();
    for idx in i..=j {
        if idx >= 0 && (idx as usize) < bytes.len() {
            results.push(LuaValue::Number(bytes[idx as usize] as f64));
        }
    }
    Ok(results)
}

pub fn lua_string_char(args: &[LuaValue]) -> Result<Vec<LuaValue>, String> {
    let mut s = String::new();
    for (i, arg) in args.iter().enumerate() {
        let n = arg
            .to_number()
            .ok_or_else(|| format!("bad argument #{} to 'char' (number expected)", i + 1))?
            as u32;
        s.push(char::from_u32(n).unwrap_or('\u{FFFD}'));
    }
    Ok(vec![LuaValue::from(s)])
}

pub fn lua_string_len(args: &[LuaValue]) -> Result<Vec<LuaValue>, String> {
    let s = get_str(args, "len")?;
    Ok(vec![LuaValue::Number(s.len() as f64)])
}

pub fn lua_string_lower(args: &[LuaValue]) -> Result<Vec<LuaValue>, String> {
    let s = get_str(args, "lower")?;
    Ok(vec![LuaValue::from(s.to_lowercase())])
}

pub fn lua_string_upper(args: &[LuaValue]) -> Result<Vec<LuaValue>, String> {
    let s = get_str(args, "upper")?;
    Ok(vec![LuaValue::from(s.to_uppercase())])
}

pub fn lua_string_reverse(args: &[LuaValue]) -> Result<Vec<LuaValue>, String> {
    let s = get_str(args, "reverse")?;
    Ok(vec![LuaValue::from(s.chars().rev().collect::<String>())])
}

pub fn lua_string_rep(args: &[LuaValue]) -> Result<Vec<LuaValue>, String> {
    let s = get_str(args, "rep")?;
    let n = args
        .get(1)
        .and_then(|v| v.to_number())
        .ok_or("bad argument #2 to 'rep' (number expected)")? as usize;
    Ok(vec![LuaValue::from(s.repeat(n))])
}

pub fn lua_string_sub(args: &[LuaValue]) -> Result<Vec<LuaValue>, String> {
    let s = get_str(args, "sub")?;
    let len = s.len() as i64;
    let i = args.get(1).and_then(|v| v.to_number()).unwrap_or(1.0) as i64;
    let j = args.get(2).and_then(|v| v.to_number()).unwrap_or(-1.0) as i64;
    let start = lua_index(i, len).max(0) as usize;
    let end = (lua_index(j, len) + 1).max(0) as usize;
    let end = end.min(s.len());
    if start >= end {
        Ok(vec![LuaValue::from("")])
    } else {
        Ok(vec![LuaValue::from(&s[start..end])])
    }
}

pub fn lua_string_find(args: &[LuaValue]) -> Result<Vec<LuaValue>, String> {
    let s = get_str(args, "find")?;
    let pattern = get_str_idx(args, 2, "find")?;
    let init = args.get(2).and_then(|v| v.to_number()).unwrap_or(1.0) as i64;
    let plain = args.get(3).map(|v| v.is_truthy()).unwrap_or(false);
    let len = s.len() as i64;
    let start = (lua_index(init, len).max(0)) as usize;

    if plain {
        // Plain string search
        if let Some(pos) = s[start..].find(&*pattern) {
            let abs_start = start + pos;
            Ok(vec![
                LuaValue::Number((abs_start + 1) as f64),
                LuaValue::Number((abs_start + pattern.len()) as f64),
            ])
        } else {
            Ok(vec![LuaValue::Nil])
        }
    } else {
        // Pattern search
        match pattern_match(&s, &pattern, start) {
            Some(m) => {
                let mut results = vec![
                    LuaValue::Number((m.start + 1) as f64),
                    LuaValue::Number(m.end as f64),
                ];
                for cap in &m.captures {
                    results.push(LuaValue::from(&s[cap.0..cap.1]));
                }
                Ok(results)
            }
            None => Ok(vec![LuaValue::Nil]),
        }
    }
}

pub fn lua_string_match(args: &[LuaValue]) -> Result<Vec<LuaValue>, String> {
    let s = get_str(args, "match")?;
    let pattern = get_str_idx(args, 2, "match")?;
    let init = args.get(2).and_then(|v| v.to_number()).unwrap_or(1.0) as i64;
    let len = s.len() as i64;
    let start = (lua_index(init, len).max(0)) as usize;

    match pattern_match(&s, &pattern, start) {
        Some(m) => {
            if m.captures.is_empty() {
                Ok(vec![LuaValue::from(&s[m.start..m.end])])
            } else {
                Ok(m.captures
                    .iter()
                    .map(|(a, b)| LuaValue::from(&s[*a..*b]))
                    .collect())
            }
        }
        None => Ok(vec![LuaValue::Nil]),
    }
}

pub fn lua_string_gmatch(args: &[LuaValue]) -> Result<Vec<LuaValue>, String> {
    let s = get_str(args, "gmatch")?;
    let pattern = get_str_idx(args, 2, "gmatch")?;

    // Collect all matches upfront into a table
    let matches_table = Rc::new(RefCell::new(rlua_core::table::LuaTable::new()));
    let mut start = 0;
    let bytes = s.as_bytes();
    let mut match_idx = 1;
    while start <= bytes.len() {
        match pattern_match(&s, &pattern, start) {
            Some(m) => {
                if m.captures.is_empty() {
                    // Store the whole match
                    let match_data = Rc::new(RefCell::new(rlua_core::table::LuaTable::new()));
                    match_data
                        .borrow_mut()
                        .rawset(LuaValue::Number(1.0), LuaValue::from(&s[m.start..m.end]));
                    matches_table.borrow_mut().rawset(
                        LuaValue::Number(match_idx as f64),
                        LuaValue::Table(match_data),
                    );
                } else {
                    // Store captures
                    let match_data = Rc::new(RefCell::new(rlua_core::table::LuaTable::new()));
                    for (ci, (a, b)) in m.captures.iter().enumerate() {
                        match_data.borrow_mut().rawset(
                            LuaValue::Number((ci + 1) as f64),
                            LuaValue::from(&s[*a..*b]),
                        );
                    }
                    matches_table.borrow_mut().rawset(
                        LuaValue::Number(match_idx as f64),
                        LuaValue::Table(match_data),
                    );
                }
                match_idx += 1;
                if m.end == m.start {
                    start = m.end + 1;
                } else {
                    start = m.end;
                }
            }
            None => break,
        }
    }

    // Store current position index inside the state table at key "__pos"
    matches_table
        .borrow_mut()
        .rawset(LuaValue::from("__pos"), LuaValue::Number(0.0));

    // Return iterator function, matches_table, initial control (nil — unused)
    Ok(vec![
        LuaValue::Function(Rc::new(LuaFunction::Native {
            name: "__gmatch_iter",
            func: gmatch_iterator,
        })),
        LuaValue::Table(matches_table),
        LuaValue::Nil,
    ])
}

fn gmatch_iterator(args: &[LuaValue]) -> Result<Vec<LuaValue>, String> {
    let table = match args.first() {
        Some(LuaValue::Table(t)) => t.clone(),
        _ => return Ok(vec![LuaValue::Nil]),
    };

    // Read and advance the position stored in the state table
    let pos = match table.borrow().rawget(&LuaValue::from("__pos")) {
        LuaValue::Number(n) => n,
        _ => 0.0,
    };
    let next_idx = pos + 1.0;
    table
        .borrow_mut()
        .rawset(LuaValue::from("__pos"), LuaValue::Number(next_idx));

    let match_data = table.borrow().rawget(&LuaValue::Number(next_idx));
    match match_data {
        LuaValue::Table(data) => {
            let data = data.borrow();
            let len = data.len();
            let mut results = Vec::new();
            for i in 1..=len {
                results.push(data.rawget(&LuaValue::Number(i as f64)));
            }
            Ok(results)
        }
        _ => Ok(vec![LuaValue::Nil]),
    }
}

pub fn lua_string_gsub(args: &[LuaValue]) -> Result<Vec<LuaValue>, String> {
    let s = get_str(args, "gsub")?;
    let pattern = get_str_idx(args, 2, "gsub")?;
    let repl = args.get(2).cloned().unwrap_or(LuaValue::from(""));
    let max_s = args.get(3).and_then(|v| v.to_number());
    let max_replacements = max_s.map(|n| n as usize).unwrap_or(usize::MAX);

    let mut result = String::new();
    let mut start = 0;
    let mut count = 0;

    while start <= s.len() && count < max_replacements {
        match pattern_match(&s, &pattern, start) {
            Some(m) => {
                // Append text before the match
                result.push_str(&s[start..m.start]);

                // Apply replacement
                match &repl {
                    LuaValue::String(r) => {
                        // String replacement with capture references (%1, %2, etc.)
                        let r = r.as_str();
                        let mut i = 0;
                        let rbytes = r.as_bytes();
                        while i < rbytes.len() {
                            if rbytes[i] == b'%' && i + 1 < rbytes.len() {
                                let next = rbytes[i + 1];
                                if next.is_ascii_digit() {
                                    let cap_idx = (next - b'0') as usize;
                                    if cap_idx == 0 {
                                        result.push_str(&s[m.start..m.end]);
                                    } else if cap_idx <= m.captures.len() {
                                        let (a, b) = m.captures[cap_idx - 1];
                                        result.push_str(&s[a..b]);
                                    }
                                    i += 2;
                                } else if next == b'%' {
                                    result.push('%');
                                    i += 2;
                                } else {
                                    result.push(rbytes[i] as char);
                                    i += 1;
                                }
                            } else {
                                result.push(rbytes[i] as char);
                                i += 1;
                            }
                        }
                    }
                    LuaValue::Function(f) => {
                        // Function replacement
                        let match_str = if m.captures.is_empty() {
                            vec![LuaValue::from(&s[m.start..m.end])]
                        } else {
                            m.captures
                                .iter()
                                .map(|(a, b)| LuaValue::from(&s[*a..*b]))
                                .collect()
                        };
                        match f.as_ref() {
                            LuaFunction::Native { func, .. } => {
                                let res = func(&match_str)?;
                                let val = res.first().cloned().unwrap_or(LuaValue::Nil);
                                if val.is_truthy() {
                                    result.push_str(&val.to_lua_string());
                                } else {
                                    result.push_str(&s[m.start..m.end]);
                                }
                            }
                            _ => {
                                // For Lua functions, fall back to matched string
                                result.push_str(&s[m.start..m.end]);
                            }
                        }
                    }
                    LuaValue::Table(t) => {
                        // Table replacement
                        let key = if m.captures.is_empty() {
                            LuaValue::from(&s[m.start..m.end])
                        } else {
                            let (a, b) = m.captures[0];
                            LuaValue::from(&s[a..b])
                        };
                        let val = t.borrow().rawget(&key);
                        if val.is_truthy() {
                            result.push_str(&val.to_lua_string());
                        } else {
                            result.push_str(&s[m.start..m.end]);
                        }
                    }
                    _ => {
                        return Err(
                            "bad argument #3 to 'gsub' (string/function/table expected)".to_owned()
                        );
                    }
                }

                count += 1;
                // Avoid infinite loop on empty match
                if m.end == m.start {
                    if start < s.len() {
                        result.push(s.as_bytes()[start] as char);
                    }
                    start = m.end + 1;
                } else {
                    start = m.end;
                }
            }
            None => break,
        }
    }

    // Append remainder
    if start <= s.len() {
        result.push_str(&s[start..]);
    }

    Ok(vec![LuaValue::from(result), LuaValue::Number(count as f64)])
}

pub fn lua_string_format(args: &[LuaValue]) -> Result<Vec<LuaValue>, String> {
    let fmt = get_str(args, "format")?;
    let mut result = String::new();
    let mut arg_idx = 1;
    let bytes = fmt.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'%' {
            i += 1;
            if i >= bytes.len() {
                return Err("invalid format string to 'format'".to_owned());
            }
            match bytes[i] {
                b'%' => {
                    result.push('%');
                    i += 1;
                }
                b'd' | b'i' => {
                    let n = get_format_num(args, arg_idx)?;
                    result.push_str(&format!("{}", n as i64));
                    arg_idx += 1;
                    i += 1;
                }
                b'f' => {
                    let n = get_format_num(args, arg_idx)?;
                    result.push_str(&format!("{:.6}", n));
                    arg_idx += 1;
                    i += 1;
                }
                b'g' => {
                    let n = get_format_num(args, arg_idx)?;
                    // Use Lua-style %g: use shorter of %e and %f
                    let s = format_g(n);
                    result.push_str(&s);
                    arg_idx += 1;
                    i += 1;
                }
                b's' => {
                    let val = args.get(arg_idx).cloned().unwrap_or(LuaValue::Nil);
                    result.push_str(&val.to_lua_string());
                    arg_idx += 1;
                    i += 1;
                }
                b'x' | b'X' => {
                    let n = get_format_num(args, arg_idx)? as i64;
                    if bytes[i] == b'x' {
                        result.push_str(&format!("{:x}", n));
                    } else {
                        result.push_str(&format!("{:X}", n));
                    }
                    arg_idx += 1;
                    i += 1;
                }
                b'o' => {
                    let n = get_format_num(args, arg_idx)? as i64;
                    result.push_str(&format!("{:o}", n));
                    arg_idx += 1;
                    i += 1;
                }
                b'c' => {
                    let n = get_format_num(args, arg_idx)? as u32;
                    result.push(char::from_u32(n).unwrap_or('\u{FFFD}'));
                    arg_idx += 1;
                    i += 1;
                }
                b'q' => {
                    let val = args.get(arg_idx).cloned().unwrap_or(LuaValue::Nil);
                    let s = val.to_lua_string();
                    result.push('"');
                    for ch in s.chars() {
                        match ch {
                            '\\' => result.push_str("\\\\"),
                            '"' => result.push_str("\\\""),
                            '\n' => result.push_str("\\n"),
                            '\r' => result.push_str("\\r"),
                            '\0' => result.push_str("\\0"),
                            c => result.push(c),
                        }
                    }
                    result.push('"');
                    arg_idx += 1;
                    i += 1;
                }
                // Handle width/precision modifiers: skip flags and digits to reach the format char
                b'-' | b'+' | b' ' | b'0' | b'1' | b'2' | b'3' | b'4' | b'5' | b'6' | b'7'
                | b'8' | b'9' | b'.' | b'#' => {
                    // Parse the full format specifier
                    let spec_start = i;
                    while i < bytes.len()
                        && (bytes[i].is_ascii_digit()
                            || bytes[i] == b'-'
                            || bytes[i] == b'+'
                            || bytes[i] == b' '
                            || bytes[i] == b'.'
                            || bytes[i] == b'#'
                            || bytes[i] == b'0')
                    {
                        i += 1;
                    }
                    if i >= bytes.len() {
                        return Err("invalid format string to 'format'".to_owned());
                    }
                    let spec = std::str::from_utf8(&bytes[spec_start..i]).unwrap_or("");
                    match bytes[i] {
                        b'd' | b'i' => {
                            let n = get_format_num(args, arg_idx)? as i64;
                            let formatted = format!("%{}d", spec).replace('%', "");
                            // Simple width handling
                            let width: usize = spec
                                .trim_start_matches('-')
                                .trim_start_matches('0')
                                .split('.')
                                .next()
                                .unwrap_or("0")
                                .parse()
                                .unwrap_or(0);
                            let ns = format!("{}", n);
                            if spec.starts_with('-') {
                                result.push_str(&format!("{:<width$}", ns));
                            } else if spec.starts_with('0') {
                                result.push_str(&format!("{:0>width$}", ns));
                            } else if width > 0 {
                                result.push_str(&format!("{:>width$}", ns));
                            } else {
                                let _ = formatted;
                                result.push_str(&ns);
                            }
                            arg_idx += 1;
                            i += 1;
                        }
                        b'f' => {
                            let n = get_format_num(args, arg_idx)?;
                            // Parse precision from spec
                            let prec = if let Some(dot_pos) = spec.find('.') {
                                spec[dot_pos + 1..].parse::<usize>().unwrap_or(6)
                            } else {
                                6
                            };
                            result.push_str(&format!("{:.prec$}", n));
                            arg_idx += 1;
                            i += 1;
                        }
                        b'g' => {
                            let n = get_format_num(args, arg_idx)?;
                            result.push_str(&format_g(n));
                            arg_idx += 1;
                            i += 1;
                        }
                        b's' => {
                            let val = args.get(arg_idx).cloned().unwrap_or(LuaValue::Nil);
                            result.push_str(&val.to_lua_string());
                            arg_idx += 1;
                            i += 1;
                        }
                        b'x' => {
                            let n = get_format_num(args, arg_idx)? as i64;
                            result.push_str(&format!("{:x}", n));
                            arg_idx += 1;
                            i += 1;
                        }
                        b'X' => {
                            let n = get_format_num(args, arg_idx)? as i64;
                            result.push_str(&format!("{:X}", n));
                            arg_idx += 1;
                            i += 1;
                        }
                        _ => {
                            return Err(format!(
                                "invalid format specifier '%{}{}'",
                                spec, bytes[i] as char
                            ));
                        }
                    }
                }
                c => {
                    return Err(format!("invalid format specifier '%{}'", c as char));
                }
            }
        } else {
            result.push(bytes[i] as char);
            i += 1;
        }
    }

    Ok(vec![LuaValue::from(result)])
}

fn format_g(n: f64) -> String {
    if n == 0.0 {
        return "0".to_owned();
    }
    if n.is_nan() {
        return "-nan".to_owned();
    }
    if n.is_infinite() {
        return if n > 0.0 {
            "inf".to_owned()
        } else {
            "-inf".to_owned()
        };
    }
    // Lua %g: 6 significant digits, no trailing zeros
    let s = format!("{:.6e}", n);
    // Parse mantissa and exponent
    if let Some(e_pos) = s.find('e') {
        let exp: i32 = s[e_pos + 1..].parse().unwrap_or(0);
        if (-4..6).contains(&exp) {
            // Use fixed notation with appropriate precision
            let prec = (5 - exp).max(0) as usize;
            let fixed = format!("{:.prec$}", n);
            // Remove trailing zeros after decimal point
            if fixed.contains('.') {
                let trimmed = fixed.trim_end_matches('0').trim_end_matches('.');
                trimmed.to_owned()
            } else {
                fixed
            }
        } else {
            // Use scientific notation
            format!("{:e}", n)
        }
    } else {
        s
    }
}

// ---------------------------------------------------------------------------
// Lua pattern matching engine
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct MatchResult {
    start: usize,
    end: usize,
    captures: Vec<(usize, usize)>,
}

fn pattern_match(s: &str, pattern: &str, start: usize) -> Option<MatchResult> {
    let sbytes = s.as_bytes();
    let pbytes = pattern.as_bytes();
    let anchored = !pbytes.is_empty() && pbytes[0] == b'^';
    let pstart = if anchored { 1 } else { 0 };

    if anchored {
        let mut captures = Vec::new();
        return match_here(sbytes, start, pbytes, pstart, &mut captures).map(|end| MatchResult {
            start,
            end,
            captures,
        });
    }

    // Try matching at each position from start
    for i in start..=sbytes.len() {
        let mut captures = Vec::new();
        if let Some(end) = match_here(sbytes, i, pbytes, pstart, &mut captures) {
            return Some(MatchResult {
                start: i,
                end,
                captures,
            });
        }
    }
    None
}

fn match_here(
    s: &[u8],
    mut si: usize,
    p: &[u8],
    mut pi: usize,
    captures: &mut Vec<(usize, usize)>,
) -> Option<usize> {
    loop {
        if pi >= p.len() {
            return Some(si);
        }

        // Handle $ anchor at end of pattern
        if p[pi] == b'$' && pi + 1 == p.len() {
            return if si == s.len() { Some(si) } else { None };
        }

        // Handle captures
        if p[pi] == b'(' {
            captures.push((si, si)); // placeholder
            let result = match_here(s, si, p, pi + 1, captures);
            if result.is_some() {
                return result;
            }
            captures.pop();
            return None;
        }
        if p[pi] == b')' {
            if let Some(open_cap) = find_open_capture(captures) {
                captures[open_cap].1 = si;
                let result = match_here(s, si, p, pi + 1, captures);
                if result.is_some() {
                    return result;
                }
                captures[open_cap].1 = captures[open_cap].0; // reset
                return None;
            }
            return None;
        }

        // Get the class length for current pattern element
        let class_len = pattern_class_len(p, pi);

        // Check for quantifiers
        if pi + class_len < p.len() {
            match p[pi + class_len] {
                b'*' => {
                    return match_quantifier(s, si, p, pi, pi + class_len + 1, captures, 0, true);
                }
                b'+' => {
                    return match_quantifier(s, si, p, pi, pi + class_len + 1, captures, 1, true);
                }
                b'-' => {
                    return match_quantifier(s, si, p, pi, pi + class_len + 1, captures, 0, false);
                }
                b'?' => {
                    // Try with one match first, then without
                    if si < s.len()
                        && matches_class(s[si], p, pi)
                        && let Some(r) = match_here(s, si + 1, p, pi + class_len + 1, captures)
                    {
                        return Some(r);
                    }
                    return match_here(s, si, p, pi + class_len + 1, captures);
                }
                _ => {}
            }
        }

        // Single match
        if si < s.len() && matches_class(s[si], p, pi) {
            si += 1;
            pi += class_len;
        } else {
            return None;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn match_quantifier(
    s: &[u8],
    si: usize,
    p: &[u8],
    class_pi: usize,
    rest_pi: usize,
    captures: &mut Vec<(usize, usize)>,
    min_count: usize,
    greedy: bool,
) -> Option<usize> {
    if greedy {
        // Match as many as possible, then try rest
        let mut count = 0;
        while si + count < s.len() && matches_class(s[si + count], p, class_pi) {
            count += 1;
        }
        // Try from longest match down to min_count
        while count >= min_count {
            if let Some(r) = match_here(s, si + count, p, rest_pi, captures) {
                return Some(r);
            }
            if count == 0 {
                break;
            }
            count -= 1;
        }
        None
    } else {
        // Lazy: match as few as possible
        let mut count = min_count;
        // Verify minimum matches
        for i in 0..min_count {
            if si + i >= s.len() || !matches_class(s[si + i], p, class_pi) {
                return None;
            }
        }
        loop {
            if let Some(r) = match_here(s, si + count, p, rest_pi, captures) {
                return Some(r);
            }
            if si + count >= s.len() || !matches_class(s[si + count], p, class_pi) {
                return None;
            }
            count += 1;
        }
    }
}

fn pattern_class_len(p: &[u8], pi: usize) -> usize {
    if pi >= p.len() {
        return 0;
    }
    match p[pi] {
        b'%' => {
            if pi + 1 < p.len() {
                2
            } else {
                1
            }
        }
        b'[' => {
            // Character set: find closing ]
            let mut i = pi + 1;
            if i < p.len() && p[i] == b'^' {
                i += 1;
            }
            if i < p.len() && p[i] == b']' {
                i += 1; // ] right after [ or [^ is literal
            }
            while i < p.len() && p[i] != b']' {
                if p[i] == b'%' && i + 1 < p.len() {
                    i += 2;
                } else {
                    i += 1;
                }
            }
            if i < p.len() {
                i + 1 - pi
            } else {
                p.len() - pi
            }
        }
        _ => 1,
    }
}

fn matches_class(ch: u8, p: &[u8], pi: usize) -> bool {
    if pi >= p.len() {
        return false;
    }
    match p[pi] {
        b'.' => true,
        b'%' => {
            if pi + 1 < p.len() {
                match_char_class(ch, p[pi + 1])
            } else {
                false
            }
        }
        b'[' => match_char_set(ch, p, pi),
        c => ch == c,
    }
}

fn match_char_class(ch: u8, class: u8) -> bool {
    let result = match class.to_ascii_lowercase() {
        b'a' => (ch as char).is_ascii_alphabetic(),
        b'd' => (ch as char).is_ascii_digit(),
        b'l' => (ch as char).is_ascii_lowercase(),
        b'u' => (ch as char).is_ascii_uppercase(),
        b'w' => (ch as char).is_ascii_alphanumeric(),
        b's' => (ch as char).is_ascii_whitespace(),
        b'p' => (ch as char).is_ascii_punctuation(),
        b'c' => (ch as char).is_ascii_control(),
        _ => return ch == class, // escaped literal
    };
    // Uppercase class means complement
    if class.is_ascii_uppercase() {
        !result
    } else {
        result
    }
}

fn match_char_set(ch: u8, p: &[u8], pi: usize) -> bool {
    let mut i = pi + 1;
    let negate = i < p.len() && p[i] == b'^';
    if negate {
        i += 1;
    }

    // Handle ] as first char in set (literal)
    let mut result = false;
    if i < p.len() && p[i] == b']' {
        if ch == b']' {
            result = true;
        }
        i += 1;
    }

    while i < p.len() && p[i] != b']' {
        if p[i] == b'%' && i + 1 < p.len() {
            if match_char_class(ch, p[i + 1]) {
                result = true;
            }
            i += 2;
        } else if i + 2 < p.len() && p[i + 1] == b'-' && p[i + 2] != b']' {
            // Range
            if ch >= p[i] && ch <= p[i + 2] {
                result = true;
            }
            i += 3;
        } else {
            if ch == p[i] {
                result = true;
            }
            i += 1;
        }
    }

    if negate { !result } else { result }
}

fn find_open_capture(captures: &[(usize, usize)]) -> Option<usize> {
    // Find the last capture where start == end (still open)
    captures.iter().rposition(|(start, end)| start == end)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn get_str(args: &[LuaValue], name: &str) -> Result<String, String> {
    match args.first() {
        Some(LuaValue::String(s)) => Ok((**s).clone()),
        Some(LuaValue::Number(n)) => Ok(LuaValue::Number(*n).to_lua_string()),
        Some(other) => Err(format!(
            "bad argument #1 to '{}' (string expected, got {})",
            name,
            other.type_name()
        )),
        None => Err(format!("bad argument #1 to '{}' (string expected)", name)),
    }
}

fn get_str_idx(args: &[LuaValue], idx: usize, name: &str) -> Result<String, String> {
    match args.get(idx - 1) {
        Some(LuaValue::String(s)) => Ok((**s).clone()),
        Some(LuaValue::Number(n)) => Ok(LuaValue::Number(*n).to_lua_string()),
        Some(other) => Err(format!(
            "bad argument #{} to '{}' (string expected, got {})",
            idx,
            name,
            other.type_name()
        )),
        None => Err(format!(
            "bad argument #{} to '{}' (string expected)",
            idx, name
        )),
    }
}

fn get_format_num(args: &[LuaValue], idx: usize) -> Result<f64, String> {
    args.get(idx)
        .and_then(|v| v.to_number())
        .ok_or_else(|| format!("bad argument #{} to 'format' (number expected)", idx + 1))
}

/// Convert Lua 1-based (possibly negative) index to 0-based.
fn lua_index(i: i64, len: i64) -> i64 {
    if i >= 0 { i - 1 } else { len + i }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_simple() {
        let m = pattern_match("hello world", "world", 0).unwrap();
        assert_eq!(m.start, 6);
        assert_eq!(m.end, 11);
    }

    #[test]
    fn test_pattern_classes() {
        let m = pattern_match("abc123", "%d+", 0).unwrap();
        assert_eq!(&"abc123"[m.start..m.end], "123");
    }

    #[test]
    fn test_pattern_captures() {
        let m = pattern_match("2023-01-15", "(%d+)-(%d+)-(%d+)", 0).unwrap();
        assert_eq!(m.captures.len(), 3);
        assert_eq!(&"2023-01-15"[m.captures[0].0..m.captures[0].1], "2023");
        assert_eq!(&"2023-01-15"[m.captures[1].0..m.captures[1].1], "01");
        assert_eq!(&"2023-01-15"[m.captures[2].0..m.captures[2].1], "15");
    }

    #[test]
    fn test_pattern_anchor() {
        assert!(pattern_match("hello", "^hello$", 0).is_some());
        assert!(pattern_match("hello world", "^hello$", 0).is_none());
    }

    #[test]
    fn test_pattern_char_set() {
        let m = pattern_match("abc", "[abc]+", 0).unwrap();
        assert_eq!(&"abc"[m.start..m.end], "abc");

        let m = pattern_match("abc", "[^abc]+", 0);
        assert!(m.is_none());
    }

    #[test]
    fn test_sub() {
        let r = lua_string_sub(&[
            LuaValue::from("hello"),
            LuaValue::Number(2.0),
            LuaValue::Number(4.0),
        ])
        .unwrap();
        assert_eq!(r[0], LuaValue::from("ell"));
    }

    #[test]
    fn test_sub_negative() {
        let r = lua_string_sub(&[LuaValue::from("hello"), LuaValue::Number(-3.0)]).unwrap();
        assert_eq!(r[0], LuaValue::from("llo"));
    }
}
