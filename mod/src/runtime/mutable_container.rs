//! Exact vanilla 26.3 `SimpleMutableContainer` and `GrowableMutableContainer` model.
//!
//! `SimpleMutableContainer` declares `protected final List<ItemStack> items;` and stores
//! the caller's list in its `<init>(List)` constructor. Subclasses in mods inherit `items`,
//! `size()`, `setItem(int, ItemStack)`, and `GrowableMutableContainer.addSlotWithItem(ItemProvider)`.

use crate::runtime::{Host, MethodRef, NativeError, Runtime, Value, instance_field_key};

pub(crate) const SIMPLE_MUTABLE_CONTAINER: &str =
    "net/minecraft/world/item/component/SimpleMutableContainer";
pub(crate) const GROWABLE_MUTABLE_CONTAINER: &str =
    "net/minecraft/world/item/component/GrowableMutableContainer";

fn items_field_ref() -> MethodRef {
    MethodRef {
        owner: SIMPLE_MUTABLE_CONTAINER.into(),
        name: "items".into(),
        descriptor: "Ljava/util/List;".into(),
    }
}

pub(crate) fn field_owner(class: &str, name: &str, descriptor: &str) -> Option<&'static str> {
    if matches!(class, SIMPLE_MUTABLE_CONTAINER | GROWABLE_MUTABLE_CONTAINER)
        && name == "items"
        && descriptor == "Ljava/util/List;"
    {
        Some(SIMPLE_MUTABLE_CONTAINER)
    } else {
        None
    }
}

pub(crate) fn is_final_field(field: &MethodRef) -> bool {
    field.owner == SIMPLE_MUTABLE_CONTAINER
        && field.name == "items"
        && field.descriptor == "Ljava/util/List;"
}

fn receiver_extends_container<H: Host>(runtime: &Runtime<'_, H>, receiver: &Value) -> bool {
    let Some(class) = crate::runtime::jar_object::object_class(receiver) else {
        return false;
    };
    class == SIMPLE_MUTABLE_CONTAINER
        || class == GROWABLE_MUTABLE_CONTAINER
        || runtime.jar_extends(class, SIMPLE_MUTABLE_CONTAINER)
        || runtime.jar_extends(class, GROWABLE_MUTABLE_CONTAINER)
}

pub(crate) fn runtime_call<H: Host>(
    runtime: &mut Runtime<'_, H>,
    r: &MethodRef,
    op: u8,
    receiver: Option<&Value>,
    args: &[Value],
) -> Result<Option<Value>, NativeError> {
    let Some(receiver) = receiver else {
        return Ok(None);
    };
    // `super(items)` from a jar subclass into `SimpleMutableContainer` / `GrowableMutableContainer`.
    if op == 0xb7
        && r.name == "<init>"
        && r.descriptor == "(Ljava/util/List;)V"
        && matches!(
            r.owner.as_str(),
            SIMPLE_MUTABLE_CONTAINER | GROWABLE_MUTABLE_CONTAINER
        )
        && runtime.class(&r.owner).is_none()
    {
        let Some(items) = args.first().cloned() else {
            return Ok(None);
        };
        let field = items_field_ref();
        if let Some(key) = instance_field_key(receiver, &field) {
            runtime.fields.insert(key, items);
            return Ok(Some(Value::Void));
        }
        return Ok(None);
    }

    // Inherited methods when called on a subclass instance that does not override them
    // (or invoked explicitly via `super.*`).
    let target_is_container = matches!(
        r.owner.as_str(),
        SIMPLE_MUTABLE_CONTAINER | GROWABLE_MUTABLE_CONTAINER
    ) || (op != 0xb7
        && receiver_extends_container(runtime, receiver)
        && runtime
            .jar_method_owner(&r.owner, &r.name, &r.descriptor)
            .is_none());
    if !target_is_container || runtime.class(&r.owner).is_some() && op == 0xb7 {
        return Ok(None);
    }

    match (r.name.as_str(), r.descriptor.as_str(), args) {
        ("size", "()I", []) => {
            let field = items_field_ref();
            let Some(items) = instance_field_key(receiver, &field)
                .and_then(|key| runtime.fields.get(&key).cloned())
            else {
                return Ok(None);
            };
            runtime
                .call_virtual(
                    crate::runtime::jdk_call::Site::Interface,
                    "java/util/List",
                    "size",
                    "()I",
                    &items,
                    &[],
                )
                .map(Some)
        }
        ("setItem", "(ILnet/minecraft/world/item/ItemStack;)Z", [index, stack]) => {
            let field = items_field_ref();
            let Some(items) = instance_field_key(receiver, &field)
                .and_then(|key| runtime.fields.get(&key).cloned())
            else {
                return Ok(None);
            };
            runtime.call_virtual(
                crate::runtime::jdk_call::Site::Interface,
                "java/util/List",
                "set",
                "(ILjava/lang/Object;)Ljava/lang/Object;",
                &items,
                &[index.clone(), stack.clone()],
            )?;
            Ok(Some(Value::Int(1)))
        }
        ("addSlotWithItem", "(Lnet/minecraft/world/item/ItemProvider;)Z", [provider]) => {
            let field = items_field_ref();
            let Some(items) = instance_field_key(receiver, &field)
                .and_then(|key| runtime.fields.get(&key).cloned())
            else {
                return Ok(None);
            };
            let next = runtime.call_virtual(
                crate::runtime::jdk_call::Site::Interface,
                "net/minecraft/world/item/ItemProvider",
                "next",
                "()Lnet/minecraft/world/item/ItemStack;",
                provider,
                &[],
            )?;
            runtime.call_virtual(
                crate::runtime::jdk_call::Site::Interface,
                "java/util/List",
                "add",
                "(Ljava/lang/Object;)Z",
                &items,
                &[next],
            )?;
            Ok(Some(Value::Int(1)))
        }
        _ => Ok(None),
    }
}
