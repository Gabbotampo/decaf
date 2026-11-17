use std::sync::atomic::{AtomicU64, Ordering};
static NEXT: AtomicU64 = AtomicU64::new(1);
use crate::{
    MethodRef, NativeError, Value,
    host_ref::{HostKind, HostRef, Methods, Nested},
};
fn none(_: &HostRef, _: &mut dyn Nested) -> Result<Option<String>, NativeError> {
    Ok(None)
}
fn hash(_: &HostRef, _: &mut dyn Nested) -> Result<Option<i32>, NativeError> {
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
    to_string: none,
    hash_code: hash,
    equals: eq,
    instance_of: io,
    crosses_jars: cr,
    compare_to: None,
};
pub(crate) fn static_call(r: &MethodRef, _args: &[Value]) -> Option<Result<Value, NativeError>> {
    if r.owner == "net/fabricmc/fabric/api/creativetab/v1/FabricCreativeModeTab"
        && r.name == "builder"
        && r.descriptor == "()Lnet/minecraft/world/item/CreativeModeTab$Builder;"
    {
        return Some(Ok(Value::Host(HostRef::new(
            HostKind::CreativeModeTabBuilder,
            format!("builder:{}", NEXT.fetch_add(1, Ordering::Relaxed)),
        ))));
    }
    None
}
pub(crate) fn instance_call(
    r: &MethodRef,
    receiver: &Value,
    _args: &[Value],
) -> Option<Result<Value, NativeError>> {
    let Value::Host(h) = receiver else {
        return None;
    };
    if h.kind() == HostKind::CreativeModeTabBuilder {
        if r.owner == "net/minecraft/world/item/CreativeModeTab$Builder"
            && r.name == "build"
            && r.descriptor == "()Lnet/minecraft/world/item/CreativeModeTab;"
        {
            return Some(Ok(Value::Host(HostRef::new(
                HostKind::CreativeModeTab,
                h.key_text().to_owned(),
            ))));
        }
        if r.owner == "net/minecraft/world/item/CreativeModeTab$Builder"
            && r.descriptor
                .ends_with("Lnet/minecraft/world/item/CreativeModeTab$Builder;")
        {
            return Some(Ok(receiver.clone()));
        }
    }
    None
}
