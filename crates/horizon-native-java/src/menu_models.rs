//! Small, exact vanilla menu/container surface used by mod-owned menus.
//!
//! This is deliberately structural.  A recipe lookup needs the game's live
//! recipe index and therefore remains on the counted host path.

use crate::{MethodRef, NativeError, Value, expect_ref};
use std::collections::BTreeMap;

/// Runes' exact `ItemCombinerMenuSlotDefinition` layout from the 26.3 class.
#[allow(dead_code)]
pub(crate) const RUNE_SLOT_LAYOUT: [(i32, i32, i32); 3] = [(0, 27, 47), (1, 76, 47), (2, 134, 47)];
pub(crate) const RUNE_MENU_SLOT_COUNT: i32 = 39;

#[derive(Clone, Debug)]
pub(crate) struct MenuState {
    pub input: String,
    pub result: String,
    pub slots: String,
    pub input_items: [Value; 2],
    pub result_item: Value,
}

pub(crate) fn is_menu_class(owner: &str) -> bool {
    matches!(
        owner,
        "net/minecraft/world/inventory/AbstractContainerMenu"
            | "net/minecraft/world/inventory/ItemCombinerMenu"
            | "net/minecraft/world/SimpleContainer"
            | "net/minecraft/world/inventory/ResultContainer"
            | "net/minecraft/world/Container"
            | "net/minecraft/world/inventory/Slot"
            | "net/minecraft/world/inventory/ContainerLevelAccess"
    )
}

pub(crate) fn new_state(key: &str) -> MenuState {
    MenuState {
        input: format!("menu/{key}/input"),
        result: format!("menu/{key}/result"),
        slots: format!("menu/{key}/slots"),
        input_items: [
            Value::Ref(Some("stack/empty".into())),
            Value::Ref(Some("stack/empty".into())),
        ],
        result_item: Value::Ref(Some("stack/empty".into())),
    }
}

pub(crate) fn field(state: &MenuState, r: &MethodRef) -> Option<Value> {
    match (r.owner.as_str(), r.name.as_str(), r.descriptor.as_str()) {
        ("net/minecraft/world/inventory/ItemCombinerMenu", "inputSlots", _) => {
            Some(Value::Ref(Some(state.input.clone())))
        }
        ("net/minecraft/world/inventory/ItemCombinerMenu", "resultSlots", _) => {
            Some(Value::Ref(Some(state.result.clone())))
        }
        ("net/minecraft/world/inventory/AbstractContainerMenu", "slots", _) => {
            Some(Value::Ref(Some(state.slots.clone())))
        }
        _ => None,
    }
}

pub(crate) fn call(
    states: &mut BTreeMap<String, MenuState>,
    r: &MethodRef,
    receiver: &Value,
    args: &[Value],
) -> Option<Result<Value, NativeError>> {
    let key = expect_ref(receiver)?.to_owned();
    if r.owner == "net/minecraft/world/item/ItemStack" {
        return Some(match (r.name.as_str(), r.descriptor.as_str()) {
            ("isEmpty", "()Z") => Ok(Value::Int(
                (key == "stack/empty" || key.ends_with("@0")) as i32,
            )),
            ("getCount", "()I") => Ok(Value::Int(
                key.rsplit_once('@')
                    .and_then(|(_, n)| n.parse().ok())
                    .unwrap_or(1),
            )),
            ("copy", "()Lnet/minecraft/world/item/ItemStack;") => Ok(receiver.clone()),
            ("shrink", "(I)V") | ("onCraftedBy", _) => Ok(Value::Void),
            _ => return None,
        });
    }
    if r.owner == "net/minecraft/world/inventory/ContainerLevelAccess"
        && key == "container-level-access/null"
    {
        return Some(match r.name.as_str() {
            "execute" => Ok(Value::Void),
            "evaluate" => Ok(args.get(1).cloned().unwrap_or(Value::Int(1))),
            _ => return None,
        });
    }
    // The superclass constructor is the one reached by Runes' constructor.
    if r.owner == "net/minecraft/world/inventory/ItemCombinerMenu" && r.name == "<init>" {
        states.insert(key.clone(), new_state(&key));
        return Some(Ok(Value::Void));
    }
    if r.owner == "net/minecraft/world/inventory/AbstractContainerMenu" {
        return Some(match (r.name.as_str(), r.descriptor.as_str()) {
            ("getInventorySlotStart", "()I") => Ok(Value::Int(3)),
            ("getUseRowEnd", "()I") | ("getRowEnd", "()I") => Ok(Value::Int(RUNE_MENU_SLOT_COUNT)),
            _ => return None,
        });
    }
    if (r.owner == "net/minecraft/world/Container"
        || r.owner == "net/minecraft/world/SimpleContainer")
        && key.starts_with("menu/")
        && key.ends_with("/input")
    {
        return Some(match r.name.as_str() {
            "getContainerSize" => Ok(Value::Int(2)),
            "isEmpty" => Ok(Value::Int(1)),
            _ => return None,
        });
    }
    let state_key = if states.contains_key(&key) {
        Some(key.clone())
    } else {
        states
            .iter()
            .find(|(_, state)| state.input == key || state.result == key)
            .map(|(menu, _)| menu.clone())
    };
    let state = state_key.and_then(|menu| states.get_mut(&menu))?;
    if r.owner == "net/minecraft/world/inventory/AbstractContainerMenu" {
        return Some(match (r.name.as_str(), r.descriptor.as_str()) {
            ("getResultSlot", "()I") => Ok(Value::Int(2)),
            ("getInventorySlotStart", "()I") => Ok(Value::Int(3)),
            ("getUseRowEnd", "()I") => Ok(Value::Int(RUNE_MENU_SLOT_COUNT)),
            ("getRowEnd", "()I") => Ok(Value::Int(RUNE_MENU_SLOT_COUNT)),
            ("slotsChanged", _) | ("removed", _) => Ok(Value::Void),
            ("stillValid", "(Lnet/minecraft/world/entity/player/Player;)Z") => Ok(Value::Int(1)),
            _ => return None,
        });
    }
    if r.owner == "net/minecraft/world/inventory/ItemCombinerMenu" {
        return Some(match (r.name.as_str(), r.descriptor.as_str()) {
            ("createContainer", "(I)Lnet/minecraft/world/SimpleContainer;") => {
                Ok(Value::Ref(Some(state.input.clone())))
            }
            ("slotsChanged", "(Lnet/minecraft/world/Container;)V") => Ok(Value::Void),
            ("stillValid", _) | ("canTakeItemForPickAll", _) => Ok(Value::Int(1)),
            _ => return None,
        });
    }
    if r.owner == "net/minecraft/world/Container"
        || r.owner == "net/minecraft/world/SimpleContainer"
    {
        let marker = expect_ref(receiver).map_or("", String::as_str);
        if marker == state.input {
            return Some(container_call(state, false, r, args));
        }
        if marker == state.result {
            return Some(container_call(state, true, r, args));
        }
    }
    if r.owner == "net/minecraft/world/inventory/ResultContainer" {
        return Some(container_call(state, true, r, args));
    }
    None
}

fn container_call(
    state: &mut MenuState,
    result: bool,
    r: &MethodRef,
    args: &[Value],
) -> Result<Value, NativeError> {
    let index = args
        .first()
        .and_then(|v| match v {
            Value::Int(i) => Some(*i),
            _ => None,
        })
        .unwrap_or(0);
    match (r.name.as_str(), r.descriptor.as_str()) {
        ("getContainerSize", "()I") => Ok(Value::Int(if result { 1 } else { 2 })),
        ("isEmpty", "()Z") => Ok(Value::Int(if result {
            matches!(state.result_item, Value::Ref(Some(ref s)) if s == "stack/empty")
        } else {
            state
                .input_items
                .iter()
                .all(|v| matches!(v, Value::Ref(Some(s)) if s == "stack/empty"))
        } as i32)),
        ("getItem", _) => Ok(if result {
            state.result_item.clone()
        } else {
            state
                .input_items
                .get(index as usize)
                .cloned()
                .unwrap_or_else(|| Value::Ref(Some("stack/empty".into())))
        }),
        ("setItem", _) => {
            if result {
                state.result_item = args
                    .get(1)
                    .cloned()
                    .unwrap_or(Value::Ref(Some("stack/empty".into())));
            } else if let Some(slot) = state.input_items.get_mut(index as usize) {
                *slot = args
                    .get(1)
                    .cloned()
                    .unwrap_or(Value::Ref(Some("stack/empty".into())));
            }
            Ok(Value::Void)
        }
        ("setChanged", _)
        | ("clearContent", _)
        | ("setRecipeUsed", _)
        | ("awardUsedRecipes", _) => Ok(Value::Void),
        ("getRecipeUsed", _) => Ok(Value::Ref(None)),
        _ => Err(NativeError::UnknownHostCall(r.clone())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rune_layout_is_the_vanilla_layout() {
        assert_eq!(RUNE_SLOT_LAYOUT, [(0, 27, 47), (1, 76, 47), (2, 134, 47)]);
        assert_eq!(RUNE_MENU_SLOT_COUNT, 39);
        let state = new_state("net/runes/crafting/RuneCraftingScreenHandler#1");
        assert!(state.input.ends_with("/input"));
        assert!(state.result.ends_with("/result"));
    }
}
