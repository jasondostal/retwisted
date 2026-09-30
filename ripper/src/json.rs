//! Just enough JSON to reproduce Python's `json.dumps(obj, indent=1)` byte
//! for byte: insertion-ordered objects, `ensure_ascii` escaping (non-ASCII
//! as `\uXXXX`, astral chars as surrogate pairs), `", "`-free separators
//! under indent, and `float.__repr__` number formatting.

#[derive(Clone, Debug, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(Obj),
}

/// Insertion-ordered object with Python dict assignment semantics: setting
/// an existing key replaces the value in place.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Obj(pub Vec<(String, Json)>);

impl Obj {
    pub fn new() -> Self {
        Obj(Vec::new())
    }
    pub fn set(&mut self, k: impl Into<String>, v: impl Into<Json>) {
        let k = k.into();
        let v = v.into();
        if let Some(slot) = self.0.iter_mut().find(|(kk, _)| *kk == k) {
            slot.1 = v;
        } else {
            self.0.push((k, v));
        }
    }
    pub fn get(&self, k: &str) -> Option<&Json> {
        self.0.iter().find(|(kk, _)| kk == k).map(|(_, v)| v)
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl From<Obj> for Json {
    fn from(o: Obj) -> Self {
        Json::Obj(o)
    }
}
impl From<Vec<Json>> for Json {
    fn from(v: Vec<Json>) -> Self {
        Json::Arr(v)
    }
}
impl From<&str> for Json {
    fn from(s: &str) -> Self {
        Json::Str(s.to_string())
    }
}
impl From<String> for Json {
    fn from(s: String) -> Self {
        Json::Str(s)
    }
}
impl From<i64> for Json {
    fn from(i: i64) -> Self {
        Json::Int(i)
    }
}
impl From<bool> for Json {
    fn from(b: bool) -> Self {
        Json::Bool(b)
    }
}
impl From<f64> for Json {
    fn from(f: f64) -> Self {
        Json::Float(f)
    }
}

/// Python `float.__repr__`: shortest round-trip digits; positional notation
/// for 1e-4 <= |x| < 1e16, else `d.ddde+XX`; always a `.0` if integral.
pub fn py_float_repr(x: f64) -> String {
    if x.is_nan() {
        return "NaN".into();
    }
    if x.is_infinite() {
        return if x > 0.0 {
            "Infinity".into()
        } else {
            "-Infinity".into()
        };
    }
    if x == 0.0 {
        return if x.is_sign_negative() {
            "-0.0".into()
        } else {
            "0.0".into()
        };
    }
    // `{:e}` gives the shortest round-trip digits as d.ddddde<exp>.
    let e = format!("{:e}", x.abs());
    let (mant, exp) = e.split_once('e').unwrap();
    let exp: i32 = exp.parse().unwrap();
    let digits: String = mant.chars().filter(|c| *c != '.').collect();
    let sign = if x < 0.0 { "-" } else { "" };
    let nd = digits.len() as i32;
    if (-4..16).contains(&exp) {
        if exp >= 0 {
            let int_len = exp + 1;
            if nd <= int_len {
                format!("{sign}{}{}.0", digits, "0".repeat((int_len - nd) as usize))
            } else {
                let (a, b) = digits.split_at(int_len as usize);
                format!("{sign}{a}.{b}")
            }
        } else {
            format!("{sign}0.{}{}", "0".repeat((-exp - 1) as usize), digits)
        }
    } else {
        let m = if nd == 1 {
            digits.clone()
        } else {
            format!("{}.{}", &digits[..1], &digits[1..])
        };
        let es = if exp < 0 {
            format!("-{:02}", -exp)
        } else {
            format!("+{exp:02}")
        };
        format!("{sign}{m}e{es}")
    }
}

fn write_str(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0C}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 || (c as u32) > 0x7E => {
                let mut buf = [0u16; 2];
                for u in c.encode_utf16(&mut buf) {
                    out.push_str(&format!("\\u{:04x}", u));
                }
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

fn write(out: &mut String, v: &Json, level: usize) {
    match v {
        Json::Null => out.push_str("null"),
        Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Json::Int(i) => out.push_str(&i.to_string()),
        Json::Float(f) => out.push_str(&py_float_repr(*f)),
        Json::Str(s) => write_str(out, s),
        Json::Arr(a) => {
            if a.is_empty() {
                out.push_str("[]");
                return;
            }
            out.push('[');
            for (i, x) in a.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push('\n');
                out.push_str(&" ".repeat(level + 1));
                write(out, x, level + 1);
            }
            out.push('\n');
            out.push_str(&" ".repeat(level));
            out.push(']');
        }
        Json::Obj(o) => {
            if o.0.is_empty() {
                out.push_str("{}");
                return;
            }
            out.push('{');
            for (i, (k, x)) in o.0.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push('\n');
                out.push_str(&" ".repeat(level + 1));
                write_str(out, k);
                out.push_str(": ");
                write(out, x, level + 1);
            }
            out.push('\n');
            out.push_str(&" ".repeat(level));
            out.push('}');
        }
    }
}

/// `json.dumps(v, indent=1)`.
pub fn dumps_indent1(v: &Json) -> String {
    let mut s = String::new();
    write(&mut s, v, 0);
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn float_repr_matches_python() {
        for (x, want) in [
            (1.0, "1.0"),
            (125.0, "125.0"),
            (0.5, "0.5"),
            (22050.0 / 22254.0, "0.9908331086546239"),
            (1e16, "1e+16"),
            (1.5e-5, "1.5e-05"),
            (0.0001, "0.0001"),
            (123456789012345.6, "123456789012345.6"),
            (-2.5, "-2.5"),
        ] {
            assert_eq!(py_float_repr(x), want, "{x}");
        }
    }

    #[test]
    fn dumps_shape() {
        let mut o = Obj::new();
        o.set("a", Json::Arr(vec![Json::Int(1), Json::Int(2)]));
        o.set("b", Json::Obj(Obj::new()));
        o.set("c", Json::Str("\u{e9}\"x\n".into()));
        o.set("d", Json::Null);
        assert_eq!(dumps_indent1(&Json::Obj(o)), "{\n \"a\": [\n  1,\n  2\n ],\n \"b\": {},\n \"c\": \"\\u00e9\\\"x\\n\",\n \"d\": null\n}");
    }
}
