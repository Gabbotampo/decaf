//! Chat components that keep their style and translation arguments.
//!
//! The interpreter's first component model was text only: a literal's text
//! and a translatable's key. That is
//! enough to read a mod's name, but command feedback lost what made it
//! readable: `Component.translatable("commands.demo.set", value)` arrived as
//! the bare key with its `%s` unfilled, and `withStyle(ChatFormatting.RED)`
//! arrived white.
//!
//! A component is a host handle (`host_component.rs`) whose key is vanilla's
//! own component JSON
//! (`ComponentSerialization`: `text`/`translate`+`with`, `color`, `bold`,
//! `italic`, `underlined`, `strikethrough`, `obfuscated`, `extra`), so the game
//! hands it to the same decoder a server's chat packet goes through and a
//! vanilla client gets it unchanged.
//!
//! The calls answered here run in the interpreter
//! ([`Runtime::invoke_text`]) because two of them take a varargs array the
//! interpreter owns (`translatable(key, Object...)`,
//! `withStyle(ChatFormatting...)`):
//!
//! - `Component.translatable(key, args...)`, `translatableWithFallback(key,
//!   fallback, args...)`, `translatableEscape(key, args...)`: arguments as
//!   vanilla serialises them: a component as itself, a `String`, number or
//!   boolean as a JSON primitive, anything else as its text.
//! - `withStyle(ChatFormatting)`, `withStyle(ChatFormatting...)`,
//!   `withColor(int)`: vanilla's `Style.applyFormat` (a colour replaces the
//!   colour, a decoration sets its flag, `RESET` clears the style).
//! - `append(Component|String)`: a sibling in `extra`, which inherits the
//!   parent's style as in vanilla.
//! - `copy()`, `plainCopy()` (the root's contents only, unstyled) and
//!   `getString()` (the text with translation keys as they are: the
//!   interpreter has no language table, as before).
//!
//! A style object (`Style.EMPTY.withColor(...)`, `TextColor`) is not
//! modelled: those calls still reach the host's surface and refuse there.

use serde_json::{Map, Value as Json, json};

use crate::runtime::host_component::{CODEC_STYLE_KEYS, ComponentState, MAX_JSON_BYTES, States};
use crate::runtime::{Boxed, Host, MethodRef, NativeError, Runtime, Value, expect_ref};

const COMPONENT: &str = "net/minecraft/network/chat/Component";
const MUTABLE: &str = "net/minecraft/network/chat/MutableComponent";
const FORMATTING_PREFIX: &str = "enum/net/minecraft/ChatFormatting/";
const RETURNS_MUTABLE: &str = ")Lnet/minecraft/network/chat/MutableComponent;";

/// The structured object JSON a component holds now, its changed siblings and
/// arguments resolved through `states` (`host_component.rs`), or `None` when
/// `value` is not a component.
pub(crate) fn json_now(value: &Value, states: Option<&States>) -> Option<Json> {
    crate::runtime::host_component::json_in(value, states, &argument)
}

/// Vanilla's `ComponentSerialization.CODEC` + `JsonOps.INSTANCE` JSON for
/// `value` (plain literals collapsed to strings; fields in codec order).
pub(crate) fn codec_json_now(value: &Value, states: Option<&States>) -> Option<Json> {
    crate::runtime::host_component::codec_json_in(value, states, &argument)
}

/// A new component of `json`.
pub(crate) fn from_json(json: &Json) -> Value {
    crate::runtime::host_component::component(json)
}

/// Whether a component's JSON holds a translation anywhere (its text then
/// depends on the language, which the interpreter does not hold).
pub(crate) fn has_translation(json: &Json) -> bool {
    match json {
        Json::Object(object) => {
            object.contains_key("translate") || object.get("extra").is_some_and(has_translation)
        }
        Json::Array(items) => items.iter().any(has_translation),
        _ => false,
    }
}

/// `getString()`: every text in order, a translation as its key (the
/// interpreter has no language table).
pub(crate) fn plain(json: &Json) -> String {
    fn walk(json: &Json, out: &mut String) {
        match json {
            Json::String(text) => out.push_str(text),
            Json::Object(object) => {
                if let Some(Json::String(text)) = object.get("text") {
                    out.push_str(text);
                } else if let Some(Json::String(key)) = object.get("translate") {
                    out.push_str(key);
                }
                if let Some(Json::Array(extra)) = object.get("extra") {
                    extra.iter().for_each(|part| walk(part, out));
                }
            }
            _ => {}
        }
    }
    let mut out = String::new();
    walk(json, &mut out);
    out
}

fn is_plain_string_arg(text: &str) -> bool {
    !(text.starts_with("enum/") && !crate::runtime::value_class::is_plain_string(text))
        && !text.starts_with("string-builder/")
        && !crate::runtime::stream_pipeline::is_stream_marker(text)
}

/// Whether `value` satisfies Minecraft 26.3's
/// `TranslatableContents.isAllowedPrimitiveArgument`: `Number`, `Boolean`, or
/// `String`. Note that `Component.translatableEscape` uses this predicate
/// before falling back to `String.valueOf`, so non-finite floats remain
/// allowed primitives here and are only refused when encoding to JSON.
fn is_allowed_primitive_argument(value: &Value) -> bool {
    match value {
        Value::Int(_)
        | Value::Long(_)
        | Value::Float(_)
        | Value::Boxed(
            Boxed::Boolean(_)
            | Boxed::Byte(_)
            | Boxed::Short(_)
            | Boxed::Integer(_)
            | Boxed::Float(_),
        ) => true,
        Value::Ref(Some(text)) => is_plain_string_arg(text),
        Value::Ref(None) | Value::Boxed(Boxed::Character(_)) | Value::Host(_) | Value::Void => {
            false
        }
    }
}

/// One translation argument that is not a component, matching Minecraft 26.3's
/// `TranslatableContents.isAllowedPrimitiveArgument`: only `Number`, `Boolean`,
/// and `String` are allowed. Anything vanilla's codec rejects (`null`,
/// `Character`, arbitrary objects, non-finite floats) returns `None` so callers
/// refuse or count it at serialization time rather than stringifying it.
pub(crate) fn argument(value: &Value) -> Option<Json> {
    match value {
        Value::Int(v) => Some(json!(v)),
        Value::Long(v) => Some(json!(v)),
        Value::Float(v) => serde_json::Number::from_f64(*v).map(Json::Number),
        Value::Boxed(Boxed::Boolean(b)) => Some(json!(b)),
        Value::Boxed(Boxed::Float(bits)) => {
            serde_json::Number::from_f64(f64::from(f32::from_bits(*bits))).map(Json::Number)
        }
        Value::Boxed(Boxed::Byte(v)) => Some(json!(v)),
        Value::Boxed(Boxed::Short(v)) => Some(json!(v)),
        Value::Boxed(Boxed::Integer(v)) => Some(json!(v)),
        Value::Ref(Some(text)) if is_plain_string_arg(text) => Some(json!(text)),
        Value::Ref(_) | Value::Boxed(Boxed::Character(_)) | Value::Host(_) | Value::Void => None,
    }
}

/// `Style.applyFormat`: the JSON style keys one `ChatFormatting` sets.
fn apply_format(object: &mut Map<String, Json>, format: &str) -> Result<(), String> {
    let decoration = match format {
        "BOLD" => "bold",
        "ITALIC" => "italic",
        "UNDERLINE" => "underlined",
        "STRIKETHROUGH" => "strikethrough",
        "OBFUSCATED" => "obfuscated",
        "RESET" => {
            CODEC_STYLE_KEYS.iter().for_each(|key| {
                object.remove(*key);
            });
            return Ok(());
        }
        colour => {
            const COLOURS: [&str; 16] = [
                "BLACK",
                "DARK_BLUE",
                "DARK_GREEN",
                "DARK_AQUA",
                "DARK_RED",
                "DARK_PURPLE",
                "GOLD",
                "GRAY",
                "DARK_GRAY",
                "BLUE",
                "GREEN",
                "AQUA",
                "RED",
                "LIGHT_PURPLE",
                "YELLOW",
                "WHITE",
            ];
            if !COLOURS.contains(&colour) {
                return Err(format!("ChatFormatting.{colour} is not a vanilla format"));
            }
            object.insert("color".into(), json!(colour.to_ascii_lowercase()));
            return Ok(());
        }
    };
    object.insert(decoration.into(), json!(true));
    Ok(())
}

fn formatting(value: &Value) -> Option<&str> {
    expect_ref(value)?.strip_prefix(FORMATTING_PREFIX)
}

fn bounded(json: Json) -> Result<Value, NativeError> {
    if json.to_string().len() > MAX_JSON_BYTES {
        return Err(NativeError::Metadata(format!(
            "a chat component grew past {MAX_JSON_BYTES} bytes"
        )));
    }
    Ok(from_json(&json))
}

impl<H: Host> Runtime<'_, H> {
    /// The JSON `value` holds now, if it is a component.
    fn component_now(&self, value: &Value) -> Option<Json> {
        json_now(value, self.host.component_states())
    }

    /// Change the component `receiver` in place and answer it (`return
    /// this`); a host that keeps no component table counts the call.
    fn change_component(
        &mut self,
        r: &MethodRef,
        receiver: &Value,
        change: impl FnOnce(&mut ComponentState),
    ) -> Result<Option<Value>, NativeError> {
        let Some(states) = self.host.component_states_mut() else {
            return self.host.unknown_operand_call(r).map(Some);
        };
        crate::runtime::host_component::change(states, receiver, &argument, change)?;
        Ok(Some(receiver.clone()))
    }

    /// `String.valueOf(value)` for `Component.translatableEscape`: runs
    /// `value.toString()` through the runtime (jar overrides, records, enums,
    /// host handles, modelled markers) and counts `Object.toString()` when
    /// unknown. Preserves `null` returned by a custom `toString()`.
    fn escape_translation_arg(
        &mut self,
        value: &Value,
        class: &str,
        method: &str,
    ) -> Result<Value, NativeError> {
        if *value == Value::Ref(None) {
            return Ok(Value::Ref(Some("null".into())));
        }
        if let Value::Boxed(boxed) = value {
            return Ok(Value::Ref(Some(boxed.text())));
        }
        if let Some(text) = self.runtime_value_of(value, class, method)? {
            return Ok(text);
        }
        if let Some(text) = self.host.modelled_to_string_checked(value)? {
            return Ok(Value::Ref(Some(text)));
        }
        self.host
            .unknown_operand_call(&crate::runtime::jar_text::object_to_string())
    }

    /// The component calls this module answers; `None` for every other call
    /// (the host's text-only surface answers those).
    pub(crate) fn invoke_text(
        &mut self,
        r: &MethodRef,
        receiver: Option<&Value>,
        args: &[Value],
        class: &str,
        method: &str,
    ) -> Result<Option<Value>, NativeError> {
        if r.owner != COMPONENT && r.owner != MUTABLE {
            return Ok(None);
        }
        let name = r.name.as_str();
        let descriptor = r.descriptor.as_str();
        let Some(receiver) = receiver else {
            // Static factories with arguments.
            if name == "literal"
                && descriptor == "(Ljava/lang/String;)Lnet/minecraft/network/chat/MutableComponent;"
            {
                let made = crate::runtime::host_component::literal(
                    expect_ref(
                        args.first()
                            .ok_or_else(|| NativeError::Metadata("literal missing text".into()))?,
                    )
                    .map_or("", String::as_str),
                );
                if let Some(states) = self.host.component_states_mut()
                    && let Value::Host(handle) = &made
                {
                    states.insert(
                        crate::runtime::host_component::marker(handle).to_owned(),
                        ComponentState {
                            base: Map::from_iter([(
                                "text".into(),
                                json!(expect_ref(args.first().unwrap()).map_or("", String::as_str)),
                            )]),
                            siblings: Vec::new(),
                            args: None,
                        },
                    );
                }
                return Ok(Some(made));
            }
            if name == "translatableWithFallback"
                && descriptor == format!("(Ljava/lang/String;Ljava/lang/String;{RETURNS_MUTABLE}")
                && let [key, fallback] = args
            {
                let Some(key) = expect_ref(key) else {
                    return Ok(None);
                };
                let mut object = Map::from_iter([("translate".to_owned(), json!(key))]);
                if let Some(fallback) = expect_ref(fallback) {
                    object.insert("fallback".into(), json!(fallback));
                }
                return bounded(Json::Object(object)).map(Some);
            }
            let (key, fallback, array) = match (name, args) {
                ("translatable" | "translatableEscape", [key, array])
                    if descriptor
                        == format!("(Ljava/lang/String;[Ljava/lang/Object;{RETURNS_MUTABLE}") =>
                {
                    (key, None, array)
                }
                ("translatableWithFallback", [key, fallback, array])
                    if descriptor
                        == format!(
                            "(Ljava/lang/String;Ljava/lang/String;[Ljava/lang/Object;{RETURNS_MUTABLE}"
                        ) =>
                {
                    (key, Some(fallback), array)
                }
                _ => return Ok(None),
            };
            let Some(key) = expect_ref(key) else {
                return Ok(None);
            };
            let raw_values = self.array(array)?.into_owned();
            let escape = name == "translatableEscape";
            let mut values = Vec::with_capacity(raw_values.len());
            for (index, value) in raw_values.into_iter().enumerate() {
                if escape
                    && !crate::runtime::host_component::is_component(&value)
                    && !is_allowed_primitive_argument(&value)
                {
                    // `Component.translatableEscape` replaces any non-allowed,
                    // non-Component element in `args` with `String.valueOf(arg)`
                    // before delegating to `translatable`.
                    let escaped = self.escape_translation_arg(&value, class, method)?;
                    if let Some(array_key) = crate::runtime::host_array::array_key(array)
                        && let Some(crate::runtime::array_data::ArrayData::Ref(elements)) =
                            self.arrays.get_mut(array_key)
                        && let Some(slot) = elements.get_mut(index)
                    {
                        *slot = escaped.clone();
                    }
                    values.push(escaped);
                } else {
                    values.push(value);
                }
            }
            let mut object = Map::from_iter([("translate".to_owned(), json!(key))]);
            if let Some(fallback) = fallback.and_then(expect_ref) {
                object.insert("fallback".into(), json!(fallback));
            }
            // Populate `"with"` in the seed JSON only when every argument
            // currently serialises; otherwise preserve all constructor
            // arguments verbatim in `ComponentState` without counting here
            // (`TranslatableContents` stores any `Object[]` and only rejects
            // disallowed arguments when its codec serialises).
            let mut with = Vec::with_capacity(values.len());
            let mut all_serialisable = true;
            for value in &values {
                if crate::runtime::host_component::is_component(value) {
                    match self.component_now(value) {
                        Some(json) => with.push(json),
                        None => {
                            all_serialisable = false;
                            break;
                        }
                    }
                } else if let Some(json) = argument(value) {
                    with.push(json);
                } else {
                    all_serialisable = false;
                    break;
                }
            }
            if all_serialisable && !with.is_empty() {
                object.insert("with".into(), Json::Array(with));
            }
            let made = bounded(Json::Object(object.clone()))?;
            // `TranslatableContents` keeps its arguments: a component one
            // changed later shows in this one, and non-codec arguments stay
            // intact until serialization refuses or counts them.
            if let (Some(states), Value::Host(handle)) = (self.host.component_states_mut(), &made) {
                states.insert(
                    crate::runtime::host_component::marker(handle).to_owned(),
                    ComponentState {
                        base: object,
                        siblings: Vec::new(),
                        args: (!values.is_empty()).then_some(values),
                    },
                );
            } else if !all_serialisable {
                return self.host.unknown_operand_call(r).map(Some);
            }
            return Ok(Some(made));
        };
        if !crate::runtime::host_component::is_component(receiver) {
            return Ok(None);
        }
        match (name, descriptor) {
            ("getString", "()Ljava/lang/String;") => match self.component_now(receiver) {
                // Java prints a translation in the server's language, with its
                // arguments; the interpreter has no language table: counted,
                // the key answered as before.
                Some(json) if has_translation(&json) => {
                    self.host.unknown_operand_call(r)?;
                    Ok(Some(Value::Ref(Some(plain(&json)))))
                }
                Some(json) => Ok(Some(Value::Ref(Some(plain(&json))))),
                // Nested past what resolves (Java: a cycle overflows).
                None => self.host.unknown_operand_call(r).map(Some),
            },
            // A new object: the same contents, style, siblings and arguments.
            ("copy", "()Lnet/minecraft/network/chat/MutableComponent;") => Ok(
                crate::runtime::host_component::copy(self.host.component_states_mut(), receiver, false),
            ),
            // The contents alone: no style, no siblings.
            ("plainCopy", "()Lnet/minecraft/network/chat/MutableComponent;") => Ok(
                crate::runtime::host_component::copy(self.host.component_states_mut(), receiver, true),
            ),
            (
                "withStyle",
                "(Lnet/minecraft/ChatFormatting;)Lnet/minecraft/network/chat/MutableComponent;",
            ) => {
                let Some(format) = args.first().and_then(formatting).map(str::to_owned) else {
                    return Ok(None);
                };
                apply_format(&mut Map::new(), &format).map_err(NativeError::Metadata)?;
                self.change_component(r, receiver, |state| {
                    let _ = apply_format(&mut state.base, &format);
                })
            }
            (
                "withStyle",
                "([Lnet/minecraft/ChatFormatting;)Lnet/minecraft/network/chat/MutableComponent;",
            ) => {
                let Some(array) = args.first() else {
                    return Ok(None);
                };
                let mut formats = Vec::new();
                for format in self.array(array)?.into_owned() {
                    let Some(format) = formatting(&format) else {
                        return Ok(None);
                    };
                    apply_format(&mut Map::new(), format).map_err(NativeError::Metadata)?;
                    formats.push(format.to_owned());
                }
                self.change_component(r, receiver, |state| {
                    // `Style.applyFormats` returns `Style.EMPTY` immediately on `RESET`,
                    // ignoring any remaining formats in the varargs array.
                    for format in &formats {
                        let _ = apply_format(&mut state.base, format);
                        if format == "RESET" {
                            break;
                        }
                    }
                })
            }
            ("withColor", "(I)Lnet/minecraft/network/chat/MutableComponent;") => {
                let Some(Value::Int(rgb)) = args.first() else {
                    return Ok(None);
                };
                let colour = json!(format!("#{:06X}", rgb & 0xFF_FFFF));
                self.change_component(r, receiver, |state| {
                    state.base.insert("color".into(), colour);
                })
            }
            (
                "append",
                "(Lnet/minecraft/network/chat/Component;)Lnet/minecraft/network/chat/MutableComponent;",
            )
            | ("append", "(Ljava/lang/String;)Lnet/minecraft/network/chat/MutableComponent;") => {
                let Some(sibling) = args.first() else {
                    return Ok(None);
                };
                // `append(String)` is `append(Component.literal(text))`.
                let sibling = if descriptor.contains("Ljava/lang/String;") {
                    crate::runtime::host_component::literal(expect_ref(sibling).map_or("", String::as_str))
                } else if crate::runtime::host_component::is_component(sibling) {
                    sibling.clone()
                } else {
                    return Ok(None);
                };
                self.change_component(r, receiver, |state| state.siblings.push(sibling))
            }
            _ => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_component_is_its_json_and_no_string_is_one() {
        let literal = json_now(&from_json(&json!({"text": "hi"})), None).unwrap();
        assert_eq!(
            crate::runtime::host_component::bare_literal(&literal).as_deref(),
            Some("hi")
        );
        let key = json_now(&from_json(&json!({"translate": "demo.key"})), None).unwrap();
        assert_eq!(
            crate::runtime::host_component::bare_key(&key).as_deref(),
            Some("demo.key")
        );
        // The old markers' spellings are Strings, not components.
        for text in [
            "component/literal/hi",
            "component/translatable/demo.key",
            "component/json/{\"text\":\"hi\"}",
        ] {
            assert_eq!(
                json_now(&Value::Ref(Some(text.into())), None),
                None,
                "{text}"
            );
        }
        let styled = from_json(&json!({"text": "hi", "color": "red"}));
        assert_eq!(
            json_now(&styled, None),
            Some(json!({"text": "hi", "color": "red"}))
        );
    }

    #[test]
    fn formats_apply_like_style_apply_format() {
        let mut object = Map::new();
        apply_format(&mut object, "BOLD").unwrap();
        apply_format(&mut object, "GOLD").unwrap();
        // A colour after a decoration keeps the decoration (applyFormat, not
        // the legacy `§` rule).
        assert_eq!(
            Json::Object(object.clone()),
            json!({"bold": true, "color": "gold"})
        );
        apply_format(&mut object, "RESET").unwrap();
        assert_eq!(Json::Object(object), json!({}));
    }

    #[test]
    fn arguments_serialise_as_vanilla_keeps_them() {
        assert_eq!(argument(&Value::Int(7)), Some(json!(7)));
        assert_eq!(argument(&Value::Boxed(Boxed::Integer(3))), Some(json!(3)));
        assert_eq!(
            argument(&Value::Boxed(Boxed::Boolean(true))),
            Some(json!(true))
        );
        assert_eq!(argument(&Value::Long(42)), Some(json!(42)));
        assert_eq!(argument(&Value::Float(1.5)), Some(json!(1.5)));
        // A String spelled like the old box marker is text.
        assert_eq!(
            argument(&Value::Ref(Some("integer:3".into()))),
            Some(json!("integer:3"))
        );
        assert_eq!(
            argument(&Value::Ref(Some("Alex".into()))),
            Some(json!("Alex"))
        );
        // A String spelled like the old component marker is a String argument.
        assert_eq!(
            argument(&Value::Ref(Some("component/literal/Alex".into()))),
            Some(json!("component/literal/Alex"))
        );
        // Arguments rejected by vanilla's `TranslatableContents.isAllowedPrimitiveArgument`:
        // `null`, `Character`, arbitrary host objects, and non-finite floats.
        assert_eq!(argument(&Value::Ref(None)), None);
        assert_eq!(argument(&Value::Boxed(Boxed::Character('c' as u16))), None);
        assert_eq!(
            argument(&Value::Host(crate::runtime::HostRef::new(
                crate::runtime::HostKind::JarObject,
                "java/lang/Object#0",
            ))),
            None
        );
        assert_eq!(argument(&Value::Float(f64::NAN)), None);
        assert_eq!(
            plain(&json!({"text": "a", "extra": ["b", {"translate": "k"}]})),
            "abk"
        );
    }
}
