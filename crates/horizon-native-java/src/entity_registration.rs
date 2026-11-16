//! Vanilla entity registration and synchronized-data handles.
use crate::jar_object;
use crate::{MethodRef, NativeError, Value, expect_float, expect_int, expect_ref};
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
struct Builder {
    category: String,
    width: f64,
    height: f64,
    fire_immune: bool,
    summon: bool,
    serialize: bool,
    far_from_player: bool,
    peaceful: bool,
    track_deltas: bool,
    eye_height: f64,
    tracking: i32,
    interval: i32,
}
#[derive(Debug, Clone)]
struct Entity {
    category: String,
    width: f64,
    height: f64,
    fire_immune: bool,
    summon: bool,
    serialize: bool,
    far_from_player: bool,
    peaceful: bool,
    track_deltas: bool,
    eye_height: f64,
    tracking: i32,
    interval: i32,
    description: String,
}

#[derive(Debug, Default, Clone)]
pub(crate) struct State {
    next: u64,
    builders: BTreeMap<String, Builder>,
    entities: BTreeMap<String, Entity>,
    dimensions: BTreeMap<String, (f64, f64, f64)>,
    ids: BTreeMap<String, u16>,
}

impl State {
    pub(crate) fn entry_count(&self) -> usize {
        self.builders.len() + self.entities.len() + self.dimensions.len()
    }

    pub(crate) fn retain_keys(&mut self, keep: impl Fn(&str) -> bool) {
        self.builders.retain(|key, _| keep(key));
        self.entities.retain(|key, _| keep(key));
        self.dimensions.retain(|key, _| keep(key));
    }
}

const BUILDER: &str = "net/minecraft/world/entity/EntityType$Builder";
const ENTITY: &str = "net/minecraft/world/entity/EntityType";
const ACCESSOR: &str = "net/minecraft/network/syncher/EntityDataAccessor";
const SERIALIZER: &str = "net/minecraft/network/syncher/EntityDataSerializer";

fn handle(state: &mut State, class: &str) -> Value {
    state.next += 1;
    jar_object::handle(format!("{class}#{}", 700_000 + state.next))
}
fn class(value: &Value) -> Option<&str> {
    jar_object::value_marker_class(value)
}
fn key(value: &Value) -> Option<String> {
    jar_object::instance_key(value).map(str::to_owned)
}
fn arg_string(v: &Value) -> Option<String> {
    crate::host_ids::identifier_key(v)
        .map(str::to_owned)
        .or_else(|| expect_ref(v).cloned())
}
pub(crate) fn category_constant(name: &str) -> Value {
    let names = [
        "MONSTER",
        "CREATURE",
        "AMBIENT",
        "AXOLOTLS",
        "UNDERGROUND_WATER_CREATURE",
        "WATER_CREATURE",
        "WATER_AMBIENT",
        "MISC",
    ];
    let id = names.iter().position(|n| *n == name).unwrap_or(0);
    jar_object::handle(format!(
        "net/minecraft/world/entity/MobCategory#{}",
        710_000 + id
    ))
}
fn category_name(value: &Value) -> Option<&'static str> {
    let id: usize = jar_object::instance_key(value)?
        .rsplit_once('#')?
        .1
        .parse::<usize>()
        .ok()?
        .checked_sub(710_000)?;
    [
        "MONSTER",
        "CREATURE",
        "AMBIENT",
        "AXOLOTLS",
        "UNDERGROUND_WATER_CREATURE",
        "WATER_CREATURE",
        "WATER_AMBIENT",
        "MISC",
    ]
    .get(id)
    .copied()
}
fn serializer_handle(name: &str) -> Value {
    let names = [
        "BYTE",
        "INT",
        "LONG",
        "FLOAT",
        "STRING",
        "COMPONENT",
        "OPTIONAL_COMPONENT",
        "ITEM_STACK",
        "BLOCK_STATE",
        "OPTIONAL_BLOCK_STATE",
        "BOOLEAN",
        "PARTICLE",
        "PARTICLES",
        "ROTATIONS",
        "BLOCK_POS",
        "OPTIONAL_BLOCK_POS",
        "DIRECTION",
        "OPTIONAL_LIVING_ENTITY_REFERENCE",
        "POSE",
        "CAT_VARIANT",
        "FROG_VARIANT",
        "WOLF_VARIANT",
        "WOLF_SOUND_VARIANT",
        "PIG_VARIANT",
        "PIG_SOUND_VARIANT",
        "COW_VARIANT",
        "COW_SOUND_VARIANT",
        "CHICKEN_VARIANT",
        "CHICKEN_SOUND_VARIANT",
        "ARMADILLO_STATE",
        "SNIFFER_STATE",
        "WEATHERING_COPPER_STATE",
        "COPPER_GOLEM_STATE",
        "VECTOR3",
        "QUATERNION",
        "RESOLVABLE_PROFILE",
        "HUMANOID_ARM",
        "DYE_COLOR",
        "VILLAGER_DATA",
        "OPTIONAL_UNSIGNED_INT",
    ];
    let id = names.iter().position(|n| *n == name).unwrap_or(0);
    jar_object::handle(format!("{SERIALIZER}#{}", 720_000 + id))
}

fn category(v: &Value) -> Option<String> {
    category_name(v).map(str::to_owned).or_else(|| {
        expect_ref(v)
            .and_then(|s| s.strip_prefix("enum/net/minecraft/world/entity/MobCategory/"))
            .map(str::to_owned)
    })
}

pub(crate) fn static_field(
    _state: &mut State,
    r: &MethodRef,
) -> Option<Result<Value, NativeError>> {
    if r.owner == "net/minecraft/world/entity/MobCategory"
        && r.descriptor == "Lnet/minecraft/world/entity/MobCategory;"
    {
        let names = [
            "MONSTER",
            "CREATURE",
            "AMBIENT",
            "AXOLOTLS",
            "UNDERGROUND_WATER_CREATURE",
            "WATER_CREATURE",
            "WATER_AMBIENT",
            "MISC",
        ];
        if names.contains(&r.name.as_str()) {
            return Some(Ok(category_constant(&r.name)));
        }
    }
    if r.owner == "net/minecraft/network/syncher/EntityDataSerializers"
        && r.descriptor == "Lnet/minecraft/network/syncher/EntityDataSerializer;"
    {
        let names = [
            "BYTE",
            "INT",
            "LONG",
            "FLOAT",
            "STRING",
            "COMPONENT",
            "OPTIONAL_COMPONENT",
            "ITEM_STACK",
            "BLOCK_STATE",
            "OPTIONAL_BLOCK_STATE",
            "OPTIONAL_GLOBAL_POS",
            "BOOLEAN",
            "PARTICLE",
            "PARTICLES",
            "ROTATIONS",
            "BLOCK_POS",
            "OPTIONAL_BLOCK_POS",
            "DIRECTION",
            "OPTIONAL_LIVING_ENTITY_REFERENCE",
            "POSE",
            "CAT_VARIANT",
            "CAT_SOUND_VARIANT",
            "FROG_VARIANT",
            "WOLF_VARIANT",
            "WOLF_SOUND_VARIANT",
            "PIG_VARIANT",
            "PIG_SOUND_VARIANT",
            "COW_VARIANT",
            "COW_SOUND_VARIANT",
            "CHICKEN_VARIANT",
            "CHICKEN_SOUND_VARIANT",
            "ARMADILLO_STATE",
            "SNIFFER_STATE",
            "WEATHERING_COPPER_STATE",
            "COPPER_GOLEM_STATE",
            "VECTOR3",
            "QUATERNION",
            "RESOLVABLE_PROFILE",
            "HUMANOID_ARM",
            "DYE_COLOR",
            "VILLAGER_DATA",
            "OPTIONAL_UNSIGNED_INT",
        ];
        if names.iter().any(|n| *n == r.name) {
            return Some(Ok(serializer_handle(&r.name)));
        }
    }
    None
}

pub(crate) fn static_call(
    state: &mut State,
    r: &MethodRef,
    args: &[Value],
) -> Option<Result<Value, NativeError>> {
    if r.owner == "net/minecraft/network/syncher/SynchedEntityData" && r.name == "defineId" {
        let class_name = args.first().and_then(crate::host_class::class_name)?;
        return Some(define_id(state, class_name));
    }
    if r.owner == BUILDER && (r.name == "of" || r.name == "createNothing") {
        let cat = args.get(1).or_else(|| args.first()).and_then(category)?;
        let far_from_player = matches!(cat.as_str(), "CREATURE" | "MISC");
        let v = handle(state, BUILDER);
        let k = key(&v).unwrap();
        state.builders.insert(
            k,
            Builder {
                category: cat,
                width: 0.6,
                height: 1.8,
                fire_immune: false,
                summon: true,
                serialize: true,
                far_from_player,
                peaceful: true,
                track_deltas: true,
                eye_height: 1.53,
                tracking: 5,
                interval: 3,
            },
        );
        return Some(Ok(v));
    }
    None
}

pub(crate) fn instance_call(
    state: &mut State,
    r: &MethodRef,
    receiver: &Value,
    args: &[Value],
) -> Option<Result<Value, NativeError>> {
    let marker = class(receiver)?;
    let k = key(receiver)?;
    if marker == ACCESSOR && r.name == "id" && r.descriptor == "()I" {
        return Some(Ok(Value::Int(accessor_id(receiver)?)));
    }
    if marker == BUILDER {
        if r.name == "build" {
            let mut b = state.builders.remove(&k)?;
            let id = args
                .first()
                .and_then(crate::host_ids::key_location)
                .and_then(|v| crate::host_ids::identifier_key(&v).map(str::to_owned))
                .or_else(|| args.first().and_then(arg_string))?;
            let v = handle(state, ENTITY);
            let ek = key(&v).unwrap();
            b.width = b.width.max(0.0);
            b.height = b.height.max(0.0);
            state.entities.insert(
                ek,
                Entity {
                    category: b.category,
                    width: b.width,
                    height: b.height,
                    fire_immune: b.fire_immune,
                    summon: b.summon,
                    serialize: b.serialize,
                    far_from_player: b.far_from_player,
                    peaceful: b.peaceful,
                    track_deltas: b.track_deltas,
                    eye_height: b.eye_height,
                    tracking: b.tracking,
                    interval: b.interval,
                    description: format!("entity.{}", id.replace(':', ".")),
                },
            );
            return Some(Ok(v));
        }
        let b = state.builders.get_mut(&k)?;
        match r.name.as_str() {
            "sized" if args.len() >= 2 => {
                b.width = expect_float(r, &args[0]).ok()?;
                b.height = expect_float(r, &args[1]).ok()?;
                b.eye_height = b.height * 0.85;
            }
            "clientTrackingRange" => b.tracking = expect_int(r, args.first()?).ok()?,
            "updateInterval" => b.interval = expect_int(r, args.first()?).ok()?,
            "fireImmune" => b.fire_immune = true,
            "noSummon" => b.summon = false,
            "noSave" => b.serialize = false,
            "canSpawnFarFromPlayer" => b.far_from_player = true,
            "notInPeaceful" => b.peaceful = false,
            "dontTrackDeltas" => b.track_deltas = false,
            "eyeHeight" => b.eye_height = expect_float(r, args.first()?).ok()?,
            "noUpdateInterval" => b.interval = i32::MAX,
            _ => return None,
        }
        return Some(Ok(receiver.clone()));
    }
    if marker == "net/minecraft/world/entity/EntityDimensions" {
        let (width, height, eye_height) = state.dimensions.get(&k)?;
        return Some(Ok(match r.name.as_str() {
            "width" => Value::Float(*width),
            "height" => Value::Float(*height),
            "eyeHeight" => Value::Float(*eye_height),
            _ => return None,
        }));
    }
    if marker == "net/minecraft/world/entity/MobCategory" {
        let name = category_name(receiver)?;
        let (max, friendly, persistent, despawn) = match name {
            "MONSTER" => (70, false, false, 128),
            "CREATURE" => (10, true, true, 128),
            "AMBIENT" => (15, true, false, 128),
            "AXOLOTLS" => (5, true, false, 128),
            "UNDERGROUND_WATER_CREATURE" => (5, true, false, 128),
            "WATER_CREATURE" => (5, true, false, 128),
            "WATER_AMBIENT" => (20, true, false, 64),
            "MISC" => (-1, true, true, 128),
            _ => return None,
        };
        return Some(Ok(match r.name.as_str() {
            "getName" | "getSerializedName" => Value::Ref(Some(name.to_ascii_lowercase())),
            "toString" => Value::Ref(Some(name.to_owned())),
            "getDebugAbbreviation" => Value::Ref(Some(
                match name {
                    "MONSTER" => "MO",
                    "CREATURE" => "C",
                    "AMBIENT" => "AM",
                    "AXOLOTLS" => "AX",
                    "UNDERGROUND_WATER_CREATURE" => "UWC",
                    "WATER_CREATURE" => "WC",
                    "WATER_AMBIENT" => "WA",
                    _ => "MI",
                }
                .into(),
            )),
            "getMaxInstancesPerChunk" => Value::Int(max),
            "isFriendly" => Value::Int(friendly as i32),
            "isPersistent" => Value::Int(persistent as i32),
            "getDespawnDistance" => Value::Int(despawn),
            "getNoDespawnDistance" => Value::Int(32),
            _ => return None,
        }));
    }
    if marker == ENTITY {
        let e = state.entities.get(&k)?;
        let (entity_width, entity_height, entity_eye_height) = (e.width, e.height, e.eye_height);
        let out = match (r.name.as_str(), r.descriptor.as_str()) {
            ("getCategory", _) => category_constant(&e.category),
            ("canSerialize", "()Z") => Value::Int(e.serialize as i32),
            ("canSpawnFarFromPlayer", "()Z") => Value::Int(e.far_from_player as i32),
            ("isAllowedInPeaceful", "()Z") => Value::Int(e.peaceful as i32),
            ("trackDeltas", "()Z") => Value::Int(e.track_deltas as i32),
            ("clientTrackingRange", "()I") => Value::Int(e.tracking),
            ("updateInterval", "()I") => Value::Int(e.interval),
            ("fireImmune", "()Z") => Value::Int(e.fire_immune as i32),
            ("canSummon", "()Z") => Value::Int(e.summon as i32),
            ("getDescriptionId", "()Ljava/lang/String;") | ("toString", "()Ljava/lang/String;") => {
                Value::Ref(Some(e.description.clone()))
            }
            ("getDimensions", _) => {
                let v = handle(state, "net/minecraft/world/entity/EntityDimensions");
                let dk = key(&v).unwrap();
                state
                    .dimensions
                    .insert(dk, (entity_width, entity_height, entity_eye_height));
                return Some(Ok(v));
            }
            ("getWidth", "()F") => Value::Float(e.width),
            ("getHeight", "()F") => Value::Float(e.height),
            _ => return None,
        };
        return Some(Ok(out));
    }
    None
}

pub(crate) fn define_id(state: &mut State, class_name: &str) -> Result<Value, NativeError> {
    let next = state.ids.entry(class_name.to_owned()).or_insert_with(|| {
        if class_name == "net/minecraft/world/entity/Entity" {
            0
        } else if class_name == "net/minecraft/world/entity/LivingEntity" {
            7
        } else if class_name == "net/minecraft/world/entity/Mob" {
            16
        } else if class_name.ends_with("Entity") {
            7
        } else {
            0
        }
    });
    if *next > 254 {
        return Err(NativeError::UncaughtException {
            class: "java/lang/IllegalArgumentException".into(),
            message: Some(format!(
                "Data value id is too big with {}! (Max is 254)",
                *next
            )),
            frame: None,
        });
    }
    let id = *next;
    *next += 1;
    Ok(jar_object::handle(format!("{ACCESSOR}#{id}")))
}

pub(crate) fn accessor_id(value: &Value) -> Option<i32> {
    (class(value) == Some(ACCESSOR))
        .then(|| {
            jar_object::instance_key(value)?
                .rsplit_once('#')?
                .1
                .parse()
                .ok()
        })
        .flatten()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn builder_and_accessor_surface_keeps_vanilla_defaults() {
        let mut state = State::default();
        let category = Value::Ref(Some(
            "enum/net/minecraft/world/entity/MobCategory/CREATURE".into(),
        ));
        let b = static_call(&mut state, &MethodRef { owner: BUILDER.into(), name: "of".into(), descriptor: "(Lnet/minecraft/world/entity/EntityType$EntityFactory;Lnet/minecraft/world/entity/MobCategory;)Lnet/minecraft/world/entity/EntityType$Builder;".into() }, &[Value::Ref(Some("factory".into())), category]).unwrap().unwrap();
        instance_call(
            &mut state,
            &MethodRef {
                owner: BUILDER.into(),
                name: "sized".into(),
                descriptor: "(FF)Lnet/minecraft/world/entity/EntityType$Builder;".into(),
            },
            &b,
            &[Value::Float(1.25), Value::Float(2.5)],
        )
        .unwrap()
        .unwrap();
        let id = define_id(&mut state, "demo/ProbeEntity");
        assert_eq!(accessor_id(&id.unwrap()), Some(7));
    }
}
