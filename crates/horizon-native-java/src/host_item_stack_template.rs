use crate::{
    NativeError, Value,
    host_ref::{HostRef, Methods, Nested},
};
fn ts(_: &HostRef, _: &mut dyn Nested) -> Result<Option<String>, NativeError> {
    Ok(None)
}
fn hc(_: &HostRef, _: &mut dyn Nested) -> Result<Option<i32>, NativeError> {
    Ok(None)
}
fn eq(_: &HostRef, _: &Value, _: &mut dyn Nested) -> Result<Option<bool>, NativeError> {
    Ok(None)
}
fn io(_: &HostRef, _: &str, _: &dyn crate::host_ref::ClassHierarchy) -> Option<bool> {
    None
}
fn cr(_: &HostRef, _: &dyn Fn(&Value) -> bool) -> bool {
    false
}
pub(crate) const METHODS: Methods = Methods {
    to_string: ts,
    hash_code: hc,
    equals: eq,
    instance_of: io,
    crosses_jars: cr,
    compare_to: None,
};
