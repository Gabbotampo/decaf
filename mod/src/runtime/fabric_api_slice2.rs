//! Fabric API slice 2: generic events and biome registrations.
//!
//! Fabric's array-backed events are host handles. Their listeners and phase
//! graph are recorded in registration order; the game adapter may inspect that
//! graph when it has a matching fire point. Biome modifications stay recorded:
//! applying them needs the game's real biome tables and is outside the native
//! entrypoint host.

use std::collections::BTreeMap;

use crate::runtime::{MethodRef, NativeError, Value, host_events, host_ids};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct EventState {
    phases: BTreeMap<String, Vec<usize>>,
    orderings: Vec<(String, String)>,
    next_registration: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BiomeRegistration {
    pub modification: String,
    pub phase: String,
    pub selector: String,
    pub action: String,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct State {
    events: BTreeMap<String, EventState>,
    pub(crate) biome_registrations: Vec<BiomeRegistration>,
    next_selector: usize,
    next_modification: usize,
}

fn text(value: &Value) -> Option<&str> {
    match value {
        Value::Ref(Some(value)) => Some(value),
        _ => None,
    }
}

fn id(value: &Value) -> String {
    host_ids::identifier_key(value)
        .or_else(|| text(value))
        .unwrap_or("fabric:unknown")
        .to_owned()
}

fn event_key(value: &Value) -> Option<String> {
    host_events::fabric_event_name(value)
}

fn selector_name(name: &str, serial: usize) -> Value {
    Value::Ref(Some(format!("fabric-biome-selector/{name}/{serial}")))
}

fn modification_name(identifier: &Value, serial: usize) -> Value {
    Value::Ref(Some(format!(
        "fabric-biome-modification/{}/{}",
        id(identifier),
        serial
    )))
}

fn value_name(value: &Value) -> String {
    host_ids::identifier_key(value)
        .or_else(|| text(value))
        .map_or_else(|| format!("{value:?}"), ToOwned::to_owned)
}

fn record_biome(
    state: &mut State,
    modification: impl Into<String>,
    phase: impl Into<String>,
    selector: impl Into<String>,
    action: impl Into<String>,
) {
    state.biome_registrations.push(BiomeRegistration {
        modification: modification.into(),
        phase: phase.into(),
        selector: selector.into(),
        action: action.into(),
    });
}

pub(crate) fn static_field(r: &MethodRef) -> Option<Value> {
    if r.owner == "net/fabricmc/fabric/api/biome/v1/ModificationPhase"
        && r.descriptor == "Lnet/fabricmc/fabric/api/biome/v1/ModificationPhase;"
        && matches!(
            r.name.as_str(),
            "ADDITIONS" | "REMOVALS" | "REPLACEMENTS" | "POST_PROCESSING"
        )
    {
        return Some(Value::Ref(Some(format!("fabric-biome-phase/{}", r.name))));
    }
    let id = match (r.owner.as_str(), r.name.as_str(), r.descriptor.as_str()) {
        (
            "net/fabricmc/fabric/api/message/v1/ServerMessageDecoratorEvent",
            "CONTENT_PHASE",
            "Lnet/minecraft/resources/Identifier;",
        ) => "fabric:content",
        (
            "net/fabricmc/fabric/api/message/v1/ServerMessageDecoratorEvent",
            "STYLING_PHASE",
            "Lnet/minecraft/resources/Identifier;",
        ) => "fabric:styling",
        (
            "net/fabricmc/fabric/api/recipe/v1/sync/RecipeSynchronization",
            "RECIPE_SYNC_EVENT_PHASE",
            "Lnet/minecraft/resources/Identifier;",
        ) => "fabric:recipe_sync",
        _ => {
            if r.owner == "net/fabricmc/fabric/api/transfer/v1/item/ItemStorage"
                && r.name == "SIDED"
                && r.descriptor == "Lnet/fabricmc/fabric/api/lookup/v1/block/BlockApiLookup;"
            {
                return Some(Value::Ref(Some(
                    "fabric-block-lookup/fabric:item_storage_sided".to_owned(),
                )));
            }
            if r.owner == "net/fabricmc/fabric/api/transfer/v1/fluid/FluidStorage"
                && r.name == "SIDED"
                && r.descriptor == "Lnet/fabricmc/fabric/api/lookup/v1/block/BlockApiLookup;"
            {
                return Some(Value::Ref(Some(
                    "fabric-block-lookup/fabric:fluid_storage_sided".to_owned(),
                )));
            }
            return None;
        }
    };
    Some(host_ids::static_identifier(id))
}

pub(crate) fn static_call(
    state: &mut State,
    r: &MethodRef,
    args: &[Value],
    next_event: &mut usize,
) -> Option<Result<Value, NativeError>> {
    let owner = r.owner.as_str();
    let name = r.name.as_str();

    if owner == "net/fabricmc/fabric/api/event/EventFactory"
        && matches!(name, "createArrayBacked" | "createWithPhases")
        && r.descriptor
            .ends_with("Lnet/fabricmc/fabric/api/event/Event;")
    {
        let event = host_events::fabric_array_event(*next_event);
        *next_event = (*next_event).saturating_add(1);
        if let Some(key) = event_key(&event) {
            state.events.entry(key).or_default();
        }
        return Some(Ok(event));
    }

    if owner == "net/fabricmc/fabric/api/biome/v1/BiomeSelectors" {
        let serial = state.next_selector;
        state.next_selector = state.next_selector.saturating_add(1);
        return Some(Ok(selector_name(name, serial)));
    }

    if owner == "net/fabricmc/fabric/api/attachment/v1/AttachmentSyncPredicate"
        && matches!(name, "all" | "targetOnly" | "allButTarget")
    {
        return Some(Ok(Value::Ref(Some(format!(
            "fabric-attachment-sync/{name}"
        )))));
    }

    if owner == "net/fabricmc/fabric/api/event/registry/DynamicRegistries"
        && matches!(name, "register" | "registerSynced")
    {
        return Some(Ok(Value::Void));
    }

    if owner == "net/fabricmc/fabric/api/recipe/v1/sync/RecipeSynchronization"
        && name == "synchronizeRecipeSerializer"
    {
        return Some(Ok(Value::Void));
    }

    if owner == "net/fabricmc/fabric/api/lookup/v1/block/BlockApiLookup" && name == "get" {
        let marker = args.first().map_or_else(
            || "fabric-block-lookup/unknown".to_owned(),
            |value| format!("fabric-block-lookup/{}", value_name(value)),
        );
        return Some(Ok(Value::Ref(Some(marker))));
    }

    if owner == "net/fabricmc/fabric/api/creativetab/v1/FabricCreativeModeTab" && name == "builder"
    {
        return Some(Ok(Value::Ref(Some(
            "fabric-creative-tab-builder".to_owned(),
        ))));
    }

    if owner == "net/fabricmc/fabric/api/biome/v1/BiomeModifications" {
        match name {
            "create"
                if r.descriptor
                    == "(Lnet/minecraft/resources/Identifier;)Lnet/fabricmc/fabric/api/biome/v1/BiomeModification;" =>
            {
                let marker = modification_name(
                    args.first().unwrap_or(&Value::Ref(None)),
                    state.next_modification,
                );
                state.next_modification = state.next_modification.saturating_add(1);
                return Some(Ok(marker));
            }
            "addFeature" | "addCarver" | "addSpawn" => {
                let selector = args
                    .first()
                    .map_or_else(|| "fabric-biome-selector/unknown".to_owned(), value_name);
                let action = args
                    .iter()
                    .skip(1)
                    .map(value_name)
                    .collect::<Vec<_>>()
                    .join("|");
                record_biome(
                    state,
                    format!("static:{name}"),
                    "ADDITIONS",
                    selector,
                    action,
                );
                return Some(Ok(Value::Void));
            }
            _ => {}
        }
    }

    None
}

pub(crate) fn instance_call(
    state: &mut State,
    r: &MethodRef,
    receiver: &Value,
    args: &[Value],
) -> Option<Result<Value, NativeError>> {
    if r.owner == "net/fabricmc/fabric/api/event/Event" {
        let Some(key) = event_key(receiver) else {
            return Some(Ok(Value::Void));
        };
        let event = state.events.entry(key).or_default();
        match r.name.as_str() {
            "register" => {
                let (phase, _listener) = if args.len() >= 2 {
                    (id(&args[0]), args.last())
                } else {
                    ("fabric:default".to_owned(), args.last())
                };
                let serial = event.next_registration;
                event.next_registration = event.next_registration.saturating_add(1);
                event.phases.entry(phase).or_default().push(serial);
                return Some(Ok(Value::Void));
            }
            "addPhaseOrdering" => {
                if let [first, second] = args {
                    event.orderings.push((id(first), id(second)));
                }
                return Some(Ok(Value::Void));
            }
            _ => {}
        }
    }

    if r.owner == "net/fabricmc/fabric/api/lookup/v1/block/BlockApiLookup"
        && r.name == "registerFallback"
        && matches!(receiver, Value::Ref(Some(marker)) if marker.starts_with("fabric-block-lookup/"))
    {
        return Some(Ok(Value::Void));
    }

    if r.owner == "net/fabricmc/fabric/api/biome/v1/BiomeModification"
        && r.name == "add"
        && matches!(receiver, Value::Ref(Some(marker)) if marker.starts_with("fabric-biome-modification/"))
    {
        let phase = args
            .first()
            .map_or_else(|| "fabric:unknown".to_owned(), value_name);
        let selector = args
            .get(1)
            .map_or_else(|| "fabric-biome-selector/unknown".to_owned(), value_name);
        let action = args
            .get(2)
            .map_or_else(|| "callback:unknown".to_owned(), value_name);
        let modification = text(receiver).unwrap_or("fabric-biome-modification/unknown");
        record_biome(state, modification, phase, selector, action);
        return Some(Ok(receiver.clone()));
    }

    None
}

pub(crate) fn event_order(state: &State, event: &Value) -> Vec<(String, usize)> {
    let Some(key) = event_key(event) else {
        return Vec::new();
    };
    let Some(event) = state.events.get(&key) else {
        return Vec::new();
    };
    let mut phases: Vec<String> = event.phases.keys().cloned().collect();
    let mut changed = true;
    while changed {
        changed = false;
        for (before, after) in &event.orderings {
            let (Some(i), Some(j)) = (
                phases.iter().position(|phase| phase == before),
                phases.iter().position(|phase| phase == after),
            ) else {
                continue;
            };
            if i > j {
                let phase = phases.remove(i);
                phases.insert(j, phase);
                changed = true;
            }
        }
    }
    phases
        .into_iter()
        .flat_map(|phase| {
            event
                .phases
                .get(&phase)
                .into_iter()
                .flat_map(move |listeners| {
                    let name = phase.clone();
                    listeners.iter().map(move |serial| (name.clone(), *serial))
                })
        })
        .collect()
}
