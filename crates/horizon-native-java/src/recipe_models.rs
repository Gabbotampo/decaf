//! General vanilla 26.3 recipe, ingredient, live item stack, and component-patch models.
//!
//! Nothing here knows any particular mod, recipe class, or slot layout. Mod recipes decode
//! their own `data/<ns>/recipe/*.json` resources through their own `RecipeSerializer` / DFU
//! `MapCodec`, and match/assemble inputs by executing their own `Recipe.matches` and
//! `Recipe.assemble` bytecode.

use std::collections::{BTreeMap, BTreeSet};

use crate::host_ref::{ClassHierarchy, HostKind, HostRef, Methods, Nested, exact_class_rules_out};
use crate::{
    Boxed, Host, MethodRef, NativeError, Runtime, StdHost, Value, expect_int, expect_ref,
    vanilla_item_data,
};

pub(crate) const ITEM_STACK: &str = "net/minecraft/world/item/ItemStack";
pub(crate) const ITEM_INSTANCE: &str = "net/minecraft/world/item/ItemInstance";
pub(crate) const DATA_COMPONENT_HOLDER: &str = "net/minecraft/core/component/DataComponentHolder";
pub(crate) const DATA_COMPONENT_GETTER: &str = "net/minecraft/core/component/DataComponentGetter";
pub(crate) const DATA_COMPONENT_PATCH: &str = "net/minecraft/core/component/DataComponentPatch";
pub(crate) const DATA_COMPONENT_PATCH_BUILDER: &str =
    "net/minecraft/core/component/DataComponentPatch$Builder";
pub(crate) const DATA_COMPONENTS: &str = "net/minecraft/core/component/DataComponents";
pub(crate) const INGREDIENT: &str = "net/minecraft/world/item/crafting/Ingredient";
pub(crate) const RECIPE_COMMON_INFO: &str = "net/minecraft/world/item/crafting/Recipe$CommonInfo";
pub(crate) const RECIPE_HOLDER: &str = "net/minecraft/world/item/crafting/RecipeHolder";
pub(crate) const RECIPE_MANAGER: &str = "net/minecraft/world/item/crafting/RecipeManager";
pub(crate) const RECIPE_ACCESS: &str = "net/minecraft/world/item/crafting/RecipeAccess";
pub(crate) const TRANSMUTE_RECIPE: &str = "net/minecraft/world/item/crafting/TransmuteRecipe";
pub(crate) const FEATURE_FLAGS: &str = "net/minecraft/world/flag/FeatureFlags";
pub(crate) const MENU_TYPE: &str = "net/minecraft/world/inventory/MenuType";
pub(crate) const EMPTY_PATCH_KEY: &str = "empty";
pub(crate) const RECIPE_MANAGER_KEY: &str = "game/recipe-manager";

#[derive(Clone, Debug, Default)]
pub(crate) struct ComponentPatch {
    pub(crate) entries: BTreeMap<String, Option<Value>>,
}

impl ComponentPatch {
    pub(crate) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub(crate) fn size(&self) -> i32 {
        self.entries.len() as i32
    }
}

#[derive(Clone, Debug)]
pub(crate) struct LiveStack {
    pub(crate) item_id: String,
    pub(crate) count: i32,
    pub(crate) patch: ComponentPatch,
}

impl LiveStack {
    pub(crate) fn is_empty(&self) -> bool {
        self.count <= 0 || self.item_id == "minecraft:air"
    }

    pub(crate) fn effective_item_id(&self) -> &str {
        if self.is_empty() {
            "minecraft:air"
        } else {
            &self.item_id
        }
    }

    pub(crate) fn effective_count(&self) -> i32 {
        if self.is_empty() { 0 } else { self.count }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct TemplateData {
    pub(crate) item_id: String,
    pub(crate) count: i32,
    pub(crate) patch: ComponentPatch,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum IngredientSpec {
    Tag(String),
    Items(Vec<String>),
}

#[derive(Clone, Debug)]
pub(crate) struct HolderData {
    pub(crate) id: Value,
    pub(crate) value: Value,
}

#[derive(Clone, Debug)]
pub(crate) struct MenuTypeData {
    pub(crate) supplier: Value,
    pub(crate) features: Value,
}

#[derive(Clone, Debug)]
pub(crate) struct LoadedRecipe {
    pub(crate) id: String,
    pub(crate) source_pack: String,
    pub(crate) recipe_type: Value,
    pub(crate) recipe: Value,
    pub(crate) holder: Value,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct LoadedRecipeCache {
    pub(crate) fingerprint: (usize, usize, u64, usize),
    pub(crate) recipes: Vec<LoadedRecipe>,
    pub(crate) errors: Vec<(String, String)>,
    pub(crate) unmodelled_serializers: BTreeSet<String>,
    pub(crate) skipped_recipe_ids: BTreeSet<String>,
}

/// Persistent recipe, live-stack, template, ingredient, and component-patch state on [`StdHost`].
#[derive(Clone, Debug, Default)]
pub(crate) struct RecipeState {
    next_id: u64,
    pub(crate) stacks: BTreeMap<String, LiveStack>,
    pub(crate) templates: BTreeMap<String, TemplateData>,
    pub(crate) patches: BTreeMap<String, ComponentPatch>,
    pub(crate) patch_builders: BTreeMap<String, ComponentPatch>,
    pub(crate) ingredients: BTreeMap<String, IngredientSpec>,
    pub(crate) holders: BTreeMap<String, HolderData>,
    pub(crate) common_infos: BTreeMap<String, bool>,
    pub(crate) menu_types: BTreeMap<String, MenuTypeData>,
    pub(crate) extra_item_tags: BTreeMap<String, Vec<String>>,
    pub(crate) loaded: Option<LoadedRecipeCache>,
}

impl RecipeState {
    fn next_key(&mut self, prefix: &str) -> String {
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        format!("{prefix}/{id}")
    }

    pub(crate) fn alloc_stack(&mut self, stack: LiveStack) -> Value {
        let key = self.next_key("recipe-stack");
        self.stacks.insert(key.clone(), stack);
        Value::Host(HostRef::new(HostKind::LiveItemStack, key))
    }

    pub(crate) fn alloc_template(&mut self, data: TemplateData) -> Value {
        let key = self.next_key("recipe-template");
        self.templates.insert(key.clone(), data);
        Value::Host(HostRef::new(HostKind::ItemStackTemplate, key))
    }

    pub(crate) fn alloc_patch(&mut self, patch: ComponentPatch) -> Value {
        if patch.is_empty() {
            return empty_patch();
        }
        let key = self.next_key("recipe-patch");
        self.patches.insert(key.clone(), patch);
        Value::Host(HostRef::new(HostKind::DataComponentPatch, key))
    }

    pub(crate) fn alloc_patch_builder(&mut self) -> Value {
        let key = self.next_key("recipe-patch-builder");
        self.patch_builders
            .insert(key.clone(), ComponentPatch::default());
        Value::Host(HostRef::new(HostKind::DataComponentPatchBuilder, key))
    }

    pub(crate) fn alloc_ingredient(&mut self, spec: IngredientSpec) -> Value {
        let key = self.next_key("recipe-ingredient");
        self.ingredients.insert(key.clone(), spec);
        Value::Host(HostRef::new(HostKind::Ingredient, key))
    }

    pub(crate) fn alloc_common_info(&mut self, show_notification: bool) -> Value {
        let key = self.next_key("recipe-common-info");
        self.common_infos.insert(key.clone(), show_notification);
        Value::Host(HostRef::new(HostKind::RecipeCommonInfo, key))
    }

    pub(crate) fn alloc_holder(&mut self, id: Value, value: Value) -> Value {
        let key = self.next_key("recipe-holder");
        self.holders.insert(key.clone(), HolderData { id, value });
        Value::Host(HostRef::new(HostKind::RecipeHolder, key))
    }

    pub(crate) fn patch_of(&self, value: &Value) -> Option<&ComponentPatch> {
        let Value::Host(handle) = value else {
            return None;
        };
        if handle.kind() != HostKind::DataComponentPatch {
            return None;
        }
        if handle.key() == EMPTY_PATCH_KEY {
            static EMPTY: std::sync::OnceLock<ComponentPatch> = std::sync::OnceLock::new();
            return Some(EMPTY.get_or_init(ComponentPatch::default));
        }
        self.patches.get(handle.key())
    }

    pub(crate) fn root_texts<'a>(&'a self, out: &mut Vec<&'a str>) {
        fn push_val<'a>(value: &'a Value, out: &mut Vec<&'a str>) {
            out.extend(crate::heap::value_texts(value));
        }
        for stack in self.stacks.values() {
            for val in stack.patch.entries.values().flatten() {
                push_val(val, out);
            }
        }
        for tpl in self.templates.values() {
            for val in tpl.patch.entries.values().flatten() {
                push_val(val, out);
            }
        }
        for patch in self.patches.values().chain(self.patch_builders.values()) {
            for val in patch.entries.values().flatten() {
                push_val(val, out);
            }
        }
        for holder in self.holders.values() {
            push_val(&holder.id, out);
            push_val(&holder.value, out);
        }
        for mt in self.menu_types.values() {
            push_val(&mt.supplier, out);
            push_val(&mt.features, out);
        }
        if let Some(cache) = &self.loaded {
            for recipe in &cache.recipes {
                push_val(&recipe.recipe_type, out);
                push_val(&recipe.recipe, out);
                push_val(&recipe.holder, out);
            }
        }
    }

    pub(crate) fn entry_count(&self) -> usize {
        self.stacks.len()
            + self.templates.len()
            + self.patches.len()
            + self.patch_builders.len()
            + self.ingredients.len()
            + self.holders.len()
            + self.common_infos.len()
            + self.menu_types.len()
    }
}

pub(crate) fn empty_patch() -> Value {
    Value::Host(HostRef::new(HostKind::DataComponentPatch, EMPTY_PATCH_KEY))
}

pub(crate) fn is_empty_stack_handle(value: &Value) -> bool {
    crate::game_object::handle_key(value) == Some("client/stack/empty")
}

pub(crate) fn is_recipe_object_kind(kind: HostKind) -> bool {
    matches!(
        kind,
        HostKind::LiveItemStack
            | HostKind::ItemStackTemplate
            | HostKind::DataComponentPatch
            | HostKind::DataComponentPatchBuilder
            | HostKind::Ingredient
            | HostKind::RecipeCommonInfo
            | HostKind::RecipeHolder
    )
}

fn fixed_instance_of(
    exact: &'static str,
    supertypes: &'static [&'static str],
    wanted: &str,
    hierarchy: &dyn ClassHierarchy,
) -> Option<bool> {
    if wanted == "java/lang/Object" || wanted == exact || supertypes.contains(&wanted) {
        return Some(true);
    }
    if hierarchy.is_instance(exact, wanted) == Some(true) {
        return Some(true);
    }
    (hierarchy.is_interface(wanted) == Some(false) || exact_class_rules_out(exact, wanted))
        .then_some(false)
}

fn identity_equals(
    handle: &HostRef,
    other: &Value,
    _: &mut dyn Nested,
) -> Result<Option<bool>, NativeError> {
    Ok(match other {
        Value::Host(other) => handle.same_object(other),
        _ => Some(false),
    })
}

pub(crate) const LIVE_STACK_METHODS: Methods = Methods {
    to_string: |_, _| Ok(None),
    hash_code: |_, _| Ok(None),
    equals: identity_equals,
    instance_of: |_, wanted, hierarchy| {
        fixed_instance_of(
            ITEM_STACK,
            &[ITEM_INSTANCE, DATA_COMPONENT_HOLDER, DATA_COMPONENT_GETTER],
            wanted,
            hierarchy,
        )
    },
    crosses_jars: |_, _| false,
    compare_to: None,
};

pub(crate) const PATCH_METHODS: Methods = Methods {
    to_string: |handle, _| Ok((handle.key() == EMPTY_PATCH_KEY).then(|| "{}".to_owned())),
    hash_code: |_, _| Ok(None),
    equals: |handle, other, _| {
        if handle.key() == EMPTY_PATCH_KEY {
            return Ok(match other {
                Value::Host(other)
                    if other.kind() == HostKind::DataComponentPatch
                        && other.key() == EMPTY_PATCH_KEY =>
                {
                    Some(true)
                }
                Value::Host(other) if other.kind() == HostKind::DataComponentPatch => None,
                _ => Some(false),
            });
        }
        Ok(None)
    },
    instance_of: |_, wanted, hierarchy| {
        fixed_instance_of(DATA_COMPONENT_PATCH, &[], wanted, hierarchy)
    },
    crosses_jars: |_, _| false,
    compare_to: None,
};

pub(crate) const PATCH_BUILDER_METHODS: Methods = Methods {
    to_string: |_, _| Ok(None),
    hash_code: |_, _| Ok(None),
    equals: identity_equals,
    instance_of: |_, wanted, hierarchy| {
        fixed_instance_of(DATA_COMPONENT_PATCH_BUILDER, &[], wanted, hierarchy)
    },
    crosses_jars: |_, _| false,
    compare_to: None,
};

pub(crate) const INGREDIENT_METHODS: Methods = Methods {
    to_string: |_, _| Ok(None),
    hash_code: |_, _| Ok(None),
    equals: |_, _, _| Ok(None),
    instance_of: |_, wanted, hierarchy| {
        fixed_instance_of(
            INGREDIENT,
            &["java/util/function/Predicate"],
            wanted,
            hierarchy,
        )
    },
    crosses_jars: |_, _| false,
    compare_to: None,
};

pub(crate) const COMMON_INFO_METHODS: Methods = Methods {
    to_string: |_, _| Ok(None),
    hash_code: |_, _| Ok(None),
    equals: |_, _, _| Ok(None),
    instance_of: |_, wanted, hierarchy| {
        fixed_instance_of(RECIPE_COMMON_INFO, &["java/lang/Record"], wanted, hierarchy)
    },
    crosses_jars: |_, _| false,
    compare_to: None,
};

pub(crate) const HOLDER_METHODS: Methods = Methods {
    to_string: |_, _| Ok(None),
    hash_code: |_, _| Ok(None),
    equals: |_, _, _| Ok(None),
    instance_of: |_, wanted, hierarchy| {
        fixed_instance_of(RECIPE_HOLDER, &["java/lang/Record"], wanted, hierarchy)
    },
    crosses_jars: |_, _| false,
    compare_to: None,
};

pub(crate) fn normalize_id(raw: &str) -> Option<String> {
    if raw.is_empty() {
        return None;
    }
    let (ns, path) = match raw.split_once(':') {
        Some((ns, path)) => (ns, path),
        None => ("minecraft", raw),
    };
    if ns.is_empty()
        || path.is_empty()
        || !ns.bytes().all(|b| {
            b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'_' | b'-' | b'.')
        })
        || !path.bytes().all(|b| {
            b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'_' | b'-' | b'.' | b'/')
        })
    {
        return None;
    }
    Some(format!("{ns}:{path}"))
}

/// Convert a JSON number to a truncated 64-bit integer matching DFU `JsonOps` + Gson
/// `LazilyParsedNumber` semantics (`1.2` -> `1`, `-1.9` -> `-1`, `1e2` -> `100`, wrapping on
/// narrowing casts to `i32`/`i16`/`i8`). Returns `None` for non-numbers.
pub(crate) fn json_number_to_i64(val: &serde_json::Value) -> Option<i64> {
    let num = val.as_number()?;
    if let Some(i) = num.as_i64() {
        return Some(i);
    }
    if let Some(u) = num.as_u64() {
        return Some(u as i64);
    }
    let f = num.as_f64()?;
    f.is_finite().then_some(f.trunc() as i64)
}

pub(crate) fn is_known_item_id(host: &StdHost, id: &str) -> bool {
    vanilla_item_data::is_vanilla_item(id)
        || host
            .registrations()
            .iter()
            .any(|entry| entry.registry == "minecraft:item" && entry.id == id)
}

pub(crate) fn item_value_for_id(host: &StdHost, id: &str) -> Value {
    if let Some(entry) = host
        .registrations()
        .iter()
        .rev()
        .find(|entry| entry.registry == "minecraft:item" && entry.id == id)
    {
        return entry.value.clone();
    }
    crate::game_object::server_value(format!("game/item/{id}"))
}

pub(crate) fn item_holder_for_id(host: &StdHost, id: &str) -> Value {
    crate::host_registry_holder::vanilla("minecraft:item", id, item_value_for_id(host, id))
}

pub(crate) fn resolve_item_id(host: &StdHost, value: &Value) -> Option<String> {
    let Value::Host(handle) = value else {
        return None;
    };
    match handle.kind() {
        HostKind::GameRegistryObject | HostKind::GameValue => {
            if let Some(id) = handle
                .key()
                .strip_prefix("game/item/")
                .or_else(|| handle.key().strip_prefix("client/item/"))
            {
                return Some(id.to_owned());
            }
            if let Some(block_id) = handle
                .key()
                .strip_prefix("game/block/")
                .or_else(|| handle.key().strip_prefix("client/block/"))
            {
                return crate::vanilla_block_items::item_id(block_id).map(str::to_owned);
            }
            None
        }
        HostKind::RegistryHolder => {
            let held = handle.values()?;
            let reg = crate::host_ids::registry_name(held.first()?)?;
            let id = crate::host_ids::identifier_key(held.get(1)?)?.to_owned();
            if reg == "minecraft:item" {
                Some(id)
            } else if reg == "minecraft:block" {
                crate::vanilla_block_items::item_id(&id).map(str::to_owned)
            } else {
                None
            }
        }
        HostKind::DirectHolder => {
            let inner = handle.values()?.first()?.clone();
            resolve_item_id(host, &inner)
        }
        HostKind::JarObject => {
            let key = handle.key();
            if let Some(field) =
                key.strip_prefix("net/minecraft/world/item/Item#net/minecraft/world/item/Items.")
            {
                return crate::vanilla_items::field_id(field);
            }
            if let Some(entry) = host.registrations().iter().rev().find(|entry| {
                entry.registry == "minecraft:item"
                    && crate::jar_object::object_key(&entry.value) == Some(key)
            }) {
                return Some(entry.id.clone());
            }
            if let Some(block_item) = host.block_item_for_block_key(key) {
                return resolve_item_id(host, block_item);
            }
            None
        }
        _ => None,
    }
}

pub(crate) fn tag_key_parts(value: &Value) -> Option<(String, String)> {
    if !crate::host_ids::is_tag_key(value) {
        return None;
    }
    let reg = crate::host_ids::registry_name(&crate::host_ids::key_registry(value)?)?;
    let loc = crate::host_ids::identifier_key(&crate::host_ids::key_location(value)?)?.to_owned();
    Some((reg, loc))
}

pub(crate) fn resolve_data_component_type_id(host: &StdHost, value: &Value) -> Option<String> {
    if let Some(key) = crate::game_object::handle_key(value)
        && let Some(id) = key.strip_prefix("game/data-component-type/")
    {
        return Some(id.to_owned());
    }
    if let Some(entry) =
        host.registrations().iter().rev().find(|entry| {
            entry.registry == "minecraft:data_component_type" && entry.value == *value
        })
    {
        return Some(entry.id.clone());
    }
    None
}

fn base_max_stack_size(host: &StdHost, item_id: &str) -> Option<i32> {
    if let Some(size) = vanilla_item_data::vanilla_max_stack_size(item_id) {
        return Some(size);
    }
    let entry = host
        .registrations()
        .iter()
        .rev()
        .find(|entry| entry.registry == "minecraft:item" && entry.id == item_id)?;
    let marker = expect_ref(&entry.value)?;
    if let Some(props) = host.item_properties_for_marker(marker) {
        if let Some(raw) = props.get("stacksTo") {
            return raw.parse::<i32>().ok();
        }
        if props.contains_key("durability") {
            return Some(1);
        }
        return Some(64);
    }
    // Constructed via `new Item(...)` or `new BlockItem(...)` with no custom properties override.
    host.is_constructed_item_marker(marker).then_some(64)
}

fn base_max_damage(host: &StdHost, item_id: &str) -> Option<Option<i32>> {
    if let Some(dmg) = vanilla_item_data::vanilla_max_damage(item_id) {
        return Some(dmg);
    }
    let entry = host
        .registrations()
        .iter()
        .rev()
        .find(|entry| entry.registry == "minecraft:item" && entry.id == item_id)?;
    let marker = expect_ref(&entry.value)?;
    if let Some(props) = host.item_properties_for_marker(marker) {
        if let Some(raw) = props.get("durability") {
            return raw.parse::<i32>().ok().map(Some);
        }
        return Some(None);
    }
    host.is_constructed_item_marker(marker).then_some(None)
}

/// Exact prototype value of `comp_id` on `item_id`, or `None` when the prototype value is not
/// modelled (callers must count the operation rather than guessing `null`).
pub(crate) fn prototype_component(
    host: &StdHost,
    item_id: &str,
    comp_id: &str,
) -> Option<Option<Value>> {
    if item_id == "minecraft:air" || !is_known_item_id(host, item_id) {
        return None;
    }
    match comp_id {
        "minecraft:max_stack_size" => {
            base_max_stack_size(host, item_id).map(|n| Some(Value::Boxed(Boxed::Integer(n))))
        }
        "minecraft:repair_cost" => Some(Some(Value::Boxed(Boxed::Integer(0)))),
        "minecraft:max_damage" => {
            base_max_damage(host, item_id).map(|opt| opt.map(|d| Value::Boxed(Boxed::Integer(d))))
        }
        "minecraft:damage" => {
            base_max_damage(host, item_id).map(|opt| opt.map(|_| Value::Boxed(Boxed::Integer(0))))
        }
        // Verified empty across all 1,658 vanilla 26.3 item prototypes.
        "minecraft:custom_name"
        | "minecraft:custom_data"
        | "minecraft:custom_model_data"
        | "minecraft:unbreakable"
        | "minecraft:profile" => Some(None),
        _ => {
            // Mod-registered component types default to absent on unconfigured prototypes.
            let is_mod_component = host
                .registrations()
                .iter()
                .any(|e| e.registry == "minecraft:data_component_type" && e.id == comp_id);
            is_mod_component.then_some(None)
        }
    }
}

/// Compare a candidate patch value with a known prototype value (`Objects.equals(val, proto)`).
/// Returns `None` if equality cannot be determined without running custom Java `.equals`.
fn value_equals_prototype(val: &Option<Value>, proto: &Option<Value>) -> Option<bool> {
    match (val, proto) {
        (None, None) => Some(true),
        (None, Some(_)) | (Some(_), None) => Some(false),
        (Some(v), Some(Value::Boxed(Boxed::Integer(p)))) => match v {
            Value::Boxed(Boxed::Integer(n)) | Value::Int(n) => Some(*n == *p),
            Value::Ref(None) => Some(false),
            Value::Boxed(_) | Value::Long(_) | Value::Float(_) | Value::Ref(Some(_)) => Some(false),
            Value::Host(h) if h.kind() != HostKind::JarObject => Some(false),
            _ => None,
        },
        _ => None,
    }
}

/// Exact `PatchedDataComponentMap.set(type, value)` semantics on a stack's patch.
pub(crate) fn patched_set(
    host: &StdHost,
    item_id: &str,
    patch: &mut ComponentPatch,
    comp_id: &str,
    new_val: &Value,
) -> Option<Value> {
    let proto = prototype_component(host, item_id, comp_id)?;
    let new_opt = (!matches!(new_val, Value::Ref(None))).then(|| new_val.clone());
    let matches_proto = value_equals_prototype(&new_opt, &proto)?;
    let old_entry = if matches_proto {
        patch.entries.remove(comp_id)
    } else {
        patch.entries.insert(comp_id.to_owned(), new_opt)
    };
    let proto_val = proto.unwrap_or(Value::Ref(None));
    Some(match old_entry {
        Some(Some(prev)) => prev,
        // `Objects.requireNonNullElse(Removed.removedToNull(old), proto)` -> `proto`.
        Some(None) | None => proto_val,
    })
}

/// Exact `PatchedDataComponentMap.remove(type)` semantics on a stack's patch.
pub(crate) fn patched_remove(
    host: &StdHost,
    item_id: &str,
    patch: &mut ComponentPatch,
    comp_id: &str,
) -> Option<Value> {
    let proto = prototype_component(host, item_id, comp_id)?;
    let old_entry = if proto.is_some() {
        patch.entries.insert(comp_id.to_owned(), None)
    } else {
        patch.entries.remove(comp_id)
    };
    Some(match old_entry {
        Some(Some(prev)) => prev,
        Some(None) => Value::Ref(None),
        None => proto.unwrap_or(Value::Ref(None)),
    })
}

/// Exact `PatchedDataComponentMap.applyPatch(overlay)` semantics: drops entries redundant with the
/// item's prototype and returns `None` (without mutating `patch`) if any overlay key's prototype
/// is unmodelled.
pub(crate) fn patched_apply_patch(
    host: &StdHost,
    item_id: &str,
    patch: &mut ComponentPatch,
    overlay: &ComponentPatch,
) -> Option<()> {
    let mut actions: Vec<(String, Option<Option<Value>>)> =
        Vec::with_capacity(overlay.entries.len());
    for (comp_id, entry) in &overlay.entries {
        let proto = prototype_component(host, item_id, comp_id)?;
        let action = match entry {
            Some(_) => {
                if value_equals_prototype(entry, &proto)? {
                    None
                } else {
                    Some(entry.clone())
                }
            }
            None => {
                if proto.is_some() {
                    Some(None)
                } else {
                    None
                }
            }
        };
        actions.push((comp_id.clone(), action));
    }
    for (comp_id, action) in actions {
        match action {
            Some(entry) => {
                patch.entries.insert(comp_id, entry);
            }
            None => {
                patch.entries.remove(&comp_id);
            }
        }
    }
    Some(())
}

pub(crate) fn max_stack_size_for_item(
    host: &StdHost,
    item_id: &str,
    patch: &ComponentPatch,
) -> Option<i32> {
    match patch.entries.get("minecraft:max_stack_size") {
        Some(Some(Value::Int(n))) | Some(Some(Value::Boxed(Boxed::Integer(n)))) => Some(*n),
        Some(Some(_)) => None,
        Some(None) => Some(1),
        None => base_max_stack_size(host, item_id),
    }
}

pub(crate) fn resolve_item_tag(host: &StdHost, raw_tag_id: &str) -> Option<BTreeSet<String>> {
    let stripped = raw_tag_id.strip_prefix('#').unwrap_or(raw_tag_id);
    let tag_id = normalize_id(stripped)?;
    let mut visiting = BTreeSet::new();
    resolve_item_tag_inner(host, &tag_id, &mut visiting)
}

fn resolve_item_tag_inner(
    host: &StdHost,
    tag_id: &str,
    visiting: &mut BTreeSet<String>,
) -> Option<BTreeSet<String>> {
    if !visiting.insert(tag_id.to_owned()) {
        return Some(BTreeSet::new());
    }
    let mut found_any = false;
    let mut entries: Vec<(String, bool)> = Vec::new();
    // Layer 0 (lowest priority): vanilla built-in item tags.
    if let Some(vanilla_entries) = vanilla_item_data::vanilla_item_tag(tag_id) {
        found_any = true;
        for entry in vanilla_entries {
            entries.push(((*entry).to_owned(), true));
        }
    }
    // Stacked datapack/jar layers in vanilla pack order (lowest -> highest priority).
    if let Some((ns, path)) = tag_id.split_once(':') {
        let resource_path = format!("data/{ns}/tags/item/{path}.json");
        for (_, layer) in host.ordered_data_layers() {
            let Some(bytes) = layer.get(&resource_path) else {
                continue;
            };
            let Ok(serde_json::Value::Object(map)) = serde_json::from_slice(bytes) else {
                visiting.remove(tag_id);
                return None;
            };
            found_any = true;
            if map.get("replace").and_then(serde_json::Value::as_bool) == Some(true) {
                entries.clear();
            }
            if let Some(values) = map.get("values").and_then(serde_json::Value::as_array) {
                for item in values {
                    match item {
                        serde_json::Value::String(id) => entries.push((id.clone(), true)),
                        serde_json::Value::Object(obj) => {
                            if let Some(id) = obj.get("id").and_then(serde_json::Value::as_str) {
                                let required = obj
                                    .get("required")
                                    .and_then(serde_json::Value::as_bool)
                                    .unwrap_or(true);
                                entries.push((id.to_owned(), required));
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
    if let Some(extra) = host.recipes.extra_item_tags.get(tag_id) {
        found_any = true;
        for entry in extra {
            entries.push((entry.clone(), true));
        }
    }
    if !found_any {
        visiting.remove(tag_id);
        return None;
    }
    let mut resolved = BTreeSet::new();
    for (entry, required) in entries {
        if let Some(sub_tag) = entry.strip_prefix('#') {
            let Some(norm_sub) = normalize_id(sub_tag) else {
                if required {
                    visiting.remove(tag_id);
                    return None;
                }
                continue;
            };
            match resolve_item_tag_inner(host, &norm_sub, visiting) {
                Some(sub_items) => resolved.extend(sub_items),
                None if !required => {}
                None => {
                    visiting.remove(tag_id);
                    return None;
                }
            }
        } else {
            let Some(item_id) = normalize_id(&entry) else {
                if required {
                    visiting.remove(tag_id);
                    return None;
                }
                continue;
            };
            if is_known_item_id(host, &item_id) {
                resolved.insert(item_id);
            } else if required {
                visiting.remove(tag_id);
                return None;
            }
        }
    }
    visiting.remove(tag_id);
    Some(resolved)
}

pub(crate) fn decode_component_patch_json(
    host: &mut StdHost,
    value: &serde_json::Value,
) -> Result<ComponentPatch, DecodeError> {
    let Some(obj) = value.as_object() else {
        return Err(DecodeError::Invalid(format!(
            "Not a JSON object for DataComponentPatch: {value}"
        )));
    };
    let mut patch = ComponentPatch::default();
    for (raw_key, val) in obj {
        if let Some(removed) = raw_key.strip_prefix('!') {
            let Some(id) = normalize_id(removed) else {
                return Err(DecodeError::Invalid(format!(
                    "Invalid component id: {raw_key}"
                )));
            };
            patch.entries.insert(id, None);
            continue;
        }
        let Some(id) = normalize_id(raw_key) else {
            return Err(DecodeError::Invalid(format!(
                "Invalid component id: {raw_key}"
            )));
        };
        let decoded = match id.as_str() {
            "minecraft:max_stack_size" => {
                let Some(n) = json_number_to_i64(val).map(|v| v as i32) else {
                    return Err(DecodeError::Invalid(format!(
                        "Not a number for minecraft:max_stack_size: {val}"
                    )));
                };
                if !(1..=99).contains(&n) {
                    return Err(DecodeError::Invalid(format!(
                        "Value must be within range [1;99]: {n}"
                    )));
                }
                Value::Boxed(Boxed::Integer(n))
            }
            "minecraft:max_damage" => {
                let Some(n) = json_number_to_i64(val).map(|v| v as i32) else {
                    return Err(DecodeError::Invalid(format!(
                        "Not a number for minecraft:max_damage: {val}"
                    )));
                };
                if n < 1 {
                    return Err(DecodeError::Invalid(format!(
                        "Value must be positive for minecraft:max_damage: {n}"
                    )));
                }
                Value::Boxed(Boxed::Integer(n))
            }
            "minecraft:damage" | "minecraft:repair_cost" => {
                let Some(n) = json_number_to_i64(val).map(|v| v as i32) else {
                    return Err(DecodeError::Invalid(format!(
                        "Not a number for {id}: {val}"
                    )));
                };
                if n < 0 {
                    return Err(DecodeError::Invalid(format!(
                        "Value must be non-negative for {id}: {n}"
                    )));
                }
                Value::Boxed(Boxed::Integer(n))
            }
            "minecraft:custom_name" | "minecraft:item_name" => match val.as_str() {
                Some(text) => crate::component_value("literal", text),
                None => {
                    return Err(DecodeError::Unmodelled(format!(
                        "non-string text component JSON for {id}"
                    )));
                }
            },
            _ => {
                let _ = host;
                return Err(DecodeError::Unmodelled(format!(
                    "DataComponentPatch JSON entry for {id}"
                )));
            }
        };
        patch.entries.insert(id, Some(decoded));
    }
    Ok(patch)
}

#[derive(Clone, Debug)]
pub(crate) enum DecodeError {
    Invalid(String),
    Unmodelled(String),
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid(msg) | Self::Unmodelled(msg) => f.write_str(msg),
        }
    }
}

pub(crate) fn decode_ingredient_json(
    host: &mut StdHost,
    value: &serde_json::Value,
) -> Result<Value, DecodeError> {
    match value {
        serde_json::Value::String(text) => {
            if let Some(tag_raw) = text.strip_prefix('#') {
                let Some(tag_id) = normalize_id(tag_raw) else {
                    return Err(DecodeError::Invalid(format!(
                        "Invalid tag identifier: {text}"
                    )));
                };
                if resolve_item_tag(host, &tag_id).is_none() {
                    return Err(DecodeError::Invalid(format!(
                        "Missing tag: '{tag_id}' in 'minecraft:item'"
                    )));
                }
                Ok(host.recipes.alloc_ingredient(IngredientSpec::Tag(tag_id)))
            } else {
                let id = decode_non_air_item_id(host, text)?;
                Ok(host
                    .recipes
                    .alloc_ingredient(IngredientSpec::Items(vec![id])))
            }
        }
        serde_json::Value::Array(items) => {
            if items.is_empty() {
                return Err(DecodeError::Invalid("List must have contents".into()));
            }
            let mut ids = Vec::with_capacity(items.len());
            for elem in items {
                let Some(text) = elem.as_str() else {
                    return Err(DecodeError::Invalid(format!(
                        "Ingredient list element is not a string: {elem}"
                    )));
                };
                if text.starts_with('#') {
                    return Err(DecodeError::Invalid(format!(
                        "Tag not allowed inside ingredient list: {text}"
                    )));
                }
                ids.push(decode_non_air_item_id(host, text)?);
            }
            Ok(host.recipes.alloc_ingredient(IngredientSpec::Items(ids)))
        }
        _ => Err(DecodeError::Invalid(format!(
            "Not a valid Ingredient JSON value: {value}"
        ))),
    }
}

pub(crate) fn decode_non_air_item_id(host: &StdHost, raw: &str) -> Result<String, DecodeError> {
    let Some(id) = normalize_id(raw) else {
        return Err(DecodeError::Invalid(format!(
            "Invalid item identifier: {raw}"
        )));
    };
    if id == "minecraft:air" {
        return Err(DecodeError::Invalid(
            "Item must not be minecraft:air".into(),
        ));
    }
    if !is_known_item_id(host, &id) {
        return Err(DecodeError::Invalid(format!(
            "Unknown registry key in ResourceKey[minecraft:root / minecraft:item]: {id}"
        )));
    }
    Ok(id)
}

pub(crate) fn decode_item_stack_template_map(
    host: &mut StdHost,
    map: &serde_json::Map<String, serde_json::Value>,
) -> Result<Value, DecodeError> {
    // Evaluate all three RecordCodecBuilder fields (`id`, `count`, `components`) in field order
    // and accumulate errors just like DFU's `ItemStackTemplate.MAP_CODEC`.
    let mut invalid_errors = Vec::new();
    let mut first_unmodelled: Option<String> = None;

    let item_id = match map.get("id").filter(|v| !v.is_null()) {
        None => {
            invalid_errors.push("No key id in ItemStackTemplate".into());
            None
        }
        Some(v) => match v.as_str() {
            Some(raw_id) => match decode_non_air_item_id(host, raw_id) {
                Ok(id) => Some(id),
                Err(DecodeError::Invalid(msg)) => {
                    invalid_errors.push(msg);
                    None
                }
                Err(DecodeError::Unmodelled(msg)) => {
                    if first_unmodelled.is_none() {
                        first_unmodelled = Some(msg);
                    }
                    None
                }
            },
            None => {
                invalid_errors.push(format!("Not a string for ItemStackTemplate id: {v}"));
                None
            }
        },
    };

    // `ExtraCodecs.intRange(1, 99).optionalFieldOf("count", 1)`: JSON `null` = absent/default,
    // fractional JSON numbers truncate toward zero before range checking.
    let count = match map.get("count").filter(|v| !v.is_null()) {
        None => Some(1),
        Some(v) => match json_number_to_i64(v).map(|n| n as i32) {
            None => {
                invalid_errors.push(format!("Not a number for ItemStackTemplate count: {v}"));
                None
            }
            Some(n) if !(1..=99).contains(&n) => {
                invalid_errors.push(format!("Value must be within range [1;99]: {n}"));
                None
            }
            Some(n) => Some(n),
        },
    };

    let patch = match map.get("components").filter(|v| !v.is_null()) {
        None => Some(ComponentPatch::default()),
        Some(v) => match decode_component_patch_json(host, v) {
            Ok(p) => Some(p),
            Err(DecodeError::Invalid(msg)) => {
                invalid_errors.push(msg);
                None
            }
            Err(DecodeError::Unmodelled(msg)) => {
                if first_unmodelled.is_none() {
                    first_unmodelled = Some(msg);
                }
                None
            }
        },
    };

    if let Some(msg) = first_unmodelled {
        return Err(DecodeError::Unmodelled(msg));
    }
    if !invalid_errors.is_empty() {
        return Err(DecodeError::Invalid(invalid_errors.join("; ")));
    }
    Ok(host.recipes.alloc_template(TemplateData {
        item_id: item_id.unwrap(),
        count: count.unwrap(),
        patch: patch.unwrap(),
    }))
}

pub(crate) fn decode_item_stack_template_json(
    host: &mut StdHost,
    value: &serde_json::Value,
) -> Result<Value, DecodeError> {
    match value {
        serde_json::Value::String(raw_id) => {
            let item_id = decode_non_air_item_id(host, raw_id)?;
            Ok(host.recipes.alloc_template(TemplateData {
                item_id,
                count: 1,
                patch: ComponentPatch::default(),
            }))
        }
        serde_json::Value::Object(map) => decode_item_stack_template_map(host, map),
        _ => Err(DecodeError::Invalid(format!(
            "Not a valid ItemStackTemplate JSON value: {value}"
        ))),
    }
}

pub(crate) fn decode_common_info_map(
    host: &mut StdHost,
    map: &serde_json::Map<String, serde_json::Value>,
) -> Result<Value, DecodeError> {
    // `Codec.BOOL.optionalFieldOf("show_notification", true)`: JSON `null` = absent/default.
    let show_notification = match map.get("show_notification").filter(|v| !v.is_null()) {
        None => true,
        Some(serde_json::Value::Bool(b)) => *b,
        Some(other) => {
            return Err(DecodeError::Invalid(format!(
                "Not a boolean for show_notification: {other}"
            )));
        }
    };
    Ok(host.recipes.alloc_common_info(show_notification))
}

/// Exact MC 26.3 `ItemStackTemplate.apply(int count, DataComponentPatch basePatch)`:
/// constructs `new ItemStack(item, count, basePatch)`, calls `applyComponents(tpl.components)`,
/// then runs `ItemStack.validateStrict(stack)`:
/// - Empty stacks (`count <= 0` or `air`) pass `validateStrict` (their effective components are
///   `DataComponentMap.EMPTY`, `getCount() == 0 <= getMaxStackSize() == 1`) and are returned as
///   the newly constructed (empty) `ItemStack` instance, NOT replaced with `ItemStack.EMPTY`.
/// - Non-empty stacks fail validation and are replaced with `ItemStack.EMPTY` if they are both
///   damageable (`has(MAX_DAMAGE)`) and stackable (`getMaxStackSize() > 1`), or if
///   `count > getMaxStackSize()`.
/// - Stacks patched with unmodelled nested-item components (`container`, `bundle_contents`,
///   `charged_projectiles`) or unknown prototypes return `None` so the call is counted.
pub(crate) fn template_apply(
    host: &mut StdHost,
    template_key: &str,
    count: i32,
    base_patch: &ComponentPatch,
) -> Option<Value> {
    let tpl = host.recipes.templates.get(template_key)?.clone();
    let mut patch = ComponentPatch::default();
    if tpl.item_id != "minecraft:air" {
        patched_apply_patch(host, &tpl.item_id, &mut patch, base_patch)?;
        patched_apply_patch(host, &tpl.item_id, &mut patch, &tpl.patch)?;
    }
    let stack = LiveStack {
        item_id: tpl.item_id,
        count,
        patch,
    };
    if stack.is_empty() {
        return Some(host.recipes.alloc_stack(stack));
    }
    // `ItemStack.validateComponents` validates nested item sizes on `CONTAINER`,
    // `BUNDLE_CONTENTS`, and `CHARGED_PROJECTILES` when present; default prototypes of those
    // items are empty, but patched values are not modelled here -> count rather than guess.
    for nested_comp in [
        "minecraft:container",
        "minecraft:bundle_contents",
        "minecraft:charged_projectiles",
    ] {
        if matches!(stack.patch.entries.get(nested_comp), Some(Some(_))) {
            return None;
        }
    }
    let has_max_damage = !matches!(
        stack_component_get(host, &stack.item_id, &stack.patch, "minecraft:max_damage")?,
        Value::Ref(None)
    );
    let max_stack = max_stack_size_for_item(host, &stack.item_id, &stack.patch)?;
    if (has_max_damage && max_stack > 1) || stack.count > max_stack {
        return Some(crate::menu_models::empty_stack());
    }
    Some(host.recipes.alloc_stack(stack))
}

pub(crate) fn stack_patch_clone(host: &StdHost, stack: &Value) -> Option<ComponentPatch> {
    if is_empty_stack_handle(stack) {
        return Some(ComponentPatch::default());
    }
    let Value::Host(handle) = stack else {
        return None;
    };
    if handle.kind() != HostKind::LiveItemStack {
        return None;
    }
    let live = host.recipes.stacks.get(handle.key())?;
    if live.is_empty() {
        return Some(ComponentPatch::default());
    }
    Some(live.patch.clone())
}

pub(crate) fn stack_is_empty(host: &StdHost, stack: &Value) -> Option<bool> {
    if is_empty_stack_handle(stack) {
        return Some(true);
    }
    let Value::Host(handle) = stack else {
        return None;
    };
    if handle.kind() != HostKind::LiveItemStack {
        return None;
    }
    Some(host.recipes.stacks.get(handle.key())?.is_empty())
}

pub(crate) fn stack_max_stack_size(host: &StdHost, stack: &Value) -> Option<i32> {
    if is_empty_stack_handle(stack) {
        return Some(1);
    }
    let Value::Host(handle) = stack else {
        return None;
    };
    if handle.kind() != HostKind::LiveItemStack {
        return None;
    }
    let live = host.recipes.stacks.get(handle.key())?;
    if live.is_empty() {
        return Some(1);
    }
    max_stack_size_for_item(host, &live.item_id, &live.patch)
}

pub(crate) fn limit_stack_exact(host: &mut StdHost, stack: &Value, limit: i32) -> Option<()> {
    if is_empty_stack_handle(stack) {
        return Some(());
    }
    let Value::Host(handle) = stack else {
        return None;
    };
    if handle.kind() != HostKind::LiveItemStack {
        return None;
    }
    let key = handle.key().to_owned();
    let live = host.recipes.stacks.get_mut(&key)?;
    if !live.is_empty() && live.count > limit {
        live.count = limit;
    }
    Some(())
}

pub(crate) fn limit_stack_for_container(
    host: &mut StdHost,
    stack: &Value,
    container_max: i32,
) -> Option<()> {
    let item_max = stack_max_stack_size(host, stack)?;
    let limit = container_max.min(item_max);
    limit_stack_exact(host, stack, limit)
}

pub(crate) fn random_new(host: &mut StdHost, class: &str) -> Option<Value> {
    match class {
        ITEM_STACK if !host.in_client_frame() => Some(host.recipes.alloc_stack(LiveStack {
            item_id: "minecraft:air".into(),
            count: 0,
            patch: ComponentPatch::default(),
        })),
        "net/minecraft/world/item/ItemStackTemplate" => {
            Some(host.recipes.alloc_template(TemplateData {
                item_id: "minecraft:air".into(),
                count: 0,
                patch: ComponentPatch::default(),
            }))
        }
        RECIPE_COMMON_INFO => Some(host.recipes.alloc_common_info(true)),
        RECIPE_HOLDER => Some(
            host.recipes
                .alloc_holder(Value::Ref(None), Value::Ref(None)),
        ),
        _ => None,
    }
}

pub(crate) fn static_field(r: &MethodRef) -> Option<Value> {
    match (r.owner.as_str(), r.name.as_str(), r.descriptor.as_str()) {
        (DATA_COMPONENT_PATCH, "EMPTY", "Lnet/minecraft/core/component/DataComponentPatch;") => {
            Some(empty_patch())
        }
        (DATA_COMPONENTS, field, "Lnet/minecraft/core/component/DataComponentType;") => {
            vanilla_item_data::data_component_field_id(field).map(|id| {
                crate::game_object::server_value(format!("game/data-component-type/{id}"))
            })
        }
        (
            FEATURE_FLAGS,
            "VANILLA_SET" | "DEFAULT_FLAGS",
            "Lnet/minecraft/world/flag/FeatureFlagSet;",
        ) => Some(crate::game_object::server_value(
            "game/feature-flag-set/vanilla".into(),
        )),
        _ => None,
    }
}

fn npe() -> NativeError {
    NativeError::UncaughtException {
        class: "java/lang/NullPointerException".into(),
        message: None,
        frame: None,
    }
}

fn unsupported(message: &str) -> NativeError {
    NativeError::UncaughtException {
        class: "java/lang/UnsupportedOperationException".into(),
        message: Some(message.into()),
        frame: None,
    }
}

fn illegal_state(message: &str) -> NativeError {
    NativeError::UncaughtException {
        class: "java/lang/IllegalStateException".into(),
        message: Some(message.into()),
        frame: None,
    }
}

pub(crate) fn ingredient_test(
    host: &StdHost,
    spec: &IngredientSpec,
    stack: &Value,
) -> Option<bool> {
    if is_empty_stack_handle(stack) {
        return Some(false);
    }
    let Value::Host(handle) = stack else {
        return None;
    };
    if handle.kind() != HostKind::LiveItemStack {
        return None;
    }
    let live = host.recipes.stacks.get(handle.key())?;
    if live.is_empty() {
        return Some(false);
    }
    Some(match spec {
        IngredientSpec::Items(ids) => ids.iter().any(|id| id == &live.item_id),
        IngredientSpec::Tag(tag_id) => {
            resolve_item_tag(host, tag_id).is_some_and(|members| members.contains(&live.item_id))
        }
    })
}

pub(crate) fn static_call(
    host: &mut StdHost,
    r: &MethodRef,
    args: &[Value],
) -> Option<Result<Value, NativeError>> {
    match (r.owner.as_str(), r.name.as_str(), r.descriptor.as_str()) {
        (
            DATA_COMPONENT_PATCH,
            "builder",
            "()Lnet/minecraft/core/component/DataComponentPatch$Builder;",
        ) => Some(Ok(host.recipes.alloc_patch_builder())),
        (
            INGREDIENT,
            "of",
            "(Lnet/minecraft/world/level/ItemLike;)Lnet/minecraft/world/item/crafting/Ingredient;",
        ) => {
            let arg = args.first()?;
            if matches!(arg, Value::Ref(None)) {
                return Some(Err(npe()));
            }
            let item_id = resolve_item_id(host, arg)?;
            if item_id == "minecraft:air" {
                return Some(Err(unsupported("Ingredient can't contain air")));
            }
            Some(Ok(host
                .recipes
                .alloc_ingredient(IngredientSpec::Items(vec![item_id]))))
        }
        (
            INGREDIENT,
            "testOptionalIngredient",
            "(Ljava/util/Optional;Lnet/minecraft/world/item/ItemStack;)Z",
        ) => {
            let opt = args.first()?;
            let stack = args.get(1)?;
            if matches!(opt, Value::Ref(None)) {
                return Some(Err(npe()));
            }
            let Value::Host(opt_handle) = opt else {
                return None;
            };
            if opt_handle.kind() != HostKind::Optional {
                return None;
            }
            let held = opt_handle.values()?;
            if held.is_empty() {
                return stack_is_empty(host, stack).map(|empty| Ok(Value::Int(i32::from(empty))));
            }
            let Value::Host(ing_handle) = &held[0] else {
                return None;
            };
            if ing_handle.kind() != HostKind::Ingredient {
                return None;
            }
            let spec = host.recipes.ingredients.get(ing_handle.key())?;
            ingredient_test(host, spec, stack).map(|matched| Ok(Value::Int(i32::from(matched))))
        }
        (
            TRANSMUTE_RECIPE,
            "createWithOriginalComponents",
            "(Lnet/minecraft/world/item/ItemStackTemplate;Lnet/minecraft/world/item/ItemStack;)Lnet/minecraft/world/item/ItemStack;",
        ) => {
            let tpl = args.first()?;
            let base = args.get(1)?;
            if matches!(tpl, Value::Ref(None)) || matches!(base, Value::Ref(None)) {
                return Some(Err(npe()));
            }
            let Value::Host(tpl_handle) = tpl else {
                return None;
            };
            if tpl_handle.kind() != HostKind::ItemStackTemplate {
                return None;
            }
            let count = host.recipes.templates.get(tpl_handle.key())?.count;
            let base_patch = stack_patch_clone(host, base)?;
            template_apply(host, tpl_handle.key(), count, &base_patch).map(Ok)
        }
        (
            TRANSMUTE_RECIPE,
            "createWithOriginalComponents",
            "(Lnet/minecraft/world/item/ItemStackTemplate;Lnet/minecraft/world/item/ItemStack;I)Lnet/minecraft/world/item/ItemStack;",
        ) => {
            let tpl = args.first()?;
            let base = args.get(1)?;
            let extra = match args.get(2) {
                Some(Value::Int(n)) => *n,
                _ => return None,
            };
            if matches!(tpl, Value::Ref(None)) || matches!(base, Value::Ref(None)) {
                return Some(Err(npe()));
            }
            let Value::Host(tpl_handle) = tpl else {
                return None;
            };
            if tpl_handle.kind() != HostKind::ItemStackTemplate {
                return None;
            }
            let count = host
                .recipes
                .templates
                .get(tpl_handle.key())?
                .count
                .wrapping_add(extra);
            let base_patch = stack_patch_clone(host, base)?;
            template_apply(host, tpl_handle.key(), count, &base_patch).map(Ok)
        }
        _ => None,
    }
}

pub(crate) fn instance_call(
    host: &mut StdHost,
    r: &MethodRef,
    receiver: &Value,
    args: &[Value],
) -> Option<Result<Value, NativeError>> {
    // MenuType.<init>(MenuSupplier, FeatureFlagSet)
    if r.owner == MENU_TYPE
        && r.name == "<init>"
        && r.descriptor
            == "(Lnet/minecraft/world/inventory/MenuType$MenuSupplier;Lnet/minecraft/world/flag/FeatureFlagSet;)V"
        && let Some(marker) = crate::jar_object::object_key(receiver)
    {
        host.recipes.menu_types.insert(
            marker.to_owned(),
            MenuTypeData {
                supplier: args.first().cloned().unwrap_or(Value::Ref(None)),
                features: args.get(1).cloned().unwrap_or(Value::Ref(None)),
            },
        );
        return Some(Ok(Value::Void));
    }

    // Item.asItem() on known server item handles returns `this`.
    if !host.in_client_frame()
        && matches!(
            r.owner.as_str(),
            "net/minecraft/world/item/Item" | "net/minecraft/world/level/ItemLike"
        )
        && r.name == "asItem"
        && r.descriptor == "()Lnet/minecraft/world/item/Item;"
        && crate::game_object::handle_key(receiver).is_some_and(|key| key.starts_with("game/item/"))
    {
        return Some(Ok(receiver.clone()));
    }

    // Item.builtInRegistryHolder() on known items.
    if r.owner == "net/minecraft/world/item/Item"
        && r.name == "builtInRegistryHolder"
        && r.descriptor == "()Lnet/minecraft/core/Holder$Reference;"
        && let Some(item_id) = resolve_item_id(host, receiver)
    {
        return Some(Ok(item_holder_for_id(host, &item_id)));
    }

    // ItemStack.EMPTY outside a client frame.
    if !host.in_client_frame()
        && is_empty_stack_handle(receiver)
        && matches!(
            r.owner.as_str(),
            ITEM_STACK | ITEM_INSTANCE | DATA_COMPONENT_HOLDER | DATA_COMPONENT_GETTER
        )
    {
        return stack_instance_call_empty(host, r, args);
    }

    let Value::Host(handle) = receiver else {
        return None;
    };
    match handle.kind() {
        HostKind::LiveItemStack => live_stack_instance_call(host, handle, r, args),
        HostKind::DataComponentPatch => patch_instance_call(host, handle, r, args),
        HostKind::DataComponentPatchBuilder => patch_builder_instance_call(host, handle, r, args),
        HostKind::Ingredient => ingredient_instance_call(host, handle, r, args),
        HostKind::RecipeCommonInfo => common_info_instance_call(host, handle, r, args),
        HostKind::RecipeHolder => holder_instance_call(host, handle, r, args),
        _ => None,
    }
}

fn stack_instance_call_empty(
    host: &mut StdHost,
    r: &MethodRef,
    args: &[Value],
) -> Option<Result<Value, NativeError>> {
    match (r.name.as_str(), r.descriptor.as_str()) {
        ("isEmpty", "()Z") => Some(Ok(Value::Int(1))),
        ("getCount" | "count", "()I") => Some(Ok(Value::Int(0))),
        ("setCount" | "grow" | "shrink" | "limitSize", "(I)V") => Some(Ok(Value::Void)),
        // MC 26.3: ItemInstance.getMaxStackSize() = getOrDefault(MAX_STACK_SIZE, 1); EMPTY has empty map -> 1.
        ("getMaxStackSize", "()I") => Some(Ok(Value::Int(1))),
        ("getItem", "()Lnet/minecraft/world/item/Item;") => {
            Some(Ok(item_value_for_id(host, "minecraft:air")))
        }
        ("typeHolder" | "getItemHolder", "()Lnet/minecraft/core/Holder;") => {
            Some(Ok(item_holder_for_id(host, "minecraft:air")))
        }
        ("is", "(Lnet/minecraft/world/item/Item;)Z") => {
            let item_id = resolve_item_id(host, args.first()?)?;
            Some(Ok(Value::Int(i32::from(item_id == "minecraft:air"))))
        }
        ("is", "(Lnet/minecraft/core/Holder;)Z") => {
            let item_id = resolve_item_id(host, args.first()?)?;
            Some(Ok(Value::Int(i32::from(item_id == "minecraft:air"))))
        }
        ("is", "(Lnet/minecraft/tags/TagKey;)Z") => Some(Ok(Value::Int(0))),
        ("getComponentsPatch", "()Lnet/minecraft/core/component/DataComponentPatch;") => {
            Some(Ok(empty_patch()))
        }
        (
            "get" | "remove",
            "(Lnet/minecraft/core/component/DataComponentType;)Ljava/lang/Object;",
        ) => Some(Ok(Value::Ref(None))),
        (
            "set",
            "(Lnet/minecraft/core/component/DataComponentType;Ljava/lang/Object;)Ljava/lang/Object;",
        ) => Some(Ok(Value::Ref(None))),
        ("has", "(Lnet/minecraft/core/component/DataComponentType;)Z") => Some(Ok(Value::Int(0))),
        ("applyComponents", "(Lnet/minecraft/core/component/DataComponentPatch;)V") => {
            Some(Ok(Value::Void))
        }
        ("copy", "()Lnet/minecraft/world/item/ItemStack;")
        | ("copyWithCount", "(I)Lnet/minecraft/world/item/ItemStack;") => {
            Some(Ok(crate::menu_models::empty_stack()))
        }
        _ => None,
    }
}

fn live_stack_instance_call(
    host: &mut StdHost,
    handle: &HostRef,
    r: &MethodRef,
    args: &[Value],
) -> Option<Result<Value, NativeError>> {
    let key = handle.key().to_owned();
    if r.owner == ITEM_STACK && r.name == "<init>" {
        let (item_arg, count, patch) = match r.descriptor.as_str() {
            "(Lnet/minecraft/world/level/ItemLike;)V" | "(Lnet/minecraft/core/Holder;)V" => {
                (args.first()?, 1, ComponentPatch::default())
            }
            "(Lnet/minecraft/world/level/ItemLike;I)V" | "(Lnet/minecraft/core/Holder;I)V" => {
                let count = match expect_int(r, args.get(1)?) {
                    Ok(c) => c,
                    Err(e) => return Some(Err(e)),
                };
                (args.first()?, count, ComponentPatch::default())
            }
            "(Lnet/minecraft/core/Holder;ILnet/minecraft/core/component/DataComponentPatch;)V" => {
                let count = match expect_int(r, args.get(1)?) {
                    Ok(c) => c,
                    Err(e) => return Some(Err(e)),
                };
                let patch_arg = args.get(2)?;
                if matches!(patch_arg, Value::Ref(None)) {
                    return Some(Err(npe()));
                }
                let patch = host.recipes.patch_of(patch_arg)?.clone();
                (args.first()?, count, patch)
            }
            _ => return None,
        };
        if matches!(item_arg, Value::Ref(None)) {
            return Some(Err(npe()));
        }
        let item_id = resolve_item_id(host, item_arg)?;
        let mut sanitized = ComponentPatch::default();
        if item_id != "minecraft:air" && !patch.is_empty() {
            patched_apply_patch(host, &item_id, &mut sanitized, &patch)?;
        }
        host.recipes.stacks.insert(
            key,
            LiveStack {
                item_id,
                count,
                patch: sanitized,
            },
        );
        return Some(Ok(Value::Void));
    }

    if !matches!(
        r.owner.as_str(),
        ITEM_STACK | ITEM_INSTANCE | DATA_COMPONENT_HOLDER | DATA_COMPONENT_GETTER
    ) {
        return None;
    }

    let live = host.recipes.stacks.get(&key)?.clone();
    match (r.name.as_str(), r.descriptor.as_str()) {
        ("isEmpty", "()Z") => Some(Ok(Value::Int(i32::from(live.is_empty())))),
        ("getCount" | "count", "()I") => Some(Ok(Value::Int(live.effective_count()))),
        ("setCount", "(I)V") => {
            let count = match expect_int(r, args.first()?) {
                Ok(c) => c,
                Err(e) => return Some(Err(e)),
            };
            if let Some(slot) = host.recipes.stacks.get_mut(&key) {
                slot.count = count;
            }
            Some(Ok(Value::Void))
        }
        ("grow", "(I)V") => {
            let delta = match expect_int(r, args.first()?) {
                Ok(c) => c,
                Err(e) => return Some(Err(e)),
            };
            if let Some(slot) = host.recipes.stacks.get_mut(&key) {
                slot.count = slot.effective_count().wrapping_add(delta);
            }
            Some(Ok(Value::Void))
        }
        ("shrink", "(I)V") => {
            let delta = match expect_int(r, args.first()?) {
                Ok(c) => c,
                Err(e) => return Some(Err(e)),
            };
            if let Some(slot) = host.recipes.stacks.get_mut(&key) {
                slot.count = slot.effective_count().wrapping_sub(delta);
            }
            Some(Ok(Value::Void))
        }
        ("limitSize", "(I)V") => {
            let max = match expect_int(r, args.first()?) {
                Ok(c) => c,
                Err(e) => return Some(Err(e)),
            };
            if let Some(slot) = host.recipes.stacks.get_mut(&key)
                && !slot.is_empty()
                && slot.count > max
            {
                slot.count = max;
            }
            Some(Ok(Value::Void))
        }
        ("getMaxStackSize", "()I") => {
            if live.is_empty() {
                return Some(Ok(Value::Int(1)));
            }
            let max = max_stack_size_for_item(host, &live.item_id, &live.patch)?;
            Some(Ok(Value::Int(max)))
        }
        ("getItem", "()Lnet/minecraft/world/item/Item;") => {
            Some(Ok(item_value_for_id(host, live.effective_item_id())))
        }
        ("typeHolder" | "getItemHolder", "()Lnet/minecraft/core/Holder;") => {
            Some(Ok(item_holder_for_id(host, live.effective_item_id())))
        }
        ("is", "(Lnet/minecraft/world/item/Item;)Z") | ("is", "(Lnet/minecraft/core/Holder;)Z") => {
            let arg = args.first()?;
            if matches!(arg, Value::Ref(None)) {
                return Some(Ok(Value::Int(0)));
            }
            let other_id = resolve_item_id(host, arg)?;
            Some(Ok(Value::Int(i32::from(
                live.effective_item_id() == other_id,
            ))))
        }
        ("is", "(Lnet/minecraft/tags/TagKey;)Z") => {
            if live.is_empty() {
                return Some(Ok(Value::Int(0)));
            }
            let arg = args.first()?;
            let (reg, loc) = tag_key_parts(arg)?;
            if reg != "minecraft:item" {
                return Some(Ok(Value::Int(0)));
            }
            let matched =
                resolve_item_tag(host, &loc).is_some_and(|members| members.contains(&live.item_id));
            Some(Ok(Value::Int(i32::from(matched))))
        }
        ("getComponentsPatch", "()Lnet/minecraft/core/component/DataComponentPatch;") => {
            if live.is_empty() || live.patch.is_empty() {
                Some(Ok(empty_patch()))
            } else {
                Some(Ok(host.recipes.alloc_patch(live.patch)))
            }
        }
        (
            "set",
            "(Lnet/minecraft/core/component/DataComponentType;Ljava/lang/Object;)Ljava/lang/Object;",
        ) => {
            let type_arg = args.first()?;
            let val_arg = args.get(1)?;
            if matches!(type_arg, Value::Ref(None)) {
                return Some(Err(npe()));
            }
            let comp_id = resolve_data_component_type_id(host, type_arg)?;
            let mut next_patch = live.patch.clone();
            let prev = patched_set(host, &live.item_id, &mut next_patch, &comp_id, val_arg)?;
            if let Some(slot) = host.recipes.stacks.get_mut(&key) {
                slot.patch = next_patch;
            }
            Some(Ok(prev))
        }
        ("get", "(Lnet/minecraft/core/component/DataComponentType;)Ljava/lang/Object;") => {
            if live.is_empty() {
                return Some(Ok(Value::Ref(None)));
            }
            let type_arg = args.first()?;
            if matches!(type_arg, Value::Ref(None)) {
                return Some(Err(npe()));
            }
            let comp_id = resolve_data_component_type_id(host, type_arg)?;
            stack_component_get(host, &live.item_id, &live.patch, &comp_id).map(Ok)
        }
        ("remove", "(Lnet/minecraft/core/component/DataComponentType;)Ljava/lang/Object;") => {
            let type_arg = args.first()?;
            if matches!(type_arg, Value::Ref(None)) {
                return Some(Err(npe()));
            }
            let comp_id = resolve_data_component_type_id(host, type_arg)?;
            let mut next_patch = live.patch.clone();
            let prev = patched_remove(host, &live.item_id, &mut next_patch, &comp_id)?;
            if let Some(slot) = host.recipes.stacks.get_mut(&key) {
                slot.patch = next_patch;
            }
            Some(Ok(prev))
        }
        ("has", "(Lnet/minecraft/core/component/DataComponentType;)Z") => {
            if live.is_empty() {
                return Some(Ok(Value::Int(0)));
            }
            let type_arg = args.first()?;
            if matches!(type_arg, Value::Ref(None)) {
                return Some(Err(npe()));
            }
            let comp_id = resolve_data_component_type_id(host, type_arg)?;
            stack_component_get(host, &live.item_id, &live.patch, &comp_id)
                .map(|v| Ok(Value::Int(i32::from(!matches!(v, Value::Ref(None))))))
        }
        ("applyComponents", "(Lnet/minecraft/core/component/DataComponentPatch;)V") => {
            let patch_arg = args.first()?;
            if matches!(patch_arg, Value::Ref(None)) {
                return Some(Err(npe()));
            }
            let overlay = host.recipes.patch_of(patch_arg)?.clone();
            let mut next_patch = live.patch.clone();
            patched_apply_patch(host, &live.item_id, &mut next_patch, &overlay)?;
            if let Some(slot) = host.recipes.stacks.get_mut(&key) {
                slot.patch = next_patch;
            }
            Some(Ok(Value::Void))
        }
        ("copy", "()Lnet/minecraft/world/item/ItemStack;") => {
            if live.is_empty() {
                Some(Ok(crate::menu_models::empty_stack()))
            } else {
                Some(Ok(host.recipes.alloc_stack(live)))
            }
        }
        ("copyWithCount", "(I)Lnet/minecraft/world/item/ItemStack;") => {
            let count = match expect_int(r, args.first()?) {
                Ok(c) => c,
                Err(e) => return Some(Err(e)),
            };
            if live.is_empty() || count <= 0 {
                Some(Ok(crate::menu_models::empty_stack()))
            } else {
                let mut cloned = live;
                cloned.count = count;
                Some(Ok(host.recipes.alloc_stack(cloned)))
            }
        }
        _ => None,
    }
}

pub(crate) fn stack_component_get(
    host: &StdHost,
    item_id: &str,
    patch: &ComponentPatch,
    comp_id: &str,
) -> Option<Value> {
    if let Some(entry) = patch.entries.get(comp_id) {
        return Some(entry.clone().unwrap_or(Value::Ref(None)));
    }
    prototype_component(host, item_id, comp_id).map(|opt| opt.unwrap_or(Value::Ref(None)))
}

fn patch_instance_call(
    host: &mut StdHost,
    handle: &HostRef,
    r: &MethodRef,
    args: &[Value],
) -> Option<Result<Value, NativeError>> {
    if r.owner != DATA_COMPONENT_PATCH {
        return None;
    }
    let patch = host.recipes.patch_of(&Value::Host(handle.clone()))?.clone();
    match (r.name.as_str(), r.descriptor.as_str()) {
        ("isEmpty", "()Z") => Some(Ok(Value::Int(i32::from(patch.is_empty())))),
        ("size", "()I") => Some(Ok(Value::Int(patch.size()))),
        ("get", "(Lnet/minecraft/core/component/DataComponentType;)Ljava/util/Optional;") => {
            let type_arg = args.first()?;
            if matches!(type_arg, Value::Ref(None)) {
                return Some(Err(npe()));
            }
            let comp_id = resolve_data_component_type_id(host, type_arg)?;
            Some(Ok(match patch.entries.get(&comp_id) {
                None => Value::Ref(None),
                Some(None) => crate::empty_optional(),
                Some(Some(val)) => crate::optional_wrap(val.clone()),
            }))
        }
        _ => None,
    }
}

fn patch_builder_instance_call(
    host: &mut StdHost,
    handle: &HostRef,
    r: &MethodRef,
    args: &[Value],
) -> Option<Result<Value, NativeError>> {
    if r.owner != DATA_COMPONENT_PATCH_BUILDER {
        return None;
    }
    let key = handle.key().to_owned();
    match (r.name.as_str(), r.descriptor.as_str()) {
        (
            "set",
            "(Lnet/minecraft/core/component/DataComponentType;Ljava/lang/Object;)Lnet/minecraft/core/component/DataComponentPatch$Builder;",
        ) => {
            let type_arg = args.first()?;
            let val_arg = args.get(1)?;
            if matches!(type_arg, Value::Ref(None)) || matches!(val_arg, Value::Ref(None)) {
                return Some(Err(npe()));
            }
            let comp_id = resolve_data_component_type_id(host, type_arg)?;
            let builder = host.recipes.patch_builders.get_mut(&key)?;
            builder.entries.insert(comp_id, Some(val_arg.clone()));
            Some(Ok(Value::Host(handle.clone())))
        }
        (
            "remove",
            "(Lnet/minecraft/core/component/DataComponentType;)Lnet/minecraft/core/component/DataComponentPatch$Builder;",
        ) => {
            let type_arg = args.first()?;
            if matches!(type_arg, Value::Ref(None)) {
                return Some(Err(npe()));
            }
            let comp_id = resolve_data_component_type_id(host, type_arg)?;
            let builder = host.recipes.patch_builders.get_mut(&key)?;
            builder.entries.insert(comp_id, None);
            Some(Ok(Value::Host(handle.clone())))
        }
        ("build", "()Lnet/minecraft/core/component/DataComponentPatch;") => {
            let patch = host.recipes.patch_builders.get(&key)?.clone();
            Some(Ok(host.recipes.alloc_patch(patch)))
        }
        _ => None,
    }
}

fn ingredient_instance_call(
    host: &mut StdHost,
    handle: &HostRef,
    r: &MethodRef,
    args: &[Value],
) -> Option<Result<Value, NativeError>> {
    let spec = host.recipes.ingredients.get(handle.key())?.clone();
    match (r.owner.as_str(), r.name.as_str(), r.descriptor.as_str()) {
        (
            INGREDIENT | "java/util/function/Predicate",
            "test",
            "(Lnet/minecraft/world/item/ItemStack;)Z" | "(Ljava/lang/Object;)Z",
        ) => {
            let arg = args.first()?;
            if matches!(arg, Value::Ref(None)) {
                return Some(Err(npe()));
            }
            ingredient_test(host, &spec, arg).map(|matched| Ok(Value::Int(i32::from(matched))))
        }
        (INGREDIENT, "isEmpty", "()Z") => {
            let empty = match &spec {
                IngredientSpec::Items(ids) => ids.is_empty(),
                IngredientSpec::Tag(tag_id) => {
                    resolve_item_tag(host, tag_id).is_none_or(|set| set.is_empty())
                }
            };
            Some(Ok(Value::Int(i32::from(empty))))
        }
        _ => None,
    }
}

fn common_info_instance_call(
    host: &mut StdHost,
    handle: &HostRef,
    r: &MethodRef,
    args: &[Value],
) -> Option<Result<Value, NativeError>> {
    if r.owner != RECIPE_COMMON_INFO {
        return None;
    }
    let key = handle.key().to_owned();
    match (r.name.as_str(), r.descriptor.as_str()) {
        ("<init>", "(Z)V") => {
            let flag = match expect_int(r, args.first()?) {
                Ok(v) => v != 0,
                Err(e) => return Some(Err(e)),
            };
            host.recipes.common_infos.insert(key, flag);
            Some(Ok(Value::Void))
        }
        ("showNotification", "()Z") => {
            let flag = *host.recipes.common_infos.get(&key)?;
            Some(Ok(Value::Int(i32::from(flag))))
        }
        _ => None,
    }
}

fn holder_instance_call(
    host: &mut StdHost,
    handle: &HostRef,
    r: &MethodRef,
    args: &[Value],
) -> Option<Result<Value, NativeError>> {
    if r.owner != RECIPE_HOLDER {
        return None;
    }
    let key = handle.key().to_owned();
    match (r.name.as_str(), r.descriptor.as_str()) {
        (
            "<init>",
            "(Lnet/minecraft/resources/ResourceKey;Lnet/minecraft/world/item/crafting/Recipe;)V",
        ) => {
            let id = args.first()?.clone();
            let value = args.get(1)?.clone();
            host.recipes.holders.insert(key, HolderData { id, value });
            Some(Ok(Value::Void))
        }
        ("id", "()Lnet/minecraft/resources/ResourceKey;") => {
            Some(Ok(host.recipes.holders.get(&key)?.id.clone()))
        }
        ("value", "()Lnet/minecraft/world/item/crafting/Recipe;") => {
            Some(Ok(host.recipes.holders.get(&key)?.value.clone()))
        }
        _ => None,
    }
}

/// Stateful `toString` for recipe-model handles (invoked from both `Runtime` and `StdHost`).
pub(crate) fn object_to_string(host: &StdHost, handle: &HostRef) -> Option<String> {
    match handle.kind() {
        HostKind::LiveItemStack => {
            let live = host.recipes.stacks.get(handle.key())?;
            Some(format!(
                "{} {}",
                live.effective_count(),
                live.effective_item_id()
            ))
        }
        HostKind::RecipeCommonInfo => {
            let flag = *host.recipes.common_infos.get(handle.key())?;
            Some(format!("CommonInfo[showNotification={flag}]"))
        }
        HostKind::DataComponentPatch if handle.key() == EMPTY_PATCH_KEY => Some("{}".to_owned()),
        HostKind::RecipeHolder => {
            let holder = host.recipes.holders.get(handle.key())?;
            let Value::Host(id_handle) = &holder.id else {
                return None;
            };
            id_handle.to_string(&mut crate::MarkerText).ok().flatten()
        }
        _ => None,
    }
}

/// Stateful `hashCode` for recipe-model handles.
pub(crate) fn object_hash_code(host: &StdHost, handle: &HostRef) -> Option<i32> {
    match handle.kind() {
        HostKind::RecipeCommonInfo => {
            let flag = *host.recipes.common_infos.get(handle.key())?;
            Some(if flag { 1231 } else { 1237 })
        }
        _ => None,
    }
}

/// Stateful `equals` for recipe-model handles that don't need recursive jar `equals`.
pub(crate) fn host_object_equals(host: &StdHost, handle: &HostRef, other: &Value) -> Option<bool> {
    match handle.kind() {
        HostKind::LiveItemStack | HostKind::DataComponentPatchBuilder => match other {
            Value::Host(other) => handle.same_object(other),
            _ => Some(false),
        },
        HostKind::RecipeCommonInfo => match other {
            Value::Host(other) if other.kind() == HostKind::RecipeCommonInfo => {
                let a = host.recipes.common_infos.get(handle.key())?;
                let b = host.recipes.common_infos.get(other.key())?;
                Some(a == b)
            }
            _ => Some(false),
        },
        HostKind::Ingredient => match other {
            Value::Host(other) if other.kind() == HostKind::Ingredient => {
                let a = host.recipes.ingredients.get(handle.key())?;
                let b = host.recipes.ingredients.get(other.key())?;
                Some(a == b)
            }
            _ => Some(false),
        },
        HostKind::RecipeHolder => match other {
            Value::Host(other) if other.kind() == HostKind::RecipeHolder => {
                let a = host.recipes.holders.get(handle.key())?;
                let b = host.recipes.holders.get(other.key())?;
                // MC 26.3 RecipeHolder.equals compares `this.id == other.id` (ResourceKey identity).
                match (&a.id, &b.id) {
                    (Value::Host(ha), Value::Host(hb)) => ha.same_object(hb),
                    (Value::Ref(None), Value::Ref(None)) => Some(true),
                    (Value::Ref(None), _) | (_, Value::Ref(None)) => Some(false),
                    _ => None,
                }
            }
            _ => Some(false),
        },
        HostKind::DataComponentPatch => match other {
            Value::Host(other) if other.kind() == HostKind::DataComponentPatch => {
                let a = host.recipes.patch_of(&Value::Host(handle.clone()))?;
                let b = host.recipes.patch_of(&Value::Host(other.clone()))?;
                if a.entries.len() != b.entries.len() {
                    return Some(false);
                }
                if a.entries.is_empty() {
                    return Some(true);
                }
                None
            }
            _ => Some(false),
        },
        HostKind::ItemStackTemplate => match other {
            Value::Host(other) if other.kind() == HostKind::ItemStackTemplate => {
                let a = host.recipes.templates.get(handle.key())?;
                let b = host.recipes.templates.get(other.key())?;
                if a.item_id != b.item_id || a.count != b.count {
                    return Some(false);
                }
                if a.patch.is_empty() && b.patch.is_empty() {
                    return Some(true);
                }
                None
            }
            _ => Some(false),
        },
        _ => None,
    }
}

/// Runtime-side `equals` for patches and templates that may hold arbitrary Java values.
pub(crate) fn runtime_object_equals<H: Host>(
    runtime: &mut Runtime<'_, H>,
    handle: &HostRef,
    other: &Value,
) -> Result<Option<bool>, NativeError> {
    if let Some(std_host) = runtime.host.as_std_host()
        && let Some(ans) = host_object_equals(std_host, handle, other)
    {
        return Ok(Some(ans));
    }
    let Some(std_host) = runtime.host.as_std_host() else {
        return Ok(None);
    };
    match handle.kind() {
        HostKind::DataComponentPatch => {
            let Value::Host(other_h) = other else {
                return Ok(Some(false));
            };
            if other_h.kind() != HostKind::DataComponentPatch {
                return Ok(Some(false));
            }
            let (Some(a), Some(b)) = (
                std_host
                    .recipes
                    .patch_of(&Value::Host(handle.clone()))
                    .cloned(),
                std_host
                    .recipes
                    .patch_of(&Value::Host(other_h.clone()))
                    .cloned(),
            ) else {
                return Ok(None);
            };
            patches_equal_runtime(runtime, &a, &b)
        }
        HostKind::ItemStackTemplate => {
            let Value::Host(other_h) = other else {
                return Ok(Some(false));
            };
            if other_h.kind() != HostKind::ItemStackTemplate {
                return Ok(Some(false));
            }
            let (Some(a), Some(b)) = (
                std_host.recipes.templates.get(handle.key()).cloned(),
                std_host.recipes.templates.get(other_h.key()).cloned(),
            ) else {
                return Ok(None);
            };
            if a.item_id != b.item_id || a.count != b.count {
                return Ok(Some(false));
            }
            patches_equal_runtime(runtime, &a.patch, &b.patch)
        }
        _ => Ok(None),
    }
}

fn patches_equal_runtime<H: Host>(
    runtime: &mut Runtime<'_, H>,
    a: &ComponentPatch,
    b: &ComponentPatch,
) -> Result<Option<bool>, NativeError> {
    if a.entries.len() != b.entries.len() {
        return Ok(Some(false));
    }
    for (k, va) in &a.entries {
        let Some(vb) = b.entries.get(k) else {
            return Ok(Some(false));
        };
        match (va, vb) {
            (None, None) => {}
            (Some(x), Some(y)) => {
                let before = runtime.host.counted_unknown_calls();
                let equal = runtime.objects_equals(x, y)?;
                if runtime.host.counted_unknown_calls() != before {
                    return Ok(None);
                }
                if !equal {
                    return Ok(Some(false));
                }
            }
            _ => return Ok(Some(false)),
        }
    }
    Ok(Some(true))
}

/// Runtime-side `Ingredient.of(ItemLike...)` (reads array elements from `Runtime::arrays`).
pub(crate) fn runtime_static_call<H: Host>(
    runtime: &mut Runtime<'_, H>,
    r: &MethodRef,
    args: &[Value],
) -> Result<Option<Value>, NativeError> {
    if r.owner == INGREDIENT
        && r.name == "of"
        && r.descriptor
            == "([Lnet/minecraft/world/level/ItemLike;)Lnet/minecraft/world/item/crafting/Ingredient;"
    {
        let Some(array_arg) = args.first() else {
            return Ok(None);
        };
        if matches!(array_arg, Value::Ref(None)) {
            return Err(npe());
        }
        let Ok(elements) = runtime.array(array_arg).map(std::borrow::Cow::into_owned) else {
            return Ok(None);
        };
        let Some(std_host) = runtime.host.as_std_host_mut() else {
            return Ok(None);
        };
        let mut ids = Vec::with_capacity(elements.len());
        for elem in &elements {
            if matches!(elem, Value::Ref(None)) {
                return Err(npe());
            }
            let Some(id) = resolve_item_id(std_host, elem) else {
                return Ok(None);
            };
            if id == "minecraft:air" {
                return Err(unsupported("Ingredient can't contain air"));
            }
            ids.push(id);
        }
        return Ok(Some(
            std_host
                .recipes
                .alloc_ingredient(IngredientSpec::Items(ids)),
        ));
    }
    Ok(None)
}

fn cache_fingerprint(host: &StdHost) -> (usize, usize, u64, usize) {
    let serializers = host
        .registrations()
        .iter()
        .filter(|e| e.registry == "minecraft:recipe_serializer")
        .count();
    let items = host
        .registrations()
        .iter()
        .filter(|e| e.registry == "minecraft:item")
        .count();
    let res_gen = host.resource_generation();
    let tags = host.recipes.extra_item_tags.len();
    (serializers, items, res_gen, tags)
}

/// Read the `MapCodec` component from PR #614's canonical `RecipeSerializer` record handle.
fn serializer_codec(value: &Value) -> Option<Value> {
    let Value::Host(handle) = value else {
        return None;
    };
    let (codec, _) = crate::recipe_serializer::parts(handle)?;
    (!matches!(codec, Value::Ref(None))).then_some(codec)
}

fn parse_recipe_resource_id(path: &str) -> Option<String> {
    let rest = path.strip_prefix("data/")?;
    let (ns, after_ns) = rest.split_once("/recipe/")?;
    let recipe_path = after_ns.strip_suffix(".json")?;
    if ns.is_empty() || ns.contains('/') || recipe_path.is_empty() {
        return None;
    }
    normalize_id(&format!("{ns}:{recipe_path}"))
}

/// Extract a recipe id from a `ResourceKey`, enforcing that its registry is `minecraft:recipe`.
/// Returns `None` if `key` is not a modelled `ResourceKey` (caller must count), `Some(None)` if
/// `key` is null or belongs to another registry (e.g. `minecraft:item`), or `Some(Some(id))`.
fn recipe_key_id(key: &Value) -> Option<Option<String>> {
    if matches!(key, Value::Ref(None)) {
        return Some(None);
    }
    if !crate::host_ids::is_resource_key(key) {
        return None;
    }
    let reg = crate::host_ids::registry_name(&crate::host_ids::key_registry(key)?)?;
    if reg != "minecraft:recipe" {
        return Some(None);
    }
    let loc = crate::host_ids::identifier_key(&crate::host_ids::key_location(key)?)?.to_owned();
    Some(Some(loc))
}

/// Load and cache all `data/<ns>/recipe/**/*.json` resources across every loaded jar and datapack
/// layer in vanilla's pack order (higher-priority layer overrides lower-priority layer for
/// duplicate recipe ids), decoding each mod-serializer recipe through its own DFU `MapCodec`.
pub(crate) fn ensure_recipes_loaded<H: Host>(
    runtime: &mut Runtime<'_, H>,
) -> Result<(), NativeError> {
    let (fingerprint, serializer_codecs, recipe_entries) = {
        let Some(std_host) = runtime.host.as_std_host() else {
            return Ok(());
        };
        let fp = cache_fingerprint(std_host);
        if std_host
            .recipes
            .loaded
            .as_ref()
            .is_some_and(|c| c.fingerprint == fp)
        {
            return Ok(());
        }
        let mut ser_codecs: BTreeMap<String, Value> = BTreeMap::new();
        for entry in std_host.registrations() {
            if entry.registry == "minecraft:recipe_serializer"
                && let Some(codec) = serializer_codec(&entry.value)
            {
                ser_codecs.insert(entry.id.clone(), codec);
            }
        }
        // Build ordered layer map (lowest -> highest priority): later packs override earlier packs
        // for duplicate recipe ids while retaining source-pack provenance.
        let mut by_id: BTreeMap<String, (String, Vec<u8>)> = BTreeMap::new();
        for (pack_id, layer) in std_host.ordered_data_layers() {
            for (path, bytes) in layer {
                if let Some(id) = parse_recipe_resource_id(path) {
                    by_id.insert(id, (pack_id.to_owned(), bytes.clone()));
                }
            }
        }
        (fp, ser_codecs, by_id)
    };

    let mut loaded_recipes = Vec::new();
    let mut errors = Vec::new();
    let mut unmodelled_serializers = BTreeSet::new();
    let mut skipped_recipe_ids = BTreeSet::new();

    for (recipe_id, (source_pack, bytes)) in recipe_entries {
        let parsed: serde_json::Value = match serde_json::from_slice(&bytes) {
            Ok(v) => v,
            Err(err) => {
                errors.push((
                    recipe_id,
                    format!("[{source_pack}] invalid recipe JSON: {err}"),
                ));
                continue;
            }
        };
        let Some(obj) = parsed.as_object() else {
            errors.push((
                recipe_id,
                format!("[{source_pack}] recipe JSON root is not an object"),
            ));
            continue;
        };
        let Some(raw_type) = obj.get("type").and_then(serde_json::Value::as_str) else {
            errors.push((
                recipe_id,
                format!("[{source_pack}] missing recipe serializer type"),
            ));
            continue;
        };
        let Some(norm_type) = normalize_id(raw_type) else {
            errors.push((
                recipe_id,
                format!("[{source_pack}] invalid recipe serializer id: {raw_type}"),
            ));
            continue;
        };
        let Some(map_codec) = serializer_codecs.get(&norm_type) else {
            skipped_recipe_ids.insert(recipe_id);
            if !vanilla_item_data::is_vanilla_recipe_serializer(&norm_type) {
                unmodelled_serializers.insert(norm_type);
            }
            continue;
        };

        let before_decode = runtime.host.counted_unknown_calls();
        let decode_result = crate::dfu_codec::decode_map_json(runtime, map_codec, obj)?;
        let decode_counted = runtime.host.counted_unknown_calls() != before_decode;
        let outcome = if decode_counted {
            Err(DecodeError::Unmodelled(
                "counted unknown call during recipe decode".into(),
            ))
        } else {
            decode_result
        };

        match outcome {
            Ok(recipe_value) => {
                let before_type = runtime.host.counted_unknown_calls();
                let recipe_type = runtime.call_virtual(
                    crate::jdk_call::Site::Interface,
                    "net/minecraft/world/item/crafting/Recipe",
                    "getType",
                    "()Lnet/minecraft/world/item/crafting/RecipeType;",
                    &recipe_value,
                    &[],
                )?;
                if runtime.host.counted_unknown_calls() != before_type {
                    let _ = runtime.host.unknown_operand_call(&MethodRef {
                        owner: RECIPE_MANAGER.into(),
                        name: "decodeRecipe".into(),
                        descriptor: format!("({norm_type})V"),
                    });
                    unmodelled_serializers.insert(norm_type);
                    skipped_recipe_ids.insert(recipe_id.clone());
                    errors.push((
                        recipe_id,
                        format!("[{source_pack}] counted unknown call inside Recipe.getType"),
                    ));
                    continue;
                }
                let key = crate::host_ids::resource_key_named("minecraft:recipe", &recipe_id);
                let holder = {
                    let Some(std_host) = runtime.host.as_std_host_mut() else {
                        return Ok(());
                    };
                    std_host.recipes.alloc_holder(key, recipe_value.clone())
                };
                loaded_recipes.push(LoadedRecipe {
                    id: recipe_id,
                    source_pack,
                    recipe_type,
                    recipe: recipe_value,
                    holder,
                });
            }
            Err(DecodeError::Invalid(msg)) => {
                errors.push((recipe_id, format!("[{source_pack}] {msg}")));
            }
            Err(DecodeError::Unmodelled(msg)) => {
                let _ = runtime.host.unknown_operand_call(&MethodRef {
                    owner: RECIPE_MANAGER.into(),
                    name: "decodeRecipe".into(),
                    descriptor: format!("({norm_type})V"),
                });
                unmodelled_serializers.insert(norm_type);
                skipped_recipe_ids.insert(recipe_id.clone());
                errors.push((
                    recipe_id,
                    format!("[{source_pack}] unmodelled codec: {msg}"),
                ));
            }
        }
    }

    if let Some(std_host) = runtime.host.as_std_host_mut() {
        std_host.recipes.loaded = Some(LoadedRecipeCache {
            fingerprint,
            recipes: loaded_recipes,
            errors,
            unmodelled_serializers,
            skipped_recipe_ids,
        });
    }
    Ok(())
}

/// Dispatch `RecipeManager` / `RecipeAccess` methods on the real `game/recipe-manager` handle.
/// A bare `new RecipeManager()` (not `game/recipe-manager`) returns `Ok(None)` so it stays counted.
pub(crate) fn runtime_recipe_manager_call<H: Host>(
    runtime: &mut Runtime<'_, H>,
    r: &MethodRef,
    receiver: Option<&Value>,
    args: &[Value],
) -> Result<Option<Value>, NativeError> {
    let Some(receiver) = receiver else {
        return Ok(None);
    };
    if crate::game_object::handle_key(receiver) != Some(RECIPE_MANAGER_KEY) {
        return Ok(None);
    }
    if r.owner != RECIPE_MANAGER && r.owner != RECIPE_ACCESS {
        return Ok(None);
    }
    match (r.name.as_str(), r.descriptor.as_str()) {
        (
            "getRecipeFor",
            "(Lnet/minecraft/world/item/crafting/RecipeType;Lnet/minecraft/world/item/crafting/RecipeInput;Lnet/minecraft/world/level/Level;)Ljava/util/Optional;",
        ) => {
            let [recipe_type, input, level] = args else {
                return Ok(None);
            };
            find_matching_recipe(runtime, recipe_type, input, level, None, None)
        }
        (
            "getRecipeFor",
            "(Lnet/minecraft/world/item/crafting/RecipeType;Lnet/minecraft/world/item/crafting/RecipeInput;Lnet/minecraft/world/level/Level;Lnet/minecraft/resources/ResourceKey;)Ljava/util/Optional;",
        ) => {
            let [recipe_type, input, level, last_key] = args else {
                return Ok(None);
            };
            let hint_key = (!matches!(last_key, Value::Ref(None))).then_some(last_key);
            find_matching_recipe(runtime, recipe_type, input, level, hint_key, None)
        }
        (
            "getRecipeFor",
            "(Lnet/minecraft/world/item/crafting/RecipeType;Lnet/minecraft/world/item/crafting/RecipeInput;Lnet/minecraft/world/level/Level;Lnet/minecraft/world/item/crafting/RecipeHolder;)Ljava/util/Optional;",
        ) => {
            let [recipe_type, input, level, last_holder] = args else {
                return Ok(None);
            };
            let hint_holder = (!matches!(last_holder, Value::Ref(None))).then_some(last_holder);
            find_matching_recipe(runtime, recipe_type, input, level, None, hint_holder)
        }
        ("byKey", "(Lnet/minecraft/resources/ResourceKey;)Ljava/util/Optional;") => {
            let Some(key) = args.first() else {
                return Ok(None);
            };
            let Some(wanted_opt) = recipe_key_id(key) else {
                return Ok(None);
            };
            let Some(wanted_id) = wanted_opt else {
                // Null key or non-`minecraft:recipe` ResourceKey (e.g. `minecraft:item`): never matches.
                return Ok(Some(crate::empty_optional()));
            };
            ensure_recipes_loaded(runtime)?;
            let Some(std_host) = runtime.host.as_std_host() else {
                return Ok(None);
            };
            let cache = std_host.recipes.loaded.as_ref();
            if let Some(holder) = cache.and_then(|c| {
                c.recipes
                    .iter()
                    .find(|rec| rec.id == wanted_id)
                    .map(|rec| rec.holder.clone())
            }) {
                return Ok(Some(crate::optional_wrap(holder)));
            }
            // Never return an uncounted empty Optional for vanilla recipes or skipped/unmodelled
            // recipes whose serializer we could not decode.
            if wanted_id.starts_with("minecraft:")
                || cache.is_some_and(|c| c.skipped_recipe_ids.contains(&wanted_id))
            {
                return Ok(None);
            }
            Ok(Some(crate::empty_optional()))
        }
        _ => Ok(None),
    }
}

fn objects_equals_exact<H: Host>(
    runtime: &mut Runtime<'_, H>,
    a: &Value,
    b: &Value,
) -> Result<Option<bool>, NativeError> {
    let before = runtime.host.counted_unknown_calls();
    let equal = runtime.objects_equals(a, b)?;
    if runtime.host.counted_unknown_calls() != before {
        return Ok(None);
    }
    Ok(Some(equal))
}

fn recipe_matches_exact<H: Host>(
    runtime: &mut Runtime<'_, H>,
    recipe: &Value,
    input: &Value,
    level: &Value,
) -> Result<Option<bool>, NativeError> {
    let before = runtime.host.counted_unknown_calls();
    let result = runtime.call_virtual(
        crate::jdk_call::Site::Interface,
        "net/minecraft/world/item/crafting/Recipe",
        "matches",
        "(Lnet/minecraft/world/item/crafting/RecipeInput;Lnet/minecraft/world/level/Level;)Z",
        recipe,
        &[input.clone(), level.clone()],
    )?;
    if runtime.host.counted_unknown_calls() != before {
        return Ok(None);
    }
    Ok(Some(matches!(result, Value::Int(v) if v != 0)))
}

/// Evaluate `RecipeInput.isEmpty()` virtually: runs the mod's override when defined, otherwise
/// runs MC 26.3's default `RecipeInput.isEmpty()` loop over virtual `size()` and `getItem(i)`.
/// Returns `Ok(None)` if any inner call was counted or an unknown `ItemStack` was encountered.
fn recipe_input_is_empty<H: Host>(
    runtime: &mut Runtime<'_, H>,
    input: &Value,
) -> Result<Option<bool>, NativeError> {
    if matches!(input, Value::Ref(None)) {
        return Err(npe());
    }
    let Some(class_name) = crate::jar_object::object_key(input)
        .and_then(|key| key.split_once('#').map(|(cls, _)| cls.to_owned()))
    else {
        return Ok(None);
    };
    if runtime
        .jar_method_owner(&class_name, "isEmpty", "()Z")
        .is_some()
    {
        let before = runtime.host.counted_unknown_calls();
        let res = runtime.call_virtual(
            crate::jdk_call::Site::Interface,
            "net/minecraft/world/item/crafting/RecipeInput",
            "isEmpty",
            "()Z",
            input,
            &[],
        )?;
        if runtime.host.counted_unknown_calls() != before {
            return Ok(None);
        }
        return Ok(Some(matches!(res, Value::Int(v) if v != 0)));
    }
    // Default `RecipeInput.isEmpty()` from MC 26.3:
    // `for (int i = 0; i < this.size(); ++i) if (!this.getItem(i).isEmpty()) return false; return true;`
    let before_size = runtime.host.counted_unknown_calls();
    let size_val = runtime.call_virtual(
        crate::jdk_call::Site::Interface,
        "net/minecraft/world/item/crafting/RecipeInput",
        "size",
        "()I",
        input,
        &[],
    )?;
    if runtime.host.counted_unknown_calls() != before_size {
        return Ok(None);
    }
    let Value::Int(size) = size_val else {
        return Ok(None);
    };
    for i in 0..size {
        let before_item = runtime.host.counted_unknown_calls();
        let stack = runtime.call_virtual(
            crate::jdk_call::Site::Interface,
            "net/minecraft/world/item/crafting/RecipeInput",
            "getItem",
            "(I)Lnet/minecraft/world/item/ItemStack;",
            input,
            &[Value::Int(i)],
        )?;
        if runtime.host.counted_unknown_calls() != before_item {
            return Ok(None);
        }
        if matches!(stack, Value::Ref(None)) {
            return Err(npe());
        }
        let Some(empty) = runtime
            .host
            .as_std_host()
            .and_then(|h| stack_is_empty(h, &stack))
        else {
            return Ok(None);
        };
        if !empty {
            return Ok(Some(false));
        }
    }
    Ok(Some(true))
}

/// Whether `recipe_type` is a mod-registered non-vanilla `RecipeType` whose recipes are completely
/// loaded (no unmodelled or unregistered non-vanilla serializers in the datapack layers).
/// Vanilla `RecipeType`s and unknown/incomplete types return `Some(false)` so `getRecipeFor` stays
/// counted instead of returning a false empty `Optional`.
fn is_known_complete_recipe_type<H: Host>(
    runtime: &mut Runtime<'_, H>,
    recipe_type: &Value,
) -> Result<Option<bool>, NativeError> {
    if matches!(recipe_type, Value::Ref(None)) {
        return Ok(Some(false));
    }
    let (registered_types, has_unmodelled) = {
        let Some(std_host) = runtime.host.as_std_host() else {
            return Ok(None);
        };
        let types: Vec<(String, Value)> = std_host
            .registrations()
            .iter()
            .filter(|e| e.registry == "minecraft:recipe_type")
            .map(|e| (e.id.clone(), e.value.clone()))
            .collect();
        let unmodelled = std_host
            .recipes
            .loaded
            .as_ref()
            .is_some_and(|c| !c.unmodelled_serializers.is_empty());
        (types, unmodelled)
    };
    if has_unmodelled {
        return Ok(Some(false));
    }
    for (id, reg_val) in &registered_types {
        let Some(eq) = objects_equals_exact(runtime, reg_val, recipe_type)? else {
            return Ok(None);
        };
        if eq {
            let is_vanilla =
                vanilla_item_data::is_vanilla_recipe_type(id) || id.starts_with("minecraft:");
            return Ok(Some(!is_vanilla));
        }
    }
    Ok(Some(false))
}

fn find_matching_recipe<H: Host>(
    runtime: &mut Runtime<'_, H>,
    recipe_type: &Value,
    input: &Value,
    level: &Value,
    hint_key: Option<&Value>,
    hint_holder: Option<&Value>,
) -> Result<Option<Value>, NativeError> {
    ensure_recipes_loaded(runtime)?;

    // Keep vanilla / unregistered / incomplete recipe types counted rather than returning an
    // uncounted `Optional.empty()`.
    if is_known_complete_recipe_type(runtime, recipe_type)? != Some(true) {
        return Ok(None);
    }

    let candidates: Vec<LoadedRecipe> = runtime
        .host
        .as_std_host()
        .and_then(|h| h.recipes.loaded.as_ref())
        .map(|c| c.recipes.clone())
        .unwrap_or_default();

    // Exact MC 26.3 `RecipeManager.getRecipeFor(type, input, level, ResourceKey)`:
    // resolves `byKeyTyped(type, lastKey)` (checking registry `minecraft:recipe` and `getType()`)
    // then delegates to the 4-arg `RecipeHolder` overload.
    let resolved_hint_holder = if let Some(holder_val) = hint_holder {
        Some(holder_val.clone())
    } else if let Some(key_val) = hint_key {
        let Some(wanted_opt) = recipe_key_id(key_val) else {
            return Ok(None);
        };
        let mut found = None;
        if let Some(wanted_id) = wanted_opt
            && let Some(candidate) = candidates.iter().find(|c| c.id == wanted_id)
        {
            let Some(same_type) =
                objects_equals_exact(runtime, &candidate.recipe_type, recipe_type)?
            else {
                return Ok(None);
            };
            if same_type {
                found = Some(candidate.holder.clone());
            }
        }
        found
    } else {
        None
    };

    // Exact MC 26.3 4-arg `RecipeHolder` fast path: check `lastRecipe.value().matches(input, level)`.
    if let Some(holder_val) = resolved_hint_holder {
        let holder_recipe = runtime.host.as_std_host().and_then(|h| {
            let Value::Host(handle) = &holder_val else {
                return None;
            };
            (handle.kind() == HostKind::RecipeHolder)
                .then(|| h.recipes.holders.get(handle.key()).map(|d| d.value.clone()))
                .flatten()
        });
        let Some(recipe) = holder_recipe else {
            return Ok(None);
        };
        let Some(matched) = recipe_matches_exact(runtime, &recipe, input, level)? else {
            return Ok(None);
        };
        if matched {
            return Ok(Some(crate::optional_wrap(holder_val)));
        }
    }

    // Exact MC 26.3 `RecipeMap.getRecipesFor`: precheck `input.isEmpty()` before testing any recipe.
    let Some(empty) = recipe_input_is_empty(runtime, input)? else {
        return Ok(None);
    };
    if empty {
        return Ok(Some(crate::empty_optional()));
    }

    for candidate in &candidates {
        let Some(same_type) = objects_equals_exact(runtime, &candidate.recipe_type, recipe_type)?
        else {
            return Ok(None);
        };
        if !same_type {
            continue;
        }
        let Some(matched) = recipe_matches_exact(runtime, &candidate.recipe, input, level)? else {
            return Ok(None);
        };
        if matched {
            return Ok(Some(crate::optional_wrap(candidate.holder.clone())));
        }
    }
    Ok(Some(crate::empty_optional()))
}

pub(crate) fn item_stack_template_illegal_state(message: &str) -> NativeError {
    illegal_state(message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn item_tags_support_nested_vanilla_replace_optional_and_cycles() {
        // Nested vanilla tag
        let host = StdHost::new().with_resource_bytes(
            "data/demo/tags/item/stone_like.json",
            br##"{"values":["#minecraft:stone_tool_materials",{"id":"demo:missing_optional","required":false}]}"##
                .to_vec(),
        );
        let stone_like = resolve_item_tag(&host, "demo:stone_like").unwrap();
        assert!(stone_like.contains("minecraft:cobblestone"));
        assert!(stone_like.contains("minecraft:blackstone"));
        assert!(stone_like.contains("minecraft:cobbled_deepslate"));
        assert!(!stone_like.contains("minecraft:dirt"));

        // Replace vanilla tag cleanly
        let host = host.with_resource_bytes(
            "data/minecraft/tags/item/stone_tool_materials.json",
            br#"{"replace":true,"values":["minecraft:granite"]}"#.to_vec(),
        );
        let replaced = resolve_item_tag(&host, "demo:stone_like").unwrap();
        assert_eq!(replaced.len(), 1);
        assert!(replaced.contains("minecraft:granite"));

        // Cycle detection terminates cleanly
        let host = host
            .with_resource_bytes(
                "data/demo/tags/item/cycle_a.json",
                br##"{"values":["#demo:cycle_b","minecraft:stick"]}"##.to_vec(),
            )
            .with_resource_bytes(
                "data/demo/tags/item/cycle_b.json",
                br##"{"values":["#demo:cycle_a"]}"##.to_vec(),
            );
        let cyclic = resolve_item_tag(&host, "demo:cycle_a").unwrap();
        assert!(cyclic.contains("minecraft:stick"));

        // Missing required tag entry fails resolution
        let host = host.with_resource_bytes(
            "data/demo/tags/item/broken.json",
            br#"{"values":["demo:does_not_exist"]}"#.to_vec(),
        );
        assert!(resolve_item_tag(&host, "demo:broken").is_none());
    }

    #[test]
    fn component_patch_overlay_and_template_max_stack_validation() {
        let mut host = StdHost::new();
        let mut base = ComponentPatch::default();
        base.entries.insert(
            "minecraft:repair_cost".into(),
            Some(Value::Boxed(Boxed::Integer(5))),
        );
        base.entries.insert(
            "minecraft:custom_name".into(),
            Some(crate::component_value("literal", "Old")),
        );
        let mut tpl_patch = ComponentPatch::default();
        tpl_patch.entries.insert(
            "minecraft:repair_cost".into(),
            Some(Value::Boxed(Boxed::Integer(11))),
        );
        let tpl = host.recipes.alloc_template(TemplateData {
            item_id: "minecraft:ender_pearl".into(),
            count: 16,
            patch: tpl_patch,
        });
        let Value::Host(tpl_handle) = &tpl else {
            panic!("expected template handle");
        };
        let valid = template_apply(&mut host, tpl_handle.key(), 16, &base).unwrap();
        assert_eq!(stack_is_empty(&host, &valid), Some(false));
        let merged = stack_patch_clone(&host, &valid).unwrap();
        assert_eq!(
            merged.entries.get("minecraft:repair_cost"),
            Some(&Some(Value::Boxed(Boxed::Integer(11))))
        );
        assert!(merged.entries.contains_key("minecraft:custom_name"));

        // Count > 16 for ender_pearl returns ItemStack.EMPTY
        let overflow = template_apply(&mut host, tpl_handle.key(), 17, &base).unwrap();
        assert!(is_empty_stack_handle(&overflow));
    }
}
