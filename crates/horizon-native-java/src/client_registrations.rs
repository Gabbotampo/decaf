//! Client registration-only APIs.
//!
//! These calls populate registries that the real client consumes later. The
//! native probe records their order and identities, but never invokes a
//! renderer, model, screen, particle provider, or other game-state callback.

use std::collections::{BTreeMap, BTreeSet};

use crate::{MethodRef, NativeError, Value};

const PARTICLE_REGISTRY: &str =
    "net/fabricmc/fabric/api/client/particle/v1/ParticleProviderRegistry";
const PARTICLE_TYPES: &str = "net/fabricmc/fabric/api/particle/v1/FabricParticleTypes";
const DEBUG_ENTRIES: &str = "net/minecraft/client/gui/components/debug/DebugScreenEntries";
const MENU_SCREENS: &str = "net/minecraft/client/gui/screens/MenuScreens";
const MODEL_LAYERS: &str = "net/fabricmc/fabric/api/client/rendering/v1/ModelLayerRegistry";
const ENTITY_RENDERERS: &str = "net/fabricmc/fabric/api/client/rendering/v1/EntityRendererRegistry";
const MODEL_LOADING: &str = "net/fabricmc/fabric/api/client/model/loading/v1/ModelLoadingPlugin";
const VANILLA_HUD: &str = "net/fabricmc/fabric/api/client/rendering/v1/hud/VanillaHudElements";
const DEBUG_PROFILE: &str = "net/minecraft/client/gui/components/debug/DebugScreenProfile";

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Registration {
    DebugEntry {
        id: String,
        entry: Value,
    },
    ParticleProvider {
        particle_type: Value,
        provider: Value,
    },
    MenuScreen {
        menu: Value,
        constructor: Value,
    },
    EntityRenderer {
        entity_type: Value,
        provider: Value,
    },
    ModelLayer {
        location: Value,
        provider: Value,
    },
    ModelLoadingPlugin {
        plugin: Value,
    },
}

#[derive(Clone, Debug, Default)]
pub(crate) struct State {
    pub(crate) registrations: Vec<Registration>,
    next_particle_type: u64,
    model_layers: BTreeSet<String>,
    armor_layers: BTreeSet<String>,
    menu_screens: BTreeSet<String>,
    static_values: BTreeMap<String, Value>,
}

fn client_handle(text: impl Into<String>) -> Value {
    crate::game_object::client_value(format!("client/{}", text.into()))
}

fn value_key(value: &Value) -> String {
    crate::expect_ref(value)
        .cloned()
        .unwrap_or_else(|| format!("{value:?}"))
}

fn identifier(value: &Value) -> Option<String> {
    crate::host_ids::identifier_key(value).map(str::to_owned)
}

fn illegal_state(message: impl Into<String>) -> NativeError {
    NativeError::UncaughtException {
        class: "java/lang/IllegalStateException".into(),
        message: Some(message.into()),
        frame: None,
    }
}

fn illegal_argument(message: impl Into<String>) -> NativeError {
    NativeError::UncaughtException {
        class: "java/lang/IllegalArgumentException".into(),
        message: Some(message.into()),
        frame: None,
    }
}

const DEBUG_IDS: &[&str] = &[
    "game_version",
    "fps",
    "tps",
    "memory",
    "detailed_memory",
    "system_specs",
    "looking_at_block_state",
    "looking_at_block_tags",
    "looking_at_fluid_state",
    "looking_at_fluid_tags",
    "looking_at_entity",
    "looking_at_entity_tags",
    "chunk_render_stats",
    "chunk_generation_stats",
    "entity_render_stats",
    "particle_render_stats",
    "chunk_source_stats",
    "player_position",
    "player_section_position",
    "player_speed",
    "light_levels",
    "heightmap",
    "biome",
    "local_difficulty",
    "day_count",
    "entity_spawn_counts",
    "sound_mood",
    "sound_cache",
    "post_effects",
    "entity_hitboxes",
    "chunk_borders",
    "three_dimensional_crosshair",
    "chunk_section_paths",
    "gpu_utilization",
    "simple_performance_impactors",
    "chunk_section_octree",
    "visualize_water_levels",
    "visualize_heightmap",
    "visualize_collision_boxes",
    "visualize_entity_supporting_blocks",
    "visualize_block_light_levels",
    "visualize_sky_light_levels",
    "visualize_solid_faces",
    "visualize_chunks_on_server",
    "visualize_sky_light_sections",
    "chunk_section_visibility",
];

const HUD_IDS: &[&str] = &[
    "misc_overlays",
    "crosshair",
    "spectator_menu",
    "hotbar",
    "armor_bar",
    "health_bar",
    "food_bar",
    "air_bar",
    "mount_health",
    "info_bar",
    "experience_level",
    "held_item_tooltip",
    "spectator_tooltip",
    "mob_effects",
    "boss_bar",
    "sleep",
    "demo_timer",
    "scoreboard",
    "overlay_message",
    "title_and_subtitle",
    "chat",
    "player_list",
    "subtitles",
];

/// Static fields whose values are canonical identifiers or enum constants.
pub(crate) fn static_field(state: &mut State, r: &MethodRef) -> Option<Value> {
    let cache_key = format!("{}#{}:{}", r.owner, r.name, r.descriptor);
    if let Some(value) = state.static_values.get(&cache_key) {
        return Some(value.clone());
    }
    let value = if r.owner == VANILLA_HUD
        && r.descriptor == "Lnet/minecraft/resources/Identifier;"
        && HUD_IDS.iter().any(|id| *id == r.name.to_ascii_lowercase())
    {
        Some(crate::host_ids::identifier_of_text(&format!(
            "minecraft:{}",
            r.name
        )))
    } else if r.owner == DEBUG_ENTRIES
        && r.descriptor == "Lnet/minecraft/resources/Identifier;"
        && DEBUG_IDS
            .iter()
            .any(|id| *id == r.name.to_ascii_lowercase())
    {
        let id = r.name.to_ascii_lowercase();
        Some(crate::host_ids::identifier_of_text(&format!(
            "minecraft:{id}"
        )))
    } else if r.owner == DEBUG_PROFILE
        && r.descriptor == "Lnet/minecraft/client/gui/components/debug/DebugScreenProfile;"
        && matches!(r.name.as_str(), "DEFAULT" | "PERFORMANCE")
    {
        Some(Value::Ref(Some(format!("enum/{DEBUG_PROFILE}/{}", r.name))))
    } else {
        None
    }?;
    state.static_values.insert(cache_key, value.clone());
    Some(value)
}

/// Static factories and registration-only static APIs.
pub(crate) fn invoke(
    state: &mut State,
    r: &MethodRef,
    args: &[Value],
) -> Option<Result<Value, NativeError>> {
    match (r.owner.as_str(), r.name.as_str()) {
        (PARTICLE_TYPES, "simple" | "complex") => {
            let key = state.next_particle_type;
            state.next_particle_type = state.next_particle_type.saturating_add(1);
            Some(Ok(client_handle(format!("particle-type/{key}"))))
        }
        (PARTICLE_REGISTRY, "getInstance") => Some(Ok(client_handle("particle-provider-registry"))),
        (PARTICLE_REGISTRY, "register") => {
            let particle_type = args.first().cloned().unwrap_or(Value::Ref(None));
            let provider = args.get(1).cloned().unwrap_or(Value::Ref(None));
            state.registrations.push(Registration::ParticleProvider {
                particle_type,
                provider,
            });
            Some(Ok(Value::Void))
        }
        (DEBUG_ENTRIES, "register") if args.len() == 2 => {
            let id = identifier(&args[0]).unwrap_or_else(|| value_key(&args[0]));
            state.registrations.push(Registration::DebugEntry {
                id: id.clone(),
                entry: args[1].clone(),
            });
            Some(Ok(args[0].clone()))
        }
        (MENU_SCREENS, "register") if args.len() == 2 => {
            let key = value_key(&args[0]);
            if !state.menu_screens.insert(key.clone()) {
                return Some(Err(illegal_state(format!("Duplicate registration for {key}"))));
            }
            state.registrations.push(Registration::MenuScreen {
                menu: args[0].clone(),
                constructor: args[1].clone(),
            });
            Some(Ok(Value::Void))
        }
        (ENTITY_RENDERERS, "register") if args.len() == 2 => {
            state.registrations.push(Registration::EntityRenderer {
                entity_type: args[0].clone(),
                provider: args[1].clone(),
            });
            Some(Ok(Value::Void))
        }
        (MODEL_LAYERS, "registerModelLayer") if args.len() == 2 => {
            if args[0] == Value::Ref(None) {
                return Some(Err(NativeError::UncaughtException {
                    class: "java/lang/NullPointerException".into(),
                    message: Some("ModelLayerLocation cannot be null".into()),
                    frame: None,
                }));
            }
            if args[1] == Value::Ref(None) {
                return Some(Err(NativeError::UncaughtException {
                    class: "java/lang/NullPointerException".into(),
                    message: Some("TexturedLayerDefinitionProvider cannot be null".into()),
                    frame: None,
                }));
            }
            let key = value_key(&args[0]);
            if !state.model_layers.insert(key.clone()) {
                return Some(Err(illegal_argument(format!(
                    "Cannot replace registration for model layer \"{key}\""
                ))));
            }
            state.registrations.push(Registration::ModelLayer {
                location: args[0].clone(),
                provider: args[1].clone(),
            });
            Some(Ok(Value::Void))
        }
        (MODEL_LAYERS, "registerArmorModelLayers") if args.len() == 2 => {
            if args[0] == Value::Ref(None) {
                return Some(Err(NativeError::UncaughtException {
                    class: "java/lang/NullPointerException".into(),
                    message: Some("ArmorModelSet cannot be null".into()),
                    frame: None,
                }));
            }
            if args[1] == Value::Ref(None) {
                return Some(Err(NativeError::UncaughtException {
                    class: "java/lang/NullPointerException".into(),
                    message: Some("TexturedArmorModelSetProvider cannot be null".into()),
                    frame: None,
                }));
            }
            let key = value_key(&args[0]);
            if !state.armor_layers.insert(key.clone()) {
                return Some(Err(illegal_argument(format!(
                    "Cannot replace registration for armor model layer \"{key}\""
                ))));
            }
            state.registrations.push(Registration::ModelLayer {
                location: args[0].clone(),
                provider: args[1].clone(),
            });
            Some(Ok(Value::Void))
        }
        (MODEL_LOADING, "register") if args.len() == 1 => {
            if args[0] == Value::Ref(None) {
                return Some(Err(NativeError::UncaughtException {
                    class: "java/lang/NullPointerException".into(),
                    message: Some("plugin must not be null".into()),
                    frame: None,
                }));
            }
            state.registrations.push(Registration::ModelLoadingPlugin {
                plugin: args[0].clone(),
            });
            Some(Ok(Value::Void))
        }
        _ => None,
    }
}

/// Instance calls on the particle registry singleton.
pub(crate) fn instance(
    state: &mut State,
    r: &MethodRef,
    receiver: &Value,
    args: &[Value],
) -> Option<Result<Value, NativeError>> {
    if r.owner != PARTICLE_REGISTRY || r.name != "register" || args.len() != 2 {
        return None;
    }
    if crate::game_object::handle_key(receiver) != Some("client/particle-provider-registry") {
        return None;
    }
    state.registrations.push(Registration::ParticleProvider {
        particle_type: args[0].clone(),
        provider: args[1].clone(),
    });
    Some(Ok(Value::Void))
}

pub(crate) fn summary(state: &State) -> serde_json::Value {
    let order = state
        .registrations
        .iter()
        .map(|registration| match registration {
            Registration::DebugEntry { id, .. } => format!("debug:{id}"),
            Registration::ParticleProvider { .. } => "particle_provider".to_owned(),
            Registration::MenuScreen { .. } => "menu_screen".to_owned(),
            Registration::EntityRenderer { .. } => "entity_renderer".to_owned(),
            Registration::ModelLayer { .. } => "model_layer".to_owned(),
            Registration::ModelLoadingPlugin { .. } => "model_loading_plugin".to_owned(),
        })
        .collect::<Vec<_>>();
    serde_json::json!({ "count": order.len(), "order": order })
}
