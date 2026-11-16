use crate::runtime::{Boxed, NativeError, Value, gson_tree::TreeStyle, host_gson};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Scope {
    EmptyDocument = 6,
    NonemptyDocument = 7,
    EmptyArray = 1,
    NonemptyArray = 2,
    EmptyObject = 3,
    NonemptyObject = 5,
    DanglingName = 4,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Strictness {
    LegacyStrict,
    Strict,
    Lenient,
}

#[derive(Clone)]
pub(crate) struct State {
    pub(crate) target: String,
    stack: Vec<Scope>,
    deferred_name: Option<String>,
    formatting: host_gson::Style,
    formatting_value: Value,
    strictness: Strictness,
    strictness_value: Value,
    html_safe: bool,
    serialize_nulls: bool,
}
impl State {
    pub(crate) fn new(target: String) -> Self {
        let style = host_gson::Style {
            newline: String::new(),
            indent: String::new(),
            space_after_separators: false,
        };
        Self {
            target,
            stack: vec![Scope::EmptyDocument],
            deferred_name: None,
            formatting_value: host_gson::formatting_style(&style, Some("COMPACT")),
            formatting: style,
            strictness: Strictness::LegacyStrict,
            strictness_value: strictness_value("LEGACY_STRICT"),
            html_safe: false,
            serialize_nulls: true,
        }
    }
    pub(crate) fn tree_style(&self) -> TreeStyle {
        TreeStyle {
            newline: self.formatting.newline.clone(),
            indent: self.formatting.indent.clone(),
            space_after_separators: self.formatting.space_after_separators,
            html_safe: self.html_safe,
            serialize_nulls: self.serialize_nulls,
            lenient: self.strictness == Strictness::Lenient,
        }
    }
    pub(crate) fn base_depth(&self) -> usize {
        self.stack.len().saturating_sub(1)
    }
    pub(crate) fn formatting_value(&self) -> Value {
        self.formatting_value.clone()
    }
    pub(crate) fn strictness_value(&self) -> Value {
        self.strictness_value.clone()
    }
    pub(crate) fn apply_tree_style(&mut self, style: &TreeStyle) {
        self.formatting = host_gson::Style {
            newline: style.newline.clone(),
            indent: style.indent.clone(),
            space_after_separators: style.space_after_separators,
        };
        self.formatting_value = host_gson::formatting_style(&self.formatting, None);
        self.html_safe = style.html_safe;
        self.serialize_nulls = style.serialize_nulls;
    }

    pub(crate) fn set_indent(&mut self, indent: &str) -> Result<(), NativeError> {
        if indent.chars().any(|c| c != ' ' && c != '\t') {
            return Err(illegal_argument(
                "Only combinations of spaces and tabs are allowed in indent.",
            ));
        }
        let style = if indent.is_empty() {
            host_gson::Style {
                newline: String::new(),
                indent: String::new(),
                space_after_separators: false,
            }
        } else {
            host_gson::Style {
                newline: "\n".into(),
                indent: indent.into(),
                space_after_separators: true,
            }
        };
        let value = if indent.is_empty() {
            host_gson::formatting_style(&style, Some("COMPACT"))
        } else {
            host_gson::formatting_style(&style, None)
        };
        self.formatting = style;
        self.formatting_value = value;
        Ok(())
    }
    pub(crate) fn set_formatting_style(&mut self, value: &Value) -> Result<(), NativeError> {
        let Some(style) = host_gson::style_of(value) else {
            return Err(null_pointer());
        };
        self.formatting = style;
        self.formatting_value = value.clone();
        Ok(())
    }
    pub(crate) fn set_lenient(&mut self, b: bool) {
        self.set_strictness_name(if b { "LENIENT" } else { "LEGACY_STRICT" })
    }
    pub(crate) fn set_strictness(&mut self, v: &Value) -> Result<(), NativeError> {
        let Some(n) = strictness_name(v) else {
            return Err(null_pointer());
        };
        self.set_strictness_name(n);
        Ok(())
    }
    fn set_strictness_name(&mut self, n: &str) {
        self.strictness = match n {
            "LENIENT" => Strictness::Lenient,
            "STRICT" => Strictness::Strict,
            _ => Strictness::LegacyStrict,
        };
        self.strictness_value = strictness_value(n);
    }
    pub(crate) fn is_lenient(&self) -> bool {
        self.strictness == Strictness::Lenient
    }
    pub(crate) fn is_html_safe(&self) -> bool {
        self.html_safe
    }
    pub(crate) fn set_html_safe(&mut self, b: bool) {
        self.html_safe = b
    }
    pub(crate) fn serialize_nulls(&self) -> bool {
        self.serialize_nulls
    }
    pub(crate) fn set_serialize_nulls(&mut self, b: bool) {
        self.serialize_nulls = b
    }
    fn peek(&self) -> Result<Scope, NativeError> {
        self.stack
            .last()
            .copied()
            .ok_or_else(|| illegal("JsonWriter is closed."))
    }
    fn replace_top(&mut self, s: Scope) {
        if let Some(x) = self.stack.last_mut() {
            *x = s
        }
    }
    fn newline(&self) -> String {
        if self.formatting.newline.is_empty() && self.formatting.indent.is_empty() {
            return String::new();
        }
        let mut s = self.formatting.newline.clone();
        for _ in 1..self.stack.len() {
            s.push_str(&self.formatting.indent)
        }
        s
    }
    fn comma(&self) -> &str {
        if self.formatting.space_after_separators && self.formatting.newline.is_empty() {
            ", "
        } else {
            ","
        }
    }
    fn colon(&self) -> &str {
        if self.formatting.space_after_separators {
            ": "
        } else {
            ":"
        }
    }
    fn before_name(&mut self) -> Result<String, NativeError> {
        let top = self.peek()?;
        let mut out = String::new();
        match top {
            Scope::NonemptyObject => out.push_str(self.comma()),
            Scope::EmptyObject => {}
            _ => return Err(illegal("Nesting problem.")),
        };
        out.push_str(&self.newline());
        self.replace_top(Scope::DanglingName);
        Ok(out)
    }
    fn write_deferred_name(&mut self) -> Result<String, NativeError> {
        let Some(n) = self.deferred_name.take() else {
            return Ok(String::new());
        };
        let mut out = self.before_name()?;
        out.push_str(&quote_with_html(&n, self.html_safe));
        Ok(out)
    }
    fn before_value(&mut self) -> Result<String, NativeError> {
        let top = self.peek()?;
        let mut out = String::new();
        match top {
            Scope::EmptyDocument => self.replace_top(Scope::NonemptyDocument),
            Scope::NonemptyDocument => {
                if !self.is_lenient() {
                    return Err(illegal("JSON must have only one top-level value."));
                }
            }
            Scope::EmptyArray => {
                self.replace_top(Scope::NonemptyArray);
                out.push_str(&self.newline())
            }
            Scope::NonemptyArray => {
                out.push_str(self.comma());
                out.push_str(&self.newline())
            }
            Scope::DanglingName => {
                out.push_str(self.colon());
                self.replace_top(Scope::NonemptyObject)
            }
            _ => return Err(illegal("Nesting problem.")),
        }
        Ok(out)
    }
    fn raw(&mut self, text: &str) -> Result<String, NativeError> {
        let mut out = self.write_deferred_name()?;
        out.push_str(&self.before_value()?);
        out.push_str(text);
        Ok(out)
    }
    pub(crate) fn begin_array(&mut self) -> Result<String, NativeError> {
        let mut o = self.write_deferred_name()?;
        o.push_str(&self.before_value()?);
        self.stack.push(Scope::EmptyArray);
        o.push('[');
        Ok(o)
    }
    pub(crate) fn begin_object(&mut self) -> Result<String, NativeError> {
        let mut o = self.write_deferred_name()?;
        o.push_str(&self.before_value()?);
        self.stack.push(Scope::EmptyObject);
        o.push('{');
        Ok(o)
    }
    fn close(&mut self, empty: Scope, nonempty: Scope, ch: char) -> Result<String, NativeError> {
        let top = self.peek()?;
        if top != empty && top != nonempty {
            return Err(illegal("Nesting problem."));
        }
        if let Some(n) = &self.deferred_name {
            return Err(illegal(&format!("Dangling name: {n}")));
        }
        self.stack.pop();
        let mut o = String::new();
        if top == nonempty {
            o.push_str(&self.newline())
        }
        o.push(ch);
        Ok(o)
    }
    pub(crate) fn end_array(&mut self) -> Result<String, NativeError> {
        self.close(Scope::EmptyArray, Scope::NonemptyArray, ']')
    }
    pub(crate) fn end_object(&mut self) -> Result<String, NativeError> {
        self.close(Scope::EmptyObject, Scope::NonemptyObject, '}')
    }
    pub(crate) fn name(&mut self, n: &str) -> Result<(), NativeError> {
        if self.deferred_name.is_some() {
            return Err(illegal("Already wrote a name, expecting a value."));
        }
        match self.peek()? {
            Scope::EmptyObject | Scope::NonemptyObject => {
                self.deferred_name = Some(n.into());
                Ok(())
            }
            _ => Err(illegal("Please begin an object before writing a name.")),
        }
    }
    pub(crate) fn string_value(&mut self, v: Option<&str>) -> Result<String, NativeError> {
        match v {
            Some(v) => self.raw(&quote_with_html(v, self.html_safe)),
            None => self.null_value(),
        }
    }
    pub(crate) fn bool_value(&mut self, b: bool) -> Result<String, NativeError> {
        self.raw(if b { "true" } else { "false" })
    }
    pub(crate) fn skip_null_member(&mut self) {
        self.deferred_name = None;
    }

    pub(crate) fn null_value(&mut self) -> Result<String, NativeError> {
        if self.deferred_name.is_some() && !self.serialize_nulls {
            self.deferred_name = None;
            return Ok(String::new());
        }
        self.raw("null")
    }
    pub(crate) fn json_value(&mut self, v: Option<&str>) -> Result<String, NativeError> {
        match v {
            Some(v) => self.raw(v),
            None => self.null_value(),
        }
    }
    pub(crate) fn number_value(&mut self, v: &Value) -> Result<Option<String>, NativeError> {
        // Gson's primitive and Number overloads call writeDeferredName before
        // validating the value. A rejected number therefore consumes the name
        // and leaves the object in its dangling-name state.
        let mut out = self.write_deferred_name()?;
        if matches!(v, Value::Ref(None)) {
            out.push_str(&self.before_value()?);
            out.push_str("null");
            return Ok(Some(out));
        }
        let Some(text) = number_text(v) else {
            return Ok(None);
        };
        if !self.is_lenient() && matches!(text.as_str(), "NaN" | "Infinity" | "-Infinity") {
            return Err(illegal_argument(&format!(
                "Numeric values must be finite, but was {text}"
            )));
        }
        if !matches!(text.as_str(), "NaN" | "Infinity" | "-Infinity") && !valid_number(&text) {
            return Err(illegal_argument(&format!(
                "String created by {} is not a valid JSON number: {text}",
                number_class(v)
            )));
        }
        out.push_str(&self.before_value()?);
        out.push_str(&text);
        Ok(Some(out))
    }
    pub(crate) fn raw_tree(&mut self, text: String) -> Result<String, NativeError> {
        self.raw(&text)
    }
    pub(crate) fn close_document(&mut self) -> Result<(), NativeError> {
        let incomplete = self.stack.len() > 1
            || self
                .stack
                .last()
                .is_some_and(|s| *s != Scope::NonemptyDocument);
        self.stack.clear();
        if incomplete {
            Err(NativeError::UncaughtException {
                class: "java/io/IOException".into(),
                message: Some("Incomplete document".into()),
                frame: None,
            })
        } else {
            Ok(())
        }
    }
    pub(crate) fn flush(&self) -> Result<(), NativeError> {
        self.peek().map(|_| ())
    }
}
fn strictness_value(n: &str) -> Value {
    Value::Ref(Some(format!("enum/com/google/gson/Strictness/{n}")))
}
fn strictness_name(v: &Value) -> Option<&str> {
    crate::runtime::expect_ref(v)?.rsplit_once('/').map(|(_, n)| n)
}
fn illegal(s: &str) -> NativeError {
    NativeError::UncaughtException {
        class: "java/lang/IllegalStateException".into(),
        message: Some(s.into()),
        frame: None,
    }
}
fn illegal_argument(s: &str) -> NativeError {
    NativeError::UncaughtException {
        class: "java/lang/IllegalArgumentException".into(),
        message: Some(s.into()),
        frame: None,
    }
}
fn quote_with_html(text: &str, html: bool) -> String {
    let mut o = String::with_capacity(text.len() + 2);
    o.push('"');
    for c in text.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\t' => o.push_str("\\t"),
            '\u{08}' => o.push_str("\\b"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\u{0c}' => o.push_str("\\f"),
            '<' | '>' | '&' | '=' | '\'' if html => o.push_str(&format!("\\u{:04x}", c as u32)),
            '\u{2028}' => o.push_str("\\u2028"),
            '\u{2029}' => o.push_str("\\u2029"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}
fn number_text(v: &Value) -> Option<String> {
    match v {
        Value::Int(x) => Some(x.to_string()),
        Value::Long(x) => Some(x.to_string()),
        Value::Float(x) => Some(crate::runtime::format_java_float(*x)),
        Value::Boxed(Boxed::Float(b)) => {
            Some(crate::runtime::format_java_float32(f64::from(f32::from_bits(*b))))
        }
        Value::Boxed(Boxed::Integer(x)) => Some(x.to_string()),
        Value::Boxed(Boxed::Byte(x)) => Some(x.to_string()),
        Value::Boxed(Boxed::Short(x)) => Some(x.to_string()),
        _ => None,
    }
}
fn number_class(v: &Value) -> &'static str {
    match v {
        Value::Boxed(Boxed::Float(_)) => "class java.lang.Float",
        Value::Boxed(Boxed::Integer(_)) => "class java.lang.Integer",
        Value::Boxed(Boxed::Byte(_)) => "class java.lang.Byte",
        Value::Boxed(Boxed::Short(_)) => "class java.lang.Short",
        _ => "class java.lang.Number",
    }
}
fn valid_number(s: &str) -> bool {
    let b = s.as_bytes();
    if b.is_empty() {
        return false;
    }
    let mut i = usize::from(b[0] == b'-');
    if i == b.len() {
        return false;
    }
    if b[i] == b'0' {
        i += 1
    } else if b[i].is_ascii_digit() && b[i] != b'0' {
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1
        }
    } else {
        return false;
    }
    if i < b.len() && b[i] == b'.' {
        i += 1;
        let st = i;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1
        }
        if i == st {
            return false;
        }
    }
    if i < b.len() && matches!(b[i], b'e' | b'E') {
        i += 1;
        if i < b.len() && matches!(b[i], b'+' | b'-') {
            i += 1
        }
        let st = i;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1
        }
        if i == st {
            return false;
        }
    }
    i == b.len()
}

pub(crate) fn static_field(r: &crate::runtime::MethodRef) -> Option<Value> {
    (r.owner == "com/google/gson/Strictness"
        && matches!(r.name.as_str(), "LEGACY_STRICT" | "STRICT" | "LENIENT"))
    .then(|| strictness_value(&r.name))
}

fn null_pointer() -> NativeError {
    NativeError::UncaughtException {
        class: "java/lang/NullPointerException".into(),
        message: None,
        frame: None,
    }
}
