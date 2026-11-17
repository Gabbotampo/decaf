use crate::host_ref::{HostKind, HostRef};
use crate::{MethodRef, NativeError, Value, host_class, host_ids};
use std::sync::{Mutex, OnceLock};

static LOOKUPS: OnceLock<Mutex<std::collections::HashMap<String, (String, String)>>> = OnceLock::new();

fn thrown(class: &str, message: &str) -> NativeError {
    NativeError::UncaughtException { class: class.into(), message: Some(message.into()), frame: None }
}

fn canonical(name: &str) -> String { name.replace('/', ".").replace('$', ".") }
pub(crate) fn static_call(r: &MethodRef, args: &[Value]) -> Option<Result<Value, NativeError>> {
    if r.owner != "net/fabricmc/fabric/api/lookup/v1/block/BlockApiLookup" || r.name != "get" {
        return None;
    }
    let [id, api, ctx] = args else { return None };
    let id = host_ids::identifier_key(id)?.to_string();
    let api = host_class::class_name(api)?.to_string();
    let ctx = host_class::class_name(ctx)?.to_string();
    let key = format!("{id}|{api}|{ctx}");
    let map = LOOKUPS.get_or_init(|| Mutex::new(std::collections::HashMap::new()));
    let mut map = map.lock().unwrap();
    if let Some((old_api, old_ctx)) = map.get(&id) {
        if old_api != &api || old_ctx != &ctx {
            return Some(Err(thrown("java/lang/IllegalArgumentException", &format!("Lookup with id {id} is already registered with api class {} and context class {}. It can\'t be registered with api class {} and context class {}.", canonical(old_api), canonical(old_ctx), canonical(&api), canonical(&ctx)))));
        }
    } else { map.insert(id, (api.clone(), ctx.clone())); }
    Some(Ok(Value::Host(HostRef::new(HostKind::FabricBlockApiLookup, key))))
}
pub(crate) fn instance_call(
    r: &MethodRef,
    recv: &Value,
    args: &[Value],
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
        "registerFallback" => {
            if matches!(args.first(), Some(Value::Ref(None))) {
                Some(Err(thrown("java/lang/NullPointerException", "BlockApiProvider may not be null.")))
            } else { Some(Ok(Value::Void)) }
        },
        _ => None,
    }
}
