//! Typed handles for NeoForge's `PayloadRegistrar` builder.

use std::sync::atomic::{AtomicU64, Ordering};

use crate::{
    Value,
    host_ref::{HostKind, HostRef},
};

static NEXT_REGISTRAR: AtomicU64 = AtomicU64::new(1);

pub(crate) fn registrar(version: &Value) -> Value {
    let serial = NEXT_REGISTRAR.fetch_add(1, Ordering::Relaxed);
    Value::Host(HostRef::of_values(
        HostKind::NeoForgePayloadRegistrar,
        &[Value::Long(serial as i64), version.clone()],
    ))
}

pub(crate) fn instance_call(
    _r: &crate::MethodRef,
    _receiver: &Value,
    _args: &[Value],
) -> Option<Value> {
    // Refinements carry transport state in NeoForge. Until that state is
    // modeled, leave the call at the counted boundary.
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MethodRef, host_ref::ClassHierarchy};

    struct Empty;
    impl ClassHierarchy for Empty {
        fn is_instance(&self, class: &str, wanted: &str) -> Option<bool> {
            Some(class == wanted || wanted == "java/lang/Object")
        }
        fn is_interface(&self, _: &str) -> Option<bool> {
            None
        }
    }

    #[test]
    fn registrar_is_a_typed_payload_registrar() {
        let value = registrar(&Value::Ref(Some("1".into())));
        assert!(matches!(
            &value,
            Value::Host(handle)
                if handle.kind() == HostKind::NeoForgePayloadRegistrar
                    && handle.instance_of("net/neoforged/neoforge/network/registration/PayloadRegistrar", &Empty) == Some(true)
        ));
        let method = MethodRef {
            owner: "net/neoforged/neoforge/network/registration/PayloadRegistrar".into(),
            name: "versioned".into(),
            descriptor:
                "(Ljava/lang/String;)Lnet/neoforged/neoforge/network/registration/PayloadRegistrar;"
                    .into(),
        };
        assert!(instance_call(&method, &value, &[Value::Ref(Some("2".into()))]).is_none());
    }
}
