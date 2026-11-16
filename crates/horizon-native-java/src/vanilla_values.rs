//! Pure vanilla 26.3 value families used during mod bootstrap.

use crate::{
    MethodRef, NativeError, StdHost, Value, host_ids,
    host_ref::{ClassHierarchy, HostKind, HostRef, Methods},
};

pub(crate) const MOB_EFFECT_CATEGORY: &str = "net/minecraft/world/effect/MobEffectCategory";
pub(crate) const RANGED_ATTRIBUTE: &str =
    "net/minecraft/world/entity/ai/attributes/RangedAttribute";
const ATTRIBUTE: &str = "net/minecraft/world/entity/ai/attributes/Attribute";
const ENTITY_TYPE_TAGS: &str = "net/minecraft/tags/EntityTypeTags";
const TAG_KEY: &str = "net/minecraft/tags/TagKey";
const CHAT_FORMATTING: &str = "net/minecraft/ChatFormatting";
const ENTITY_TYPE_TAG_NAMES: &[&str] = &[
    "SKELETONS",
    "ZOMBIES",
    "RAIDERS",
    "UNDEAD",
    "BURN_IN_DAYLIGHT",
    "BEEHIVE_INHABITORS",
    "ARROWS",
    "IMPACT_PROJECTILES",
    "POWDER_SNOW_WALKABLE_MOBS",
    "AXOLOTL_ALWAYS_HOSTILES",
    "AXOLOTL_HUNT_TARGETS",
    "FREEZE_IMMUNE_ENTITY_TYPES",
    "FREEZE_HURTS_EXTRA_TYPES",
    "CAN_BREATHE_UNDER_WATER",
    "FROG_FOOD",
    "FALL_DAMAGE_IMMUNE",
    "DISMOUNTS_UNDERWATER",
    "NON_CONTROLLING_RIDER",
    "DEFLECTS_PROJECTILES",
    "CAN_TURN_IN_BOATS",
    "ILLAGER",
    "AQUATIC",
    "ARTHROPOD",
    "IGNORES_POISON_AND_REGEN",
    "INVERTED_HEALING_AND_HARM",
    "WITHER_FRIENDS",
    "ILLAGER_FRIENDS",
    "NOT_SCARY_FOR_PUFFERFISH",
    "SENSITIVE_TO_IMPALING",
    "SENSITIVE_TO_BANE_OF_ARTHROPODS",
    "SENSITIVE_TO_SMITE",
    "NO_ANGER_FROM_WIND_CHARGE",
    "IMMUNE_TO_OOZING",
    "IMMUNE_TO_INFESTED",
    "REDIRECTABLE_PROJECTILE",
    "BOAT",
    "CAN_EQUIP_SADDLE",
    "CAN_EQUIP_HARNESS",
    "CAN_WEAR_HORSE_ARMOR",
    "CAN_WEAR_NAUTILUS_ARMOR",
    "FOLLOWABLE_FRIENDLY_MOBS",
    "CANNOT_BE_PUSHED_ONTO_BOATS",
    "ACCEPTS_IRON_GOLEM_GIFT",
    "CANDIDATE_FOR_IRON_GOLEM_GIFT",
    "NAUTILUS_HOSTILES",
    "CAN_FLOAT_WHILE_RIDDEN",
    "CANNOT_BE_AGE_LOCKED",
    "NOT_AFFECTED_BY_GEYSERS",
    "CANNOT_BE_DISMOUNTED_BY_ITEM_USAGE",
];

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct RangedAttributeState {
    pub description_id: Option<String>,
    pub default_value: f64,
    pub min_value: f64,
    pub max_value: f64,
    pub syncable: bool,
}

fn enum_name(handle: &HostRef) -> &str {
    handle.key()
}

fn enum_ordinal(name: &str) -> Option<i32> {
    Some(match name {
        "BENEFICIAL" => 0,
        "HARMFUL" => 1,
        "NEUTRAL" => 2,
        _ => return None,
    })
}

fn enum_format(name: &str) -> Option<&'static str> {
    Some(match name {
        "BENEFICIAL" | "NEUTRAL" => "BLUE",
        "HARMFUL" => "RED",
        _ => return None,
    })
}

pub(crate) fn mob_effect_category(name: &str) -> Option<Value> {
    enum_ordinal(name).map(|_| Value::Host(HostRef::new(HostKind::MobEffectCategory, name)))
}

pub(crate) fn mob_effect_category_values() -> Vec<Value> {
    ["BENEFICIAL", "HARMFUL", "NEUTRAL"]
        .into_iter()
        .map(|name| mob_effect_category(name).unwrap())
        .collect()
}

fn enum_instance_of(_: &HostRef, wanted: &str, hierarchy: &dyn ClassHierarchy) -> Option<bool> {
    if matches!(
        wanted,
        MOB_EFFECT_CATEGORY | "java/lang/Enum" | "java/lang/Object" | "java/lang/Comparable"
    ) {
        return Some(true);
    }
    if matches!(wanted, CHAT_FORMATTING | TAG_KEY | ATTRIBUTE) {
        return Some(false);
    }
    hierarchy.is_instance(MOB_EFFECT_CATEGORY, wanted)
}

fn enum_compare(handle: &HostRef, other: &Value) -> Result<Option<i32>, NativeError> {
    let Some(other) = (match other {
        Value::Host(other) if other.kind() == HostKind::MobEffectCategory => Some(other),
        Value::Ref(None) => {
            return Err(NativeError::UncaughtException {
                class: "java/lang/NullPointerException".into(),
                message: None,
                frame: None,
            });
        }
        _ => None,
    }) else {
        return Ok(None);
    };
    Ok(Some(
        enum_ordinal(enum_name(handle)).unwrap() - enum_ordinal(enum_name(other)).unwrap(),
    ))
}

pub(crate) const MOB_EFFECT_CATEGORY_METHODS: Methods = Methods {
    to_string: |handle, _| Ok(Some(enum_name(handle).to_owned())),
    hash_code: |_, _| Ok(None),
    equals: |handle, other, _| {
        Ok(Some(
            matches!(other, Value::Host(other) if other.kind() == HostKind::MobEffectCategory && handle == other),
        ))
    },
    instance_of: enum_instance_of,
    crosses_jars: |_, _| true,
    compare_to: Some(|handle, other, _| enum_compare(handle, other)),
};

fn ranged_instance_of(_: &HostRef, wanted: &str, hierarchy: &dyn ClassHierarchy) -> Option<bool> {
    if matches!(wanted, RANGED_ATTRIBUTE | ATTRIBUTE | "java/lang/Object") {
        return Some(true);
    }
    if wanted == MOB_EFFECT_CATEGORY {
        return Some(false);
    }
    hierarchy.is_instance(RANGED_ATTRIBUTE, wanted)
}

pub(crate) const RANGED_ATTRIBUTE_METHODS: Methods = Methods {
    to_string: |_, _| Ok(None),
    hash_code: |_, _| Ok(None),
    equals: |handle, other, _| {
        Ok(match other {
            Value::Host(other) if other.kind() == HostKind::RangedAttribute => {
                handle.same_object(other)
            }
            _ => Some(false),
        })
    },
    instance_of: ranged_instance_of,
    crosses_jars: |_, _| false,
    compare_to: None,
};

pub(crate) fn static_field(r: &MethodRef) -> Option<Value> {
    if r.owner == ENTITY_TYPE_TAGS
        && r.descriptor == format!("L{};", TAG_KEY)
        && ENTITY_TYPE_TAG_NAMES.contains(&r.name.as_str())
    {
        let location = r.name.to_ascii_lowercase();
        return Some(host_ids::tag_key(
            &host_ids::registry_key_of("minecraft:entity_type"),
            &host_ids::identifier_of_text(&format!("minecraft:{location}")),
            Some(format!("minecraft:{location}")),
        ));
    }
    if r.owner == MOB_EFFECT_CATEGORY && r.descriptor == format!("L{};", MOB_EFFECT_CATEGORY) {
        return mob_effect_category(&r.name);
    }
    None
}

pub(crate) fn enum_values(r: &MethodRef) -> Option<Vec<Value>> {
    (r.owner == MOB_EFFECT_CATEGORY
        && r.name == "values"
        && r.descriptor == format!("()[L{};", MOB_EFFECT_CATEGORY))
    .then(mob_effect_category_values)
}

pub(crate) fn static_call(
    host: &mut StdHost,
    r: &MethodRef,
    args: &[Value],
) -> Option<Result<Value, NativeError>> {
    if r.owner != MOB_EFFECT_CATEGORY {
        return None;
    }
    match (r.name.as_str(), r.descriptor.as_str()) {
        ("valueOf", "(Ljava/lang/String;)Lnet/minecraft/world/effect/MobEffectCategory;") => {
            let name = args.first().and_then(crate::expect_ref)?;
            Some(
                mob_effect_category(name).ok_or_else(|| NativeError::UncaughtException {
                    class: "java/lang/IllegalArgumentException".into(),
                    message: Some(format!("No enum constant {MOB_EFFECT_CATEGORY}.{name}")),
                    frame: None,
                }),
            )
        }
        _ => Some(host.unknown_host_call(r)),
    }
}

pub(crate) fn instance_call(
    host: &mut StdHost,
    r: &MethodRef,
    receiver: &Value,
    args: &[Value],
) -> Option<Result<Value, NativeError>> {
    let Value::Host(handle) = receiver else {
        return None;
    };
    match handle.kind() {
        HostKind::MobEffectCategory => match (r.name.as_str(), r.descriptor.as_str()) {
            ("name", "()Ljava/lang/String;") => {
                Some(Ok(Value::Ref(Some(enum_name(handle).to_owned()))))
            }
            ("ordinal", "()I") => Some(Ok(Value::Int(enum_ordinal(enum_name(handle))?))),
            ("getTooltipFormatting", "()Lnet/minecraft/ChatFormatting;") => {
                Some(Ok(Value::Ref(Some(format!(
                    "enum/{CHAT_FORMATTING}/{}",
                    enum_format(enum_name(handle))?
                )))))
            }
            _ => None,
        },
        HostKind::RangedAttribute if ranged_method(r) => {
            Some(ranged_instance(host, r, handle, args))
        }
        _ => None,
    }
}

fn ranged_method(r: &MethodRef) -> bool {
    matches!(
        (r.name.as_str(), r.descriptor.as_str()),
        ("<init>", "(Ljava/lang/String;DDD)V")
            | ("getDefaultValue", "()D")
            | ("getMinValue", "()D")
            | ("getMaxValue", "()D")
            | ("getDescriptionId", "()Ljava/lang/String;")
            | ("isClientSyncable", "()Z")
            | (
                "setSyncable",
                "(Z)Lnet/minecraft/world/entity/ai/attributes/Attribute;"
            )
            | ("sanitizeValue", "(D)D")
    )
}

pub(crate) fn new_ranged_attribute(host: &mut StdHost) -> Value {
    let key = format!("ranged-attribute/{}", host.next_ranged_attribute);
    host.next_ranged_attribute = host.next_ranged_attribute.saturating_add(1);
    host.ranged_attributes.insert(
        key.clone(),
        RangedAttributeState {
            description_id: None,
            default_value: 0.0,
            min_value: 0.0,
            max_value: 0.0,
            syncable: false,
        },
    );
    Value::Host(HostRef::new(HostKind::RangedAttribute, key))
}

fn ranged_instance(
    host: &mut StdHost,
    r: &MethodRef,
    handle: &HostRef,
    args: &[Value],
) -> Result<Value, NativeError> {
    let key = handle.key().to_owned();
    let Some(state) = host.ranged_attributes.get_mut(&key) else {
        return host.unknown_host_call(r);
    };
    match (r.name.as_str(), r.descriptor.as_str()) {
        ("<init>", "(Ljava/lang/String;DDD)V") => {
            let id = match StdHost::argument(r, args, 0)? {
                Value::Ref(Some(id)) => Some(id.clone()),
                Value::Ref(None) => None,
                other => Some(crate::expect_string(other).to_owned()),
            };
            let default_value = crate::expect_float(r, StdHost::argument(r, args, 1)?)?;
            let min_value = crate::expect_float(r, StdHost::argument(r, args, 2)?)?;
            let max_value = crate::expect_float(r, StdHost::argument(r, args, 3)?)?;
            if min_value > max_value {
                return Err(illegal(
                    "Minimum value cannot be bigger than maximum value!",
                ));
            }
            if default_value < min_value {
                return Err(illegal("Default value cannot be lower than minimum value!"));
            }
            if default_value > max_value {
                return Err(illegal(
                    "Default value cannot be bigger than maximum value!",
                ));
            }
            state.description_id = id;
            state.default_value = default_value;
            state.min_value = min_value;
            state.max_value = max_value;
            Ok(Value::Void)
        }
        ("getDefaultValue", "()D") => Ok(Value::Float(state.default_value)),
        ("getMinValue", "()D") => Ok(Value::Float(state.min_value)),
        ("getMaxValue", "()D") => Ok(Value::Float(state.max_value)),
        ("getDescriptionId", "()Ljava/lang/String;") => {
            Ok(Value::Ref(state.description_id.clone()))
        }
        ("isClientSyncable", "()Z") => Ok(Value::Int(state.syncable as i32)),
        ("setSyncable", "(Z)Lnet/minecraft/world/entity/ai/attributes/Attribute;") => {
            state.syncable = crate::expect_int(r, StdHost::argument(r, args, 0)?)? != 0;
            Ok(Value::Host(handle.clone()))
        }
        ("sanitizeValue", "(D)D") => {
            let value = crate::expect_float(r, StdHost::argument(r, args, 0)?)?;
            Ok(Value::Float(if value.is_nan() || value < state.min_value {
                state.min_value
            } else {
                crate::java_float_min(value, state.max_value)
            }))
        }
        _ => host.unknown_host_call(r),
    }
}

fn illegal(message: &str) -> NativeError {
    NativeError::UncaughtException {
        class: "java/lang/IllegalArgumentException".into(),
        message: Some(message.into()),
        frame: None,
    }
}

pub(crate) fn random_new(host: &mut StdHost, class: &str) -> Option<Value> {
    (class == RANGED_ATTRIBUTE).then(|| new_ranged_attribute(host))
}
