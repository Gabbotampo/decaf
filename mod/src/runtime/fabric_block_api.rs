use crate::runtime::host_ref::{HostKind, HostRef};
use crate::runtime::{MethodRef, NativeError, Value, host_class, host_ids};
pub(crate) fn static_call(r: &MethodRef, args: &[Value]) -> Option<Result<Value, NativeError>> {
    if r.owner != "net/fabricmc/fabric/api/lookup/v1/block/BlockApiLookup" || r.name != "get" {
        return None;
    }
    let [id, api, ctx] = args else { return None };
    let k = format!(
        "{}|{}|{}",
        host_ids::identifier_key(id)?,
        host_class::class_name(api)?,
        host_class::class_name(ctx)?
    );
    Some(Ok(Value::Host(HostRef::new(
        HostKind::FabricBlockApiLookup,
        k,
    ))))
}
pub(crate) fn instance_call(
    r: &MethodRef,
    recv: &Value,
    _: &[Value],
) -> Option<Result<Value, NativeError>> {
    let Value::Host(h) = recv else { return None };
    if h.kind() != HostKind::FabricBlockApiLookup {
        return None;
    };
    let p = h.key().split('|').collect::<Vec<_>>();
    match r.name.as_str() {
        "getId" => Some(Ok(host_ids::identifier_of_text(p.first().copied()?))),
        "apiClass" => Some(Ok(host_class::class_value(p.get(1).copied()?))),
        "contextClass" => Some(Ok(host_class::class_value(p.get(2).copied()?))),
        "registerFallback" => Some(Ok(Value::Void)),
        _ => None,
    }
}
