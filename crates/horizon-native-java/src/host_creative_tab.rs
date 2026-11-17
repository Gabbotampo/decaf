use crate::{
    MethodRef, NativeError, Value,
    host_ref::{HostKind, HostRef},
};
use std::collections::BTreeMap;
const FABRIC: &str = "net/fabricmc/fabric/api/creativetab/v1/FabricCreativeModeTab";
const BUILDER: &str = "net/minecraft/world/item/CreativeModeTab$Builder";
const TAB: &str = "net/minecraft/world/item/CreativeModeTab";
#[derive(Default)]
pub(crate) struct State {
    next: u64,
    builders: BTreeMap<String, Option<Value>>,
    tabs: BTreeMap<String, Value>,
}
impl State {
    pub(crate) fn values(&self) -> impl Iterator<Item = &Value> {
        self.builders.values().flatten().chain(self.tabs.values())
    }
}
pub(crate) fn static_call(
    state: &mut State,
    r: &MethodRef,
    _: &[Value],
) -> Option<Result<Value, NativeError>> {
    if r.owner == FABRIC
        && r.name == "builder"
        && r.descriptor == "()Lnet/minecraft/world/item/CreativeModeTab$Builder;"
    {
        state.next += 1;
        let k = format!("b#{}", state.next);
        state.builders.insert(k.clone(), None);
        return Some(Ok(Value::Host(HostRef::new(
            HostKind::CreativeModeTabBuilder,
            k,
        ))));
    }
    None
}
pub(crate) fn instance_call(
    state: &mut State,
    r: &MethodRef,
    recv: &Value,
    args: &[Value],
) -> Option<Result<Value, NativeError>> {
    let Value::Host(h) = recv else { return None };
    match h.kind() {
        HostKind::CreativeModeTabBuilder => {
            if r.owner != BUILDER {
                return None;
            };
            if r.name == "build" && r.descriptor == "()Lnet/minecraft/world/item/CreativeModeTab;" {
                state.next += 1;
                let k = format!("t#{}", state.next);
                let title = state
                    .builders
                    .get(h.key())?
                    .clone()
                    .unwrap_or(Value::Ref(Some("component/empty".into())));
                state.tabs.insert(k.clone(), title);
                return Some(Ok(Value::Host(HostRef::new(HostKind::CreativeModeTab, k))));
            }
            if r.descriptor
                .ends_with("Lnet/minecraft/world/item/CreativeModeTab$Builder;")
            {
                if r.name == "title" {
                    state
                        .builders
                        .insert(h.key().to_owned(), args.first().cloned());
                }
                return Some(Ok(recv.clone()));
            }
            None
        }
        HostKind::CreativeModeTab => {
            if r.owner == TAB
                && r.name == "getDisplayName"
                && r.descriptor == "()Lnet/minecraft/network/chat/Component;"
            {
                return Some(Ok(state.tabs.get(h.key())?.clone()));
            }
            None
        }
        _ => None,
    }
}
