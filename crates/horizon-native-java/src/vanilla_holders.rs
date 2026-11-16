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

// These lists were read from the 26.3 static initialisers. Keeping the list
// explicit prevents an unrelated static field with a similar name from being
// treated as a registry entry.
const ATTRIBUTE_FIELDS: &[&str] = &[
    "AIR_DRAG_MODIFIER",
    "ARMOR",
    "ARMOR_TOUGHNESS",
    "ATTACK_DAMAGE",
    "ATTACK_KNOCKBACK",
    "ATTACK_SPEED",
    "BELOW_NAME_DISTANCE",
    "BLOCK_BREAK_SPEED",
    "BLOCK_INTERACTION_RANGE",
    "BOUNCINESS",
    "BURNING_TIME",
    "CAMERA_DISTANCE",
    "EXPLOSION_KNOCKBACK_RESISTANCE",
    "ENTITY_INTERACTION_RANGE",
    "FALL_DAMAGE_MULTIPLIER",
    "FLYING_SPEED",
    "FOLLOW_RANGE",
    "FRICTION_MODIFIER",
    "GRAVITY",
    "JUMP_STRENGTH",
    "KNOCKBACK_RESISTANCE",
    "LUCK",
    "MAX_ABSORPTION",
    "MAX_HEALTH",
    "MINING_EFFICIENCY",
    "MOVEMENT_EFFICIENCY",
    "MOVEMENT_SPEED",
    "NAME_TAG_DISTANCE",
    "OXYGEN_BONUS",
    "SAFE_FALL_DISTANCE",
    "SCALE",
    "SNEAKING_SPEED",
    "SPAWN_REINFORCEMENTS_CHANCE",
    "STEP_HEIGHT",
    "SUBMERGED_MINING_SPEED",
    "SWEEPING_DAMAGE_RATIO",
    "TEMPT_RANGE",
    "WATER_MOVEMENT_EFFICIENCY",
    "WAYPOINT_TRANSMIT_RANGE",
    "WAYPOINT_RECEIVE_RANGE",
];

const MOB_EFFECT_FIELDS: &[&str] = &[
    "SPEED",
    "SLOWNESS",
    "HASTE",
    "MINING_FATIGUE",
    "STRENGTH",
    "INSTANT_HEALTH",
    "INSTANT_DAMAGE",
    "JUMP_BOOST",
    "NAUSEA",
    "REGENERATION",
    "RESISTANCE",
    "FIRE_RESISTANCE",
    "WATER_BREATHING",
    "INVISIBILITY",
    "BLINDNESS",
    "NIGHT_VISION",
    "HUNGER",
    "WEAKNESS",
    "POISON",
    "WITHER",
    "HEALTH_BOOST",
    "ABSORPTION",
    "SATURATION",
    "GLOWING",
    "LEVITATION",
    "LUCK",
    "UNLUCK",
    "SLOW_FALLING",
    "CONDUIT_POWER",
    "DOLPHINS_GRACE",
    "BAD_OMEN",
    "HERO_OF_THE_VILLAGE",
    "DARKNESS",
    "TRIAL_OMEN",
    "RAID_OMEN",
    "WIND_CHARGED",
    "WEAVING",
    "OOZING",
    "INFESTED",
    "BREATH_OF_THE_NAUTILUS",
];

fn lower_id(field: &str) -> String {
    format!("minecraft:{}", field.to_ascii_lowercase())
}

fn known(field: &str, fields: &[&str]) -> Option<String> {
    fields.contains(&field).then(|| lower_id(field))
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
            known(&r.name, ATTRIBUTE_FIELDS)?,
        ),
        MOB_EFFECTS => (
            "minecraft:mob_effect",
            "mob-effect",
            known(&r.name, MOB_EFFECT_FIELDS)?,
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
    fn vanilla_attribute_holder_has_registry_facts_and_game_value() {
        let holder = static_field(&field(ATTRIBUTES, "ARMOR")).unwrap();
        assert_eq!(
            call("getRegisteredName", "()Ljava/lang/String;", &holder, &[]),
            Value::Ref(Some("minecraft:armor".into()))
        );
        assert_eq!(call("isBound", "()Z", &holder, &[]), Value::Int(1));
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
