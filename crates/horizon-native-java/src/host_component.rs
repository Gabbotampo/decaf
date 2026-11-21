//! Chat components as host handles (#156 P5a): what `Component.literal`/`translatable`, a
//! `MutableComponent`'s `append`/`withStyle`/`copy` and the façade's `getDisplayName`s answer.
//! They were the text `component/literal/<text>`, `component/translatable/<key>` and
//! `component/json/<json>`, which a String could spell: `"component/literal/hi"` printed, sent and
//! compared as a component.
//!
//! **Key.** `MutableComponent#<serial>:<json>`: a serial minted once per allocation,
//! process-wide, then the JSON the component was made with (vanilla's: `text`/`translate`+`with`,
//! style flags, `extra`), every object's keys sorted. The marker before the first `:` is the
//! runtime's own text; the JSON is read back with a JSON parser, so jar text in it is escaped,
//! never split.
//!
//! **State.** A `MutableComponent` changes in place: `append` adds a sibling and `withStyle`/
//! `withColor`/`setStyle` replace the style, each returning `this` (`MutableComponent.append`
//! @1–12: `siblings.add(c); return this`). A component changed once has a row in its host's
//! [`States`] (`StdHost`'s table, kept and collected with the resident heap like a mutable
//! collection's elements, `heap.rs`): its contents and style as JSON, and its appended siblings
//! and component arguments as the objects they are, so a sibling changed later shows in its
//! parent. Every reader (`getString`, `equals`, `toString`, a command's feedback, the client
//! surface) resolves the state when it reads, as Java does when it sends or prints; a host
//! without the table (a test host) counts the change instead of answering it.
//!
//! **Class.** NeoForge's and vanilla 26.3's `MutableComponent` (`javap -p`): `final`, a
//! package-private constructor, `implements Component` (`extends Message, FormattedText`). Every
//! component the model makes is one, so the class is exact (`host_ref::ClassSide`). It is a game
//! class a Mixin may target: an interface a loaded Mixin plan adds is `true`, another interface is
//! counted, a class other than its own and `Object` is a proved `false`.
//!
//! **Object methods** (`MutableComponent`, 26.3 bytecode): `equals` compares contents, style and
//! siblings, as they are now. A component equals itself; two trees of plain texts and bare keys
//! (no style or arguments, any siblings) are equal exactly when their JSON is. Anything else is
//! counted: the JSON prints some different values one way (a translation argument `1` and `1L`)
//! and some equal ones two ways (a colour by name or by value).
//! `hashCode` is `31 * (31 * (31 + contents) + style) + siblings`: exact for a plain tree (its
//! value computed from the state now; an empty text is `PlainTextContents.EMPTY`, an identity
//! hash), counted otherwise. A hash collection keeps the hash an element had when it was added
//! (`hash_table.rs`), so a component changed after it went in is missed by `contains`/`get`, as in
//! Java; one whose hash is unknown makes a lookup among components counted. `toString` is the
//! contents' text, then `[style=…, siblings=…]` when there is either: exact for a bare text
//! (`literal{hi}`, `empty` for `""`) or a bare key (`translation{key='k', args=[]}`), counted
//! otherwise.
//!
//! **Bounds (departure).** A read resolves at most 64 KiB of rendered JSON and 512 levels; past
//! either (a component grown huge, or a cycle such as `a.append(a)`, where Java throws
//! `StackOverflowError` when it reads), the read is counted instead of answered. A change is not
//! measured, so `a.append(a)` itself succeeds, as in Java.
//!
//! **Identity.** Every factory call makes a new object (`Component.literal("a")` twice is two, and
//! `copy` another), and each gets its own serial: one key is one object (canonical), so `==` is
//! exact.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::{Map, Value as Json};

use crate::host_ref::{
    ClassHierarchy, ClassSide, HostKind, HostRef, Methods, Nested, exact_class_rules_out,
};
use crate::{NativeError, Value};

pub(crate) const MUTABLE: &str = "net/minecraft/network/chat/MutableComponent";
const COMPONENT: &str = "net/minecraft/network/chat/Component";
const MESSAGE: &str = "com/mojang/brigadier/Message";
const FORMATTED_TEXT: &str = "net/minecraft/network/chat/FormattedText";
const OBJECT: &str = "java/lang/Object";

/// `json` as text with every object's keys sorted: one content, one key.
fn canonical(json: &Json) -> String {
    match json {
        Json::Object(object) => {
            let mut keys: Vec<&String> = object.keys().collect();
            keys.sort();
            let fields: Vec<String> = keys
                .into_iter()
                .map(|key| {
                    format!(
                        "{}:{}",
                        Json::String(key.clone()),
                        canonical(&object[key.as_str()])
                    )
                })
                .collect();
            format!("{{{}}}", fields.join(","))
        }
        Json::Array(items) => format!(
            "[{}]",
            items.iter().map(canonical).collect::<Vec<_>>().join(",")
        ),
        other => other.to_string(),
    }
}

static NEXT_SERIAL: AtomicU64 = AtomicU64::new(0);

/// A component's longest resolved JSON: a callback appending in a loop cannot grow one without
/// end (the bound the marker model had).
pub(crate) const MAX_JSON_BYTES: usize = 64 * 1024;
/// How deep siblings and arguments resolve: deeper (or a cycle, `a.append(a)`, which Java
/// answers with a `StackOverflowError` when the component is read) is unknown, so the read is
/// counted. Java nests until its stack runs out, a few thousand levels.
const MAX_DEPTH: usize = 512;
/// The siblings one component may hold: an `append` past it is refused (a loop appending without
/// end), the one bound a change itself checks.
const MAX_SIBLINGS: usize = 16 * 1024;

/// What a component holds once it has been changed in place: its contents and style as vanilla
/// JSON, then the siblings appended since and the translation arguments, as the objects they
/// are (a sibling changed later shows in its parent, as in Java).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ComponentState {
    /// Contents and style; its own `extra` (a component decoded from JSON) comes first.
    pub(crate) base: Map<String, Json>,
    /// Siblings appended in place (`MutableComponent.append`), each a component handle.
    pub(crate) siblings: Vec<Value>,
    /// The translation arguments as values, when one is a component (`with` resolves them).
    pub(crate) args: Option<Vec<Value>>,
}

/// The host's table of changed components, keyed by [`marker`].
pub type States = BTreeMap<String, ComponentState>;

/// A new component of `json` (a fresh object, as every factory makes).
pub(crate) fn component(json: &Json) -> Value {
    let serial = NEXT_SERIAL.fetch_add(1, Ordering::Relaxed);
    Value::Host(HostRef::new(
        HostKind::Component,
        format!("{MUTABLE}#{serial}:{}", canonical(json)),
    ))
}

/// The handle of `value` when it is a component.
fn handle_of(value: &Value) -> Option<&HostRef> {
    match value {
        Value::Host(handle) if handle.kind() == HostKind::Component => Some(handle),
        _ => None,
    }
}

/// The marker naming one component's allocation, `MutableComponent#<serial>`: its row in the
/// host's [`States`], and what the resident heap's collector finds in the key
/// (`heap.rs`). The serial is the runtime's digits, so the key splits at its first `:`.
pub(crate) fn marker(handle: &HostRef) -> &str {
    handle
        .key()
        .split_once(':')
        .map_or(handle.key(), |(marker, _)| marker)
}

/// The JSON a component was made with (after its marker).
fn key_json(key: &str) -> Option<Json> {
    let (_, json) = key.split_once(':')?;
    serde_json::from_str(json).ok()
}

/// `Component.literal(text)`.
pub(crate) fn literal(text: &str) -> Value {
    component(&Json::Object(Map::from_iter([(
        "text".to_owned(),
        Json::String(text.to_owned()),
    )])))
}

/// `Component.translatable(key)`.
pub(crate) fn translatable(key: &str) -> Value {
    component(&Json::Object(Map::from_iter([(
        "translate".to_owned(),
        Json::String(key.to_owned()),
    )])))
}

/// Whether `value` is a component handle.
pub(crate) fn is_component(value: &Value) -> bool {
    handle_of(value).is_some()
}

/// A component's state now: its row in `states`, or what it was made with.
pub(crate) fn state_of(value: &Value, states: Option<&States>) -> Option<ComponentState> {
    let handle = handle_of(value)?;
    if let Some(state) = states.and_then(|states| states.get(marker(handle))) {
        return Some(state.clone());
    }
    let base = match key_json(handle.key())? {
        Json::Object(object) => object,
        Json::String(text) => Map::from_iter([("text".to_owned(), Json::String(text))]),
        other => Map::from_iter([("text".to_owned(), Json::String(other.to_string()))]),
    };
    Some(ComponentState {
        base,
        ..ComponentState::default()
    })
}

/// `Style$Serializer.MAP_CODEC` field order (Minecraft 26.3 bytecode).
pub(crate) const CODEC_STYLE_KEYS: [&str; 11] = [
    "color",
    "shadow_color",
    "bold",
    "italic",
    "underlined",
    "strikethrough",
    "obfuscated",
    "click_event",
    "hover_event",
    "insertion",
    "font",
];

/// Reorder a component object's JSON members into Minecraft 26.3's
/// `ComponentSerialization.CODEC` order: content fields (`text`, or `translate`
/// / `fallback` / `with`), then `extra`, then `Style$Serializer` fields.
fn order_codec_fields(
    mut base: Map<String, Json>,
    with: Option<Vec<Json>>,
    extra: Option<Vec<Json>>,
) -> Map<String, Json> {
    let mut ordered = Map::new();
    if let Some(text) = base.remove("text") {
        ordered.insert("text".into(), text);
    }
    if let Some(translate) = base.remove("translate") {
        ordered.insert("translate".into(), translate);
    }
    if let Some(fallback) = base.remove("fallback") {
        ordered.insert("fallback".into(), fallback);
    }
    let existing_with = base.remove("with");
    if let Some(with) = with {
        if !with.is_empty() {
            ordered.insert("with".into(), Json::Array(with));
        }
    } else if let Some(Json::Array(with)) = existing_with
        && !with.is_empty()
    {
        ordered.insert("with".into(), Json::Array(with));
    }
    base.remove("extra");
    let mut style = Vec::new();
    for key in CODEC_STYLE_KEYS {
        if let Some(value) = base.remove(key) {
            style.push((key, value));
        }
    }
    // Any other content fields (e.g. from decoded JSON) stay in the content section.
    for (key, value) in base {
        ordered.insert(key, value);
    }
    if let Some(extra) = extra
        && !extra.is_empty()
    {
        ordered.insert("extra".into(), Json::Array(extra));
    }
    for (key, value) in style {
        ordered.insert(key.into(), value);
    }
    ordered
}

/// Recursively collapse plain literal JSON objects (`{"text": s}` with no style
/// or siblings) and order members in vanilla codec order, for pre-existing JSON
/// inside `with` or `extra` of a decoded component.
fn normalize_codec_json(json: Json) -> Json {
    match json {
        Json::Object(mut object) => {
            let with = match object.remove("with") {
                Some(Json::Array(items)) => {
                    Some(items.into_iter().map(normalize_codec_json).collect())
                }
                _ => None,
            };
            let extra = match object.remove("extra") {
                Some(Json::Array(items)) => {
                    Some(items.into_iter().map(normalize_codec_json).collect())
                }
                _ => None,
            };
            let ordered = order_codec_fields(object, with, extra);
            if ordered.len() == 1
                && let Some(Json::String(text)) = ordered.get("text")
            {
                return Json::String(text.clone());
            }
            Json::Object(ordered)
        }
        other => other,
    }
}

fn resolve_component(
    value: &Value,
    states: Option<&States>,
    argument: &dyn Fn(&Value) -> Option<Json>,
    collapse_plain: bool,
    depth: usize,
    budget: &mut usize,
) -> Option<Json> {
    if depth > MAX_DEPTH {
        return None;
    }
    let state = state_of(value, states)?;
    // The node's own contents and style, as written (its children are charged below).
    let own: usize = state
        .base
        .iter()
        .filter(|(key, _)| !matches!(key.as_str(), "extra" | "with"))
        .map(|(key, value)| key.len() + value.to_string().len() + 4)
        .sum();
    *budget = budget.checked_sub(own + 2)?;
    let mut base = state.base;
    let with = if let Some(args) = &state.args {
        if args.is_empty() {
            None
        } else {
            let mut with = Vec::with_capacity(args.len());
            for arg in args {
                with.push(if is_component(arg) {
                    resolve_component(arg, states, argument, collapse_plain, depth + 1, budget)?
                } else {
                    let arg = argument(arg)?;
                    *budget = budget.checked_sub(arg.to_string().len() + 1)?;
                    arg
                });
            }
            Some(with)
        }
    } else if let Some(existing) = base.get("with") {
        *budget = budget.checked_sub(existing.to_string().len())?;
        if collapse_plain && let Some(Json::Array(items)) = base.remove("with") {
            Some(items.into_iter().map(normalize_codec_json).collect())
        } else {
            None
        }
    } else {
        None
    };
    if let Some(extra) = base.get("extra") {
        *budget = budget.checked_sub(extra.to_string().len())?;
    }
    let mut extra = match base.remove("extra") {
        Some(Json::Array(existing)) => {
            if collapse_plain {
                existing.into_iter().map(normalize_codec_json).collect()
            } else {
                existing
            }
        }
        _ => Vec::new(),
    };
    for sibling in &state.siblings {
        extra.push(resolve_component(
            sibling,
            states,
            argument,
            collapse_plain,
            depth + 1,
            budget,
        )?);
    }
    let extra = (!extra.is_empty()).then_some(extra);
    let ordered = order_codec_fields(base, with, extra);
    if collapse_plain
        && ordered.len() == 1
        && let Some(Json::String(text)) = ordered.get("text")
    {
        return Some(Json::String(text.clone()));
    }
    Some(Json::Object(ordered))
}

/// The structured object JSON `value` stands for now, its siblings and component
/// arguments as they are now (each component node kept as a JSON object for
/// internal readers such as `plain_tree`, `tree_hash`, and `bare_literal`);
/// `None` for any other value, an unsupported translation argument, or a tree
/// past [`MAX_JSON_BYTES`] as rendered or nested past [`MAX_DEPTH`].
pub(crate) fn json_in(
    value: &Value,
    states: Option<&States>,
    argument: &dyn Fn(&Value) -> Option<Json>,
) -> Option<Json> {
    let mut budget = MAX_JSON_BYTES;
    resolve_component(value, states, argument, false, 0, &mut budget)
}

/// The vanilla `ComponentSerialization.CODEC` + `JsonOps.INSTANCE` JSON `value`
/// encodes to now: plain literal components (no style, no siblings) collapse to
/// JSON strings at every level (`Component.tryCollapseToString`), and object
/// members follow vanilla's codec order (content fields, then `extra`, then
/// `Style$Serializer` fields). `None` for non-components, unsupported
/// translation arguments, or trees past the resolution bounds.
pub(crate) fn codec_json_in(
    value: &Value,
    states: Option<&States>,
    argument: &dyn Fn(&Value) -> Option<Json>,
) -> Option<Json> {
    let mut budget = MAX_JSON_BYTES;
    resolve_component(value, states, argument, true, 0, &mut budget)
}

/// Change the component `receiver` in place: `change` its state, then keep it in `states`. The
/// change itself is not measured (a read is, [`json_in`]): `a.append(a)` succeeds, as in Java. The
/// error names a component whose own contents grew past [`MAX_JSON_BYTES`] or that holds more
/// than [`MAX_SIBLINGS`] siblings (the change is undone).
pub(crate) fn change(
    states: &mut States,
    receiver: &Value,
    _argument: &dyn Fn(&Value) -> Option<Json>,
    change: impl FnOnce(&mut ComponentState),
) -> Result<(), NativeError> {
    let (Some(handle), Some(mut state)) = (handle_of(receiver), state_of(receiver, Some(states)))
    else {
        return Err(NativeError::Metadata("not a chat component".into()));
    };
    change(&mut state);
    if state.siblings.len() > MAX_SIBLINGS
        || Json::Object(state.base.clone()).to_string().len() > MAX_JSON_BYTES
    {
        return Err(NativeError::Metadata(format!(
            "a chat component grew past {MAX_JSON_BYTES} bytes or {MAX_SIBLINGS} siblings"
        )));
    }
    states.insert(marker(handle).to_owned(), state);
    Ok(())
}

/// `MutableComponent.copy()` (`new MutableComponent(contents, new ArrayList<>(siblings),
/// style)`) or, with `plain`, `plainCopy()` (the contents alone): a new object whose siblings
/// and arguments are the receiver's own objects.
pub(crate) fn copy(states: Option<&mut States>, receiver: &Value, plain: bool) -> Option<Value> {
    let mut state = state_of(receiver, states.as_deref())?;
    if plain {
        state
            .base
            .retain(|key, _| matches!(key.as_str(), "text" | "translate" | "with" | "fallback"));
        state.siblings.clear();
    }
    let made = component(&Json::Object(state.base.clone()));
    if let (Some(states), Some(handle)) = (states, handle_of(&made))
        && (!state.siblings.is_empty() || state.args.is_some())
    {
        states.insert(marker(handle).to_owned(), state);
    }
    Some(made)
}

/// The one string field a bare component holds (`{"text": …}` or `{"translate": …}`).
fn bare(json: &Json, field: &str) -> Option<String> {
    match json {
        Json::Object(object) if object.len() == 1 => match object.get(field) {
            Some(Json::String(text)) => Some(text.clone()),
            _ => None,
        },
        _ => None,
    }
}

/// Whether `json` is a component of literal texts and bare translation keys alone: each node an
/// object with one `text` or `translate` string and, optionally, an `extra` array of such nodes
/// (no style, arguments, fallback or other contents). Two such trees are equal in Java
/// (`MutableComponent.equals`: contents, an empty style, siblings in order) exactly when their
/// JSON is.
fn plain_tree(json: &Json) -> bool {
    let Json::Object(object) = json else {
        return false;
    };
    let contents = ["text", "translate"]
        .iter()
        .filter(|key| matches!(object.get(**key), Some(Json::String(_))))
        .count();
    contents == 1
        && object
            .keys()
            .all(|key| matches!(key.as_str(), "text" | "translate" | "extra"))
        && match object.get("extra") {
            None => true,
            Some(Json::Array(extra)) => extra.iter().all(plain_tree),
            Some(_) => false,
        }
}

/// The text of a bare literal component's JSON (no style, no siblings).
pub(crate) fn bare_literal(json: &Json) -> Option<String> {
    bare(json, "text")
}

/// The key of a bare translatable component's JSON (no arguments, style or siblings).
pub(crate) fn bare_key(json: &Json) -> Option<String> {
    bare(json, "translate")
}

/// `MutableComponent.toString` where it follows from the JSON: `literal{text}` and
/// `translation{key='k', args=[]}` with no style and no siblings.
fn component_to_string(
    handle: &HostRef,
    nested: &mut dyn Nested,
) -> Result<Option<String>, NativeError> {
    let Some(json) = nested.component_json(&Value::Host(handle.clone())) else {
        return Ok(None);
    };
    // `PlainTextContents.create("")` is `EMPTY`, whose `toString` is `empty`.
    Ok(bare(&json, "text")
        .map(|text| {
            if text.is_empty() {
                "empty".to_owned()
            } else {
                format!("literal{{{text}}}")
            }
        })
        .or_else(|| {
            bare(&json, "translate").map(|key| format!("translation{{key='{key}', args=[]}}"))
        }))
}

/// `Style.EMPTY.hashCode()`: `Objects.hash` of its ten null fields, `31^10`.
const EMPTY_STYLE_HASH: i32 = 31_i32
    .wrapping_mul(31)
    .wrapping_mul(31)
    .wrapping_mul(31)
    .wrapping_mul(31)
    .wrapping_mul(31)
    .wrapping_mul(31)
    .wrapping_mul(31)
    .wrapping_mul(31)
    .wrapping_mul(31);

/// `MutableComponent.hashCode()` of a plain tree ([`plain_tree`]): `31 * (31 * (31 +
/// contents) + style) + siblings` (26.3 bytecode), the contents a `LiteralContents` record
/// (its text's hash) or a `TranslatableContents` without arguments (`(31 * key + 0) * 31 +
/// Arrays.hashCode(NO_ARGS)`), the style `Style.EMPTY`'s, the siblings `List.hashCode`. An
/// empty text is `PlainTextContents.EMPTY`, an identity hash: `None`.
fn tree_hash(json: &Json) -> Option<i32> {
    let Json::Object(object) = json else {
        return None;
    };
    let contents = match (object.get("text"), object.get("translate")) {
        (Some(Json::String(text)), None) if !text.is_empty() => {
            crate::jdk_model::java_string_hash(text)
        }
        (None, Some(Json::String(key))) => crate::jdk_model::java_string_hash(key)
            .wrapping_mul(31)
            .wrapping_mul(31)
            .wrapping_add(1),
        _ => return None,
    };
    let mut siblings = 1_i32;
    if let Some(Json::Array(extra)) = object.get("extra") {
        for sibling in extra {
            siblings = siblings.wrapping_mul(31).wrapping_add(tree_hash(sibling)?);
        }
    }
    Some(
        31_i32
            .wrapping_add(contents)
            .wrapping_mul(31)
            .wrapping_add(EMPTY_STYLE_HASH)
            .wrapping_mul(31)
            .wrapping_add(siblings),
    )
}

/// `hashCode` of a component as it is now: exact for a plain tree, counted otherwise.
fn component_hash(handle: &HostRef, nested: &mut dyn Nested) -> Result<Option<i32>, NativeError> {
    Ok(nested
        .component_json(&Value::Host(handle.clone()))
        .filter(plain_tree)
        .and_then(|json| {
            if json == serde_json::json!({"text": ""}) {
                None
            } else {
                tree_hash(&json)
            }
        }))
}

/// `MutableComponent.equals`: contents, style and siblings.
fn component_equals(
    handle: &HostRef,
    other: &Value,
    nested: &mut dyn Nested,
) -> Result<Option<bool>, NativeError> {
    let Some(other_handle) = handle_of(other) else {
        return Ok(Some(false));
    };
    if handle == other_handle {
        return Ok(Some(true));
    }
    let (Some(a), Some(b)) = (
        nested.component_json(&Value::Host(handle.clone())),
        nested.component_json(other),
    ) else {
        return Ok(None);
    };
    Ok((plain_tree(&a) && plain_tree(&b)).then_some(a == b))
}

/// `instanceof` for an object of exactly `MutableComponent`.
fn class_instance_of(class: &str, wanted: &str, hierarchy: &dyn ClassHierarchy) -> Option<bool> {
    if [OBJECT, MUTABLE, COMPONENT, MESSAGE, FORMATTED_TEXT].contains(&wanted)
        || hierarchy.is_instance(class, wanted) == Some(true)
    {
        return Some(true);
    }
    (hierarchy.is_interface(wanted) == Some(false) || exact_class_rules_out(class, wanted))
        .then_some(false)
}

/// A component is a game object of the jar's runtime (its IMC crossing is unchanged: the markers
/// did not cross either).
fn stays_in_its_jar(_: &HostRef, _: &dyn Fn(&Value) -> bool) -> bool {
    false
}

pub(crate) const COMPONENT_METHODS: Methods = Methods {
    to_string: component_to_string,
    hash_code: component_hash,
    equals: component_equals,
    instance_of: |_, wanted, hierarchy| class_instance_of(MUTABLE, wanted, hierarchy),
    crosses_jars: stays_in_its_jar,
    compare_to: None,
};

pub(crate) fn component_class(_: &str) -> &str {
    MUTABLE
}

pub(crate) const COMPONENT_CLASS_SIDE: ClassSide = ClassSide {
    owns: |class| class == MUTABLE,
    instance_of: class_instance_of,
    superclass: |_| Some(OBJECT.to_owned()),
    is_interface: |_| Some(false),
    classes: || vec![MUTABLE.to_owned()],
};

#[cfg(test)]
#[path = "host_component_tests.rs"]
mod tests;
