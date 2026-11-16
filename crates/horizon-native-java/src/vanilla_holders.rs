//! Vanilla `Holder.Reference` constants whose registry id is fixed by the
//! 26.3 server classes (`Attributes` and `MobEffects`).
//!
//! The holder's key, registered name, and identity are pure registry facts.
//! Its value is a game-owned handle; methods on that value remain counted
//! until the game façade supplies the corresponding object state.

use crate::{MethodRef, Value};

const HOLDER: &str = "Lnet/minecraft/core/Holder;";
const ATTRIBUTES: &str = "net/minecraft/world/entity/ai/attributes/Attributes";
const MOB_EFFECTS: &str = "net/minecraft/world/effect/MobEffects";

fn known<'a>(field: &str, fields: &'a [(&'a str, &'a str, usize)]) -> Option<(&'a str, usize)> {
    fields
        .iter()
        .find(|(name, _, _)| *name == field)
        .map(|(_, id, index)| (*id, *index))
}

fn value(registry: &str, id: &str, family: &str) -> Value {
    // `GameRegistryObject` is canonical per registry entry. The client-style
    // key is already the game object's identity vocabulary and is also used by
    // the existing effect façade; no String can spell a holder or its value.
    let object = crate::game_object::client_value(format!("client/{family}/{id}"));
    crate::host_registry_holder::vanilla(registry, id, object)
}

/// Static fields whose type is `Holder` and whose ids are fixed by the 26.3
/// game registry bootstrap. Non-holder fields and unknown names fall through
/// to the normal counted static-field path.
pub(crate) fn static_field(r: &MethodRef) -> Option<Value> {
    if r.descriptor != HOLDER {
        return None;
    }
    let (registry, family, id) = match r.owner.as_str() {
        ATTRIBUTES => (
            "minecraft:attribute",
            "attribute",
            known(&r.name, crate::vanilla_holders_table::ATTRIBUTE_FIELDS)?.0,
        ),
        MOB_EFFECTS => (
            "minecraft:mob_effect",
            "mob-effect",
            known(&r.name, crate::vanilla_holders_table::MOB_EFFECT_FIELDS)?.0,
        ),
        _ => return None,
    };
    Some(value(registry, &id, family))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Host, MethodRef};

    fn field(owner: &str, name: &str) -> MethodRef {
        MethodRef {
            owner: owner.into(),
            name: name.into(),
            descriptor: HOLDER.into(),
        }
    }

    fn call(name: &str, descriptor: &str, receiver: &Value, args: &[Value]) -> Value {
        let mut host = crate::StdHost::new();
        host.invoke_instance(
            &MethodRef {
                owner: "net/minecraft/core/Holder".into(),
                name: name.into(),
                descriptor: descriptor.into(),
            },
            receiver,
            args,
        )
        .unwrap()
    }

    #[test]
    fn generated_tables_preserve_registry_order() {
        for (index, (_, _, recorded)) in crate::vanilla_holders_table::ATTRIBUTE_FIELDS
            .iter()
            .enumerate()
        {
            assert_eq!(index, *recorded);
        }
        for (index, (_, _, recorded)) in crate::vanilla_holders_table::MOB_EFFECT_FIELDS
            .iter()
            .enumerate()
        {
            assert_eq!(index, *recorded);
        }
    }

    #[test]
    fn vanilla_attribute_holder_has_registry_facts_and_game_value() {
        let holder = static_field(&field(ATTRIBUTES, "ARMOR")).unwrap();
        assert_eq!(
            call("getRegisteredName", "()Ljava/lang/String;", &holder, &[]),
            Value::Ref(Some("minecraft:armor".into()))
        );
        assert_eq!(call("isBound", "()Z", &holder, &[]), Value::Int(1));
        let spawn = static_field(&field(ATTRIBUTES, "SPAWN_REINFORCEMENTS_CHANCE")).unwrap();
        assert_eq!(
            call("getRegisteredName", "()Ljava/lang/String;", &spawn, &[]),
            Value::Ref(Some("minecraft:spawn_reinforcements".into()))
        );
        let value = call("value", "()Ljava/lang/Object;", &holder, &[]);
        assert!(matches!(
            value,
            Value::Host(ref handle)
                if handle.kind() == crate::HostKind::GameRegistryObject
                    && handle.key() == "client/attribute/minecraft:armor"
        ));
        assert_ne!(
            holder,
            Value::Ref(Some("client/effect-holder/minecraft:armor".into()))
        );
    }

    #[test]
    fn vanilla_effect_holder_maps_speed_and_rejects_forged_fields() {
        let holder = static_field(&field(MOB_EFFECTS, "SPEED")).unwrap();
        assert_eq!(
            call("getRegisteredName", "()Ljava/lang/String;", &holder, &[]),
            Value::Ref(Some("minecraft:speed".into()))
        );
        let value = call("value", "()Ljava/lang/Object;", &holder, &[]);
        assert!(matches!(
            value,
            Value::Host(ref handle)
                if handle.kind() == crate::HostKind::GameRegistryObject
                    && handle.key() == "client/mob-effect/minecraft:speed"
        ));
        assert!(static_field(&field(MOB_EFFECTS, "NOT_A_REAL_EFFECT")).is_none());
        assert!(static_field(&field(ATTRIBUTES, "DEFAULT_ATTACK_SPEED")).is_none());
        assert!(
            static_field(&MethodRef {
                owner: MOB_EFFECTS.into(),
                name: "SPEED".into(),
                descriptor: "Ljava/lang/String;".into(),
            })
            .is_none()
        );
    }
}
