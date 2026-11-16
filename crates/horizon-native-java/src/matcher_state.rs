use crate::{NativeError, Value};
use std::collections::BTreeMap;
use std::sync::{Mutex, OnceLock};
static STATE: OnceLock<Mutex<BTreeMap<String, (usize, usize)>>> = OnceLock::new();
fn state() -> &'static Mutex<BTreeMap<String, (usize, usize)>> {
    STATE.get_or_init(|| Mutex::new(BTreeMap::new()))
}
pub(crate) fn call(r: &crate::MethodRef, receiver: &Value) -> Option<Result<Value, NativeError>> {
    let Value::Host(h) = receiver else {
        return None;
    };
    if h.kind() != crate::host_ref::HostKind::Matcher {
        return None;
    };
    let vals = h.values()?;
    let [Value::Ref(Some(p)), Value::Ref(Some(input))] = <[Value; 2]>::try_from(vals).ok()? else {
        return None;
    };
    let key = h.key().to_owned();
    match (r.name.as_str(), r.descriptor.as_str()) {
        ("matches", "()Z") => Some(
            crate::java_regex::matches(p, input)
                .map(|b| Value::Int(b as i32))
                .map_err(|_| NativeError::Truncated),
        ),
        ("find", "()Z") => None,
        ("start", "()I") | ("end", "()I") => {
            let g = state().lock().ok()?;
            let (a, b) = g.get(&key).copied()?;
            Some(Ok(Value::Int(if r.name == "start" { a } else { b } as i32)))
        }
        _ => None,
    }
}
