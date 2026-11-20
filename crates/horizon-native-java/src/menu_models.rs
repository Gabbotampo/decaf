//! Exact, generic vanilla menu/container models used by mod-owned menus.
//!
//! Item-combiner slot geometry is data supplied by the mod's own
//! `ItemCombinerMenuSlotDefinition.Builder` bytecode. Nothing here knows a
//! particular mod or menu type.

use crate::{MethodRef, NativeError, Value, expect_ref};
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub(crate) struct SlotDef {
    index: i32,
    x: i32,
    y: i32,
}

#[derive(Clone, Debug)]
pub(crate) struct Definition {
    inputs: Vec<SlotDef>,
    result: SlotDef,
}

#[derive(Clone, Debug)]
pub(crate) struct Slot {
    index: i32,
    x: i32,
    y: i32,
}

#[derive(Clone, Debug)]
pub(crate) struct MenuState {
    pub input: String,
    pub result: String,
    pub slots: String,
    definition: Definition,
    layout: Vec<Slot>,
    slot_count: i32,
}

#[derive(Clone, Debug)]
pub(crate) enum Object {
    Menu(MenuState),
    Builder {
        inputs: Vec<SlotDef>,
        result: Option<SlotDef>,
    },
    Definition(Definition),
    SlotDefinition(SlotDef),
    Slot(Slot),
    Container {
        size: i32,
        items: Vec<Value>,
        result: bool,
    },
}

pub(crate) type States = BTreeMap<String, Object>;

pub(crate) fn is_menu_class(owner: &str) -> bool {
    matches!(
        owner,
        "net/minecraft/world/inventory/AbstractContainerMenu"
            | "net/minecraft/world/inventory/ItemCombinerMenu"
            | "net/minecraft/world/inventory/ItemCombinerMenuSlotDefinition"
            | "net/minecraft/world/inventory/ItemCombinerMenuSlotDefinition$Builder"
            | "net/minecraft/world/inventory/ItemCombinerMenuSlotDefinition$SlotDefinition"
            | "net/minecraft/world/SimpleContainer"
            | "net/minecraft/world/inventory/ResultContainer"
            | "net/minecraft/world/Container"
            | "net/minecraft/world/inventory/Slot"
            | "net/minecraft/world/inventory/ContainerLevelAccess"
    )
}

fn marker(key: impl Into<String>) -> Value {
    Value::Ref(Some(key.into()))
}

fn next_key(states: &States, prefix: &str) -> String {
    let mut n = states.len();
    loop {
        let key = format!("menu/{prefix}/{n}");
        if !states.contains_key(&key) {
            return key;
        }
        n += 1;
    }
}

#[allow(dead_code)]
fn empty_definition() -> Definition {
    Definition {
        inputs: Vec::new(),
        result: SlotDef {
            index: 0,
            x: 0,
            y: 0,
        },
    }
}

#[allow(dead_code)]
fn menu(state: &Object) -> Option<&MenuState> {
    match state {
        Object::Menu(state) => Some(state),
        _ => None,
    }
}

#[allow(dead_code)]
fn menu_mut(state: &mut Object) -> Option<&mut MenuState> {
    match state {
        Object::Menu(state) => Some(state),
        _ => None,
    }
}

pub(crate) fn field(states: &States, receiver: &Value, r: &MethodRef) -> Option<Value> {
    let key = expect_ref(receiver)?;
    let object = states.get(key)?;
    match (object, r.owner.as_str(), r.name.as_str()) {
        (Object::Menu(state), "net/minecraft/world/inventory/ItemCombinerMenu", "inputSlots") => {
            Some(marker(state.input.clone()))
        }
        (Object::Menu(state), "net/minecraft/world/inventory/ItemCombinerMenu", "resultSlots") => {
            Some(marker(state.result.clone()))
        }
        (Object::Menu(state), "net/minecraft/world/inventory/AbstractContainerMenu", "slots") => {
            Some(marker(state.slots.clone()))
        }
        (Object::Slot(slot), "net/minecraft/world/inventory/Slot", "x") => Some(Value::Int(slot.x)),
        (Object::Slot(slot), "net/minecraft/world/inventory/Slot", "y") => Some(Value::Int(slot.y)),
        (Object::Slot(slot), "net/minecraft/world/inventory/Slot", "index") => {
            Some(Value::Int(slot.index))
        }
        _ => None,
    }
}

pub(crate) fn static_call(
    states: &mut States,
    r: &MethodRef,
    _args: &[Value],
) -> Option<Result<Value, NativeError>> {
    if r.owner == "net/minecraft/world/inventory/ItemCombinerMenuSlotDefinition"
        && r.name == "create"
    {
        let key = next_key(states, "builder");
        states.insert(
            key.clone(),
            Object::Builder {
                inputs: Vec::new(),
                result: None,
            },
        );
        return Some(Ok(marker(key)));
    }
    None
}

pub(crate) fn call(
    states: &mut States,
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

    // The builder and immutable definition are ordinary objects in vanilla.
    if r.owner == "net/minecraft/world/inventory/ItemCombinerMenuSlotDefinition" {
        let Some(Object::Definition(definition)) = states.get(&key).cloned() else {
            return None;
        };
        return Some(match r.name.as_str() {
            "getNumOfInputSlots" => Ok(Value::Int(definition.inputs.len() as i32)),
            "getResultSlotIndex" => Ok(Value::Int(definition.result.index)),
            "getResultSlot" => {
                let out = next_key(states, "slot-definition");
                states.insert(out.clone(), Object::SlotDefinition(definition.result));
                Ok(marker(out))
            }
            "getSlot" => {
                let Some(Value::Int(index)) = args.first() else {
                    return None;
                };
                let slot = definition.inputs.get(*index as usize).cloned()?;
                let out = next_key(states, "slot-definition");
                states.insert(out.clone(), Object::SlotDefinition(slot));
                Ok(marker(out))
            }
            "getSlots" => Ok(marker(format!("menu/{key}/definition-slots"))),
            _ => return None,
        });
    }
    if r.owner == "net/minecraft/world/inventory/ItemCombinerMenuSlotDefinition$SlotDefinition" {
        let Some(Object::SlotDefinition(slot)) = states.get(&key) else {
            return None;
        };
        return Some(match r.name.as_str() {
            "slotIndex" => Ok(Value::Int(slot.index)),
            "x" => Ok(Value::Int(slot.x)),
            "y" => Ok(Value::Int(slot.y)),
            "mayPlace" => Ok(Value::Ref(None)),
            _ => return None,
        });
    }

    if r.owner == "net/minecraft/world/inventory/ItemCombinerMenuSlotDefinition$Builder" {
        match r.name.as_str() {
            "<init>" => {
                states.insert(
                    key,
                    Object::Builder {
                        inputs: Vec::new(),
                        result: None,
                    },
                );
                return Some(Ok(Value::Void));
            }
            "withSlot" => {
                let Some(Object::Builder { inputs, .. }) = states.get_mut(&key) else {
                    return None;
                };
                let [Value::Int(index), Value::Int(x), Value::Int(y), ..] = args else {
                    return None;
                };
                inputs.push(SlotDef {
                    index: *index,
                    x: *x,
                    y: *y,
                });
                return Some(Ok(receiver.clone()));
            }
            "withResultSlot" => {
                let Some(Object::Builder { result, .. }) = states.get_mut(&key) else {
                    return None;
                };
                let [Value::Int(index), Value::Int(x), Value::Int(y), ..] = args else {
                    return None;
                };
                *result = Some(SlotDef {
                    index: *index,
                    x: *x,
                    y: *y,
                });
                return Some(Ok(receiver.clone()));
            }
            "build" => {
                let Some(Object::Builder { inputs, result }) = states.remove(&key) else {
                    return None;
                };
                let result = result?;
                if inputs
                    .iter()
                    .enumerate()
                    .any(|(i, slot)| slot.index != i as i32)
                    || result.index != inputs.len() as i32
                {
                    return Some(Err(NativeError::UncaughtException {
                        class: "java/lang/IllegalArgumentException".into(),
                        message: Some("Expected continuous input and result slot indexes".into()),
                        frame: None,
                    }));
                }
                let definition = Definition { inputs, result };
                let out = next_key(states, "definition");
                states.insert(out.clone(), Object::Definition(definition));
                return Some(Ok(marker(out)));
            }
            _ => {}
        }
    }

    if r.owner == "net/minecraft/world/inventory/ItemCombinerMenu" && r.name == "<init>" {
        let Some(Value::Ref(Some(def_key))) = args.get(4) else {
            return None;
        };
        let Some(Object::Definition(definition)) = states.get(def_key).cloned() else {
            return None;
        };
        let input = format!("{key}/input");
        let result = format!("{key}/result");
        let slots = format!("{key}/slots");
        let mut layout = definition
            .inputs
            .iter()
            .map(|slot| Slot {
                index: slot.index,
                x: slot.x,
                y: slot.y,
            })
            .collect::<Vec<_>>();
        layout.push(Slot {
            index: definition.result.index,
            x: definition.result.x,
            y: definition.result.y,
        });
        for row in 0..3 {
            for column in 0..9 {
                layout.push(Slot {
                    index: 9 * (row + 1) + column,
                    x: 8 + 18 * column,
                    y: 84 + 18 * row,
                });
            }
        }
        for column in 0..9 {
            layout.push(Slot {
                index: column,
                x: 8 + 18 * column,
                y: 142,
            });
        }
        states.insert(
            key.clone(),
            Object::Menu(MenuState {
                input: input.clone(),
                result: result.clone(),
                slots,
                definition: definition.clone(),
                layout,
                slot_count: 0,
            }),
        );
        states.insert(
            input,
            Object::Container {
                size: definition.inputs.len() as i32,
                items: vec![marker("stack/empty"); definition.inputs.len()],
                result: false,
            },
        );
        states.insert(
            result,
            Object::Container {
                size: 1,
                items: vec![marker("stack/empty")],
                result: true,
            },
        );
        return Some(Ok(Value::Void));
    }
    if r.owner == "net/minecraft/world/inventory/ItemCombinerMenu" && r.name == "createContainer" {
        let Some(Object::Menu(state)) = states.get(&key) else {
            return None;
        };
        let input_key = state.input.clone();
        let size = args
            .first()
            .and_then(|v| {
                if let Value::Int(i) = v {
                    Some(*i)
                } else {
                    None
                }
            })
            .unwrap_or(0)
            .max(0);
        states.insert(
            input_key.clone(),
            Object::Container {
                size,
                items: vec![marker("stack/empty"); size as usize],
                result: false,
            },
        );
        return Some(Ok(marker(input_key)));
    }
    if r.owner == "net/minecraft/world/inventory/Slot" && r.name == "<init>" {
        let [
            Value::Ref(Some(_container)),
            Value::Int(index),
            Value::Int(x),
            Value::Int(y),
        ] = args
        else {
            return None;
        };
        states.insert(
            key,
            Object::Slot(Slot {
                index: *index,
                x: *x,
                y: *y,
            }),
        );
        return Some(Ok(Value::Void));
    }
    if r.owner == "net/minecraft/world/SimpleContainer" && r.name == "<init>" {
        let size = args
            .first()
            .and_then(|v| {
                if let Value::Int(i) = v {
                    Some(*i)
                } else {
                    None
                }
            })
            .unwrap_or(0)
            .max(0);
        states.insert(
            key,
            Object::Container {
                size,
                items: vec![marker("stack/empty"); size as usize],
                result: false,
            },
        );
        return Some(Ok(Value::Void));
    }
    if r.owner == "net/minecraft/world/inventory/ResultContainer" && r.name == "<init>" {
        states.insert(
            key,
            Object::Container {
                size: 1,
                items: vec![marker("stack/empty")],
                result: true,
            },
        );
        return Some(Ok(Value::Void));
    }

    let slot_present = args
        .first()
        .and_then(|value| match value {
            Value::Ref(Some(slot_key)) => {
                Some(matches!(states.get(slot_key), Some(Object::Slot(_))))
            }
            _ => None,
        })
        .unwrap_or(false);
    if let Some(Object::Menu(state)) = states.get_mut(&key)
        && r.owner == "net/minecraft/world/inventory/AbstractContainerMenu"
    {
        return Some(match (r.name.as_str(), r.descriptor.as_str()) {
            ("getResultSlot", "()I") => Ok(Value::Int(state.definition.result.index)),
            ("getInventorySlotStart", "()I") => Ok(Value::Int(state.definition.result.index + 1)),
            ("getInventorySlotEnd", "()I") => {
                Ok(Value::Int(state.definition.result.index + 1 + 27))
            }
            ("getUseRowStart", "()I") => Ok(Value::Int(state.definition.result.index + 1 + 27)),
            ("getUseRowEnd", "()I") => Ok(Value::Int(state.definition.result.index + 1 + 27 + 9)),
            ("getRowEnd", "()I") => Ok(Value::Int(state.definition.result.index + 1 + 27 + 9)),
            ("addSlot", _) => {
                if slot_present {
                    state.slot_count += 1;
                    let _ = state.layout.len();
                }
                Ok(args.first().cloned().unwrap_or(Value::Ref(None)))
            }
            ("slotsChanged", _) | ("removed", _) => Ok(Value::Void),
            ("stillValid", _) => Ok(Value::Int(1)),
            _ => return None,
        });
    }

    // Containers are shared by SimpleContainer, ResultContainer and the
    // Container interface. The recipe manager remains deliberately counted.
    if let Some(Object::Container {
        size,
        items,
        result,
    }) = states.get_mut(&key)
        && (r.owner == "net/minecraft/world/Container"
            || r.owner == "net/minecraft/world/SimpleContainer"
            || r.owner == "net/minecraft/world/inventory/ResultContainer")
    {
        let index = args
            .first()
            .and_then(|v| {
                if let Value::Int(i) = v {
                    Some(*i as usize)
                } else {
                    None
                }
            })
            .unwrap_or(0);
        return Some(match r.name.as_str() {
            "getContainerSize" => Ok(Value::Int(*size)),
            "isEmpty" => Ok(Value::Int(items.iter().all(|v| {
                matches!(v, Value::Ref(None))
                    || matches!(v, Value::Ref(Some(s)) if s == "stack/empty")
            }) as i32)),
            "getItem" => Ok(items
                .get(index)
                .cloned()
                .unwrap_or_else(|| marker("stack/empty"))),
            "setItem" => {
                if let Some(value) = args.get(1)
                    && let Some(item) = items.get_mut(index)
                {
                    *item = value.clone();
                }
                Ok(Value::Void)
            }
            "setChanged" | "clearContent" | "setRecipeUsed" | "awardUsedRecipes" => Ok(Value::Void),
            "getRecipeUsed" => Ok(Value::Ref(None)),
            _ => {
                let _ = result;
                return None;
            }
        });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn method(owner: &str, name: &str, descriptor: &str) -> MethodRef {
        MethodRef {
            owner: owner.into(),
            name: name.into(),
            descriptor: descriptor.into(),
        }
    }

    #[test]
    fn definition_drives_smithing_like_ranges() {
        let mut states = States::new();
        let builder = static_call(
            &mut states,
            &method(
                "net/minecraft/world/inventory/ItemCombinerMenuSlotDefinition",
                "create",
                "()Lnet/minecraft/world/inventory/ItemCombinerMenuSlotDefinition$Builder;",
            ),
            &[],
        )
        .unwrap()
        .unwrap();
        let receiver = builder.clone();
        for (index, x, y) in [(0, 8, 48), (1, 26, 48), (2, 44, 48)] {
            call(&mut states, &method("net/minecraft/world/inventory/ItemCombinerMenuSlotDefinition$Builder", "withSlot", "(IIILjava/util/function/Predicate;)Lnet/minecraft/world/inventory/ItemCombinerMenuSlotDefinition$Builder;"), &receiver, &[Value::Int(index), Value::Int(x), Value::Int(y), Value::Ref(None)]).unwrap().unwrap();
        }
        call(
            &mut states,
            &method(
                "net/minecraft/world/inventory/ItemCombinerMenuSlotDefinition$Builder",
                "withResultSlot",
                "(III)Lnet/minecraft/world/inventory/ItemCombinerMenuSlotDefinition$Builder;",
            ),
            &receiver,
            &[Value::Int(3), Value::Int(98), Value::Int(48)],
        )
        .unwrap()
        .unwrap();
        let definition = call(
            &mut states,
            &method(
                "net/minecraft/world/inventory/ItemCombinerMenuSlotDefinition$Builder",
                "build",
                "()Lnet/minecraft/world/inventory/ItemCombinerMenuSlotDefinition;",
            ),
            &receiver,
            &[],
        )
        .unwrap()
        .unwrap();
        let menu = Value::Ref(Some("menu/test".into()));
        call(&mut states, &method("net/minecraft/world/inventory/ItemCombinerMenu", "<init>", "(Lnet/minecraft/world/inventory/MenuType;ILnet/minecraft/world/entity/player/Inventory;Lnet/minecraft/world/inventory/ContainerLevelAccess;Lnet/minecraft/world/inventory/ItemCombinerMenuSlotDefinition;)V"), &menu, &[Value::Ref(None), Value::Int(0), Value::Ref(None), Value::Ref(Some("container-level-access/null".into())), definition]).unwrap().unwrap();
        for (name, expected) in [
            ("getResultSlot", 3),
            ("getInventorySlotStart", 4),
            ("getInventorySlotEnd", 31),
            ("getUseRowStart", 31),
            ("getUseRowEnd", 40),
            ("getRowEnd", 40),
        ] {
            assert_eq!(
                call(
                    &mut states,
                    &method(
                        "net/minecraft/world/inventory/AbstractContainerMenu",
                        name,
                        "()I"
                    ),
                    &menu,
                    &[]
                )
                .unwrap()
                .unwrap(),
                Value::Int(expected)
            );
        }
        assert_eq!(
            match states.get("menu/test") {
                Some(Object::Menu(state)) => state.layout.len(),
                _ => 0,
            },
            40
        );
    }
}
