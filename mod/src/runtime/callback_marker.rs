//! Lambdas, method references and functional combinators as the interpreter
//! holds them: [`HostKind::Callback`] handles keyed by one text form, with
//! one constructor, one parser and one `instanceof` (#156 P15a).
//!
//! A callback is a handle, never a `Value::Ref`: no `String` model (`charAt`,
//! a natural-order compare, `String.join`) reads it, so its key cannot be
//! read out, and no jar `String` is one, however it is spelled (#261 review
//! B1–B3). P15b replaces the key with a resident row (captures as values,
//! identity per evaluation).
//!
//! Keys:
//!
//! * a lambda: `@indy:<tag>:<class#method>:<cp index>:<interfaces>:<target>:<captures>`;
//! * a combinator's result: `@fn:<tag>:<op>:<operands>` (`predicate.and`,
//!   `consumer.then`, `identity/function`...).
//!
//! `<tag>` is random per process and never printed; a handle decoded from
//! text (`HostRef::decode`) must carry it ([`is_key`]). `<interfaces>` are
//! the call site's functional interface (the `invokedynamic` descriptor's
//! return type) and `altMetafactory`'s marker interfaces, `,`-joined: a
//! lambda's class implements exactly those (javap and HotSpot 25:
//! `instanceof Runnable` true, `Comparable`/`Serializable` false). Captures
//! and operands are escaped value markers (`value_marker`), split back by
//! `split_capture_markers`, so a nested callback or a captured `|` reads back
//! whole.

use crate::runtime::host_ref::{ClassHierarchy, HostKind, HostRef, JdkHierarchy, Methods};
use crate::runtime::{MethodRef, NativeError, Value};

/// What [`crate::runtime::split_capture_markers`] returns for one callback.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Callback {
    /// The implementation method the call site's bootstrap names.
    pub(crate) target: MethodRef,
    /// The functional interface the call site makes (`java/lang/Runnable`);
    /// empty when the producer did not know it.
    pub(crate) sam: String,
    /// The captured values, in the implementation method's argument order.
    pub(crate) captures: Vec<Value>,
}

/// The per-process tag every live callback's text carries.
fn tag() -> &'static str {
    static TAG: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    TAG.get_or_init(|| {
        use std::hash::{BuildHasher, Hasher};
        let value = std::collections::hash_map::RandomState::new()
            .build_hasher()
            .finish();
        format!("{value:016x}")
    })
}

/// `@indy:<tag>:`, the start of every lambda's key.
fn prefix() -> String {
    format!("@indy:{}:", tag())
}

/// `@fn:<tag>:`, the start of every combinator result's key.
fn composite_prefix() -> String {
    format!("@fn:{}:", tag())
}

/// What a callback prints where its text is counted (`toString`, string
/// concatenation, a collection's text): never its marker, which would hand
/// the tag to the jar.
pub(crate) const CALLBACK_TEXT: &str = "null";

/// The key of a new lambda from `source` (`class#method`), call site
/// `index`, implementing `interfaces` (`,`-joined), run by `target`
/// (`owner#name(desc)`, or `?` when the bootstrap names none), with
/// `captures` already escaped.
pub(crate) fn make(
    source: &str,
    index: u16,
    interfaces: &str,
    target: &str,
    captures: &str,
) -> String {
    format!(
        "{}{source}:{index}:{interfaces}:{target}:{captures}",
        prefix()
    )
}

/// A new lambda's handle ([`make`]'s key).
pub(crate) fn lambda(
    source: &str,
    index: u16,
    interfaces: &str,
    target: &str,
    captures: &str,
) -> Value {
    handle(make(source, index, interfaces, target, captures))
}

fn handle(key: String) -> Value {
    Value::Host(HostRef::new(HostKind::Callback, key))
}

/// Whether `text` is a lambda's key of this process.
pub(crate) fn is_callback(text: &str) -> bool {
    text.starts_with(&prefix())
}

/// Whether `text` is a callback key of this process (a lambda's or a
/// combinator result's): the only keys a `Callback` handle decodes from.
pub(crate) fn is_key(text: &str) -> bool {
    is_callback(text) || text.starts_with(&composite_prefix())
}

/// The key of `value` when it is a callback handle.
pub(crate) fn callback_text(value: &Value) -> Option<&str> {
    match value {
        Value::Host(handle) if handle.kind() == HostKind::Callback => Some(handle.key()),
        _ => None,
    }
}

/// Whether `text` holds a callback's key anywhere (a collection marker
/// embeds a handle's text form): such text is never printed.
pub(crate) fn holds_callback(text: &str) -> bool {
    text.contains(&prefix()) || text.contains(&composite_prefix())
}

/// The interfaces a callback keyed `key` implements: a lambda's call site
/// interface and markers, a combinator's own interface.
pub(crate) fn interfaces(key: &str) -> Vec<&str> {
    if let Some(rest) = key.strip_prefix(&prefix()) {
        return rest
            .split(':')
            .nth(2)
            .map(|list| list.split(',').filter(|name| !name.is_empty()).collect())
            .unwrap_or_default();
    }
    let op = key
        .strip_prefix(&composite_prefix())
        .and_then(|rest| rest.split_once(':'))
        .map_or("", |(op, _)| op);
    match op {
        "predicate.and" | "predicate.or" | "predicate.not" => vec!["java/util/function/Predicate"],
        "consumer.then" => vec!["java/util/function/Consumer"],
        "biconsumer.then" => vec!["java/util/function/BiConsumer"],
        "function.then" | "function.compose" | IDENTITY_FUNCTION => {
            vec!["java/util/function/Function"]
        }
        "bifunction.then" => vec!["java/util/function/BiFunction"],
        IDENTITY_UNARY => vec!["java/util/function/UnaryOperator"],
        _ => Vec::new(),
    }
}

/// Read a live callback's text back.
pub(crate) fn parse(text: &str) -> Result<Callback, NativeError> {
    let malformed = |what: &str| NativeError::Metadata(format!("callback marker has no {what}"));
    let rest = text
        .strip_prefix(&prefix())
        .ok_or_else(|| NativeError::Metadata("not a callback of this run".into()))?;
    // The first four fields hold no `:` (a class, an index, an interface and
    // a descriptor have none); the captures may (a handle's `h<tag>:<key>`).
    let mut parts = rest.splitn(5, ':');
    let _source = parts.next().ok_or_else(|| malformed("source"))?;
    let _index = parts.next().ok_or_else(|| malformed("call-site index"))?;
    let interfaces = parts.next().ok_or_else(|| malformed("interface"))?;
    let target = parts.next().ok_or_else(|| malformed("target"))?;
    let captures = parts.next().ok_or_else(|| malformed("captures"))?;
    let (owner, method_and_descriptor) = target
        .split_once('#')
        .ok_or_else(|| NativeError::Metadata("callback target has no owner separator".into()))?;
    let descriptor_at = method_and_descriptor
        .find('(')
        .ok_or_else(|| NativeError::Metadata("callback target has no descriptor".into()))?;
    let captures = if captures.is_empty() {
        Vec::new()
    } else {
        crate::runtime::split_capture_markers(captures)?
            .into_iter()
            .map(|capture| crate::runtime::parse_value_marker(&capture))
            .collect::<Result<Vec<_>, _>>()?
    };
    Ok(Callback {
        target: MethodRef {
            owner: owner.to_owned(),
            name: method_and_descriptor[..descriptor_at].to_owned(),
            descriptor: method_and_descriptor[descriptor_at..].to_owned(),
        },
        sam: interfaces.split(',').next().unwrap_or_default().to_owned(),
        captures,
    })
}

/// `Function.identity()`'s and `UnaryOperator.identity()`'s operations: one
/// object each (JDK 25: non-capturing lambdas in static methods).
pub(crate) const IDENTITY_FUNCTION: &str = "identity/function";
pub(crate) const IDENTITY_UNARY: &str = "identity/unary";

/// A combinator's result: `op` over `operands` (`p.and(q)`: `predicate.and`
/// over `p`, `q`). The operands are held as value markers in the key, which
/// only the runtime writes, so a side reads back as the value it was (a
/// handle the jar holds, a nested composite) and never from jar text.
pub(crate) fn composite(op: &str, operands: &[&Value]) -> Value {
    let markers = operands
        .iter()
        .map(|value| crate::runtime::value_marker(value))
        .collect::<Vec<_>>()
        .join(",");
    handle(format!("{}{op}:{markers}", composite_prefix()))
}

/// The operands of `value` when it is a combinator result of `op`.
pub(crate) fn composite_operands(value: &Value, op: &str) -> Option<Vec<Value>> {
    let rest = callback_text(value)?
        .strip_prefix(&composite_prefix())?
        .strip_prefix(op)?
        .strip_prefix(':')?;
    if rest.is_empty() {
        return Some(Vec::new());
    }
    crate::runtime::split_capture_markers(rest)
        .ok()?
        .iter()
        .map(|marker| crate::runtime::parse_value_marker(marker).ok())
        .collect()
}

/// Whether two callbacks with key `key` are one object: a lambda that
/// captures nothing is one per call site (HotSpot 25 links a constant call
/// site), as are the `identity()` singletons; a capturing lambda and every
/// combinator result are new per evaluation.
pub(crate) fn canonical(key: &str) -> bool {
    if let Some(rest) = key.strip_prefix(&prefix()) {
        return rest.splitn(5, ':').nth(4).is_some_and(str::is_empty);
    }
    key.strip_prefix(&composite_prefix()).is_some_and(|rest| {
        rest == format!("{IDENTITY_FUNCTION}:") || rest == format!("{IDENTITY_UNARY}:")
    })
}

/// The types a callback surely is, for the class rule: `Object` (its
/// interfaces depend on the key; [`instance_of`] reads them).
pub(crate) fn declared_types(_: &str) -> &'static [&'static str] {
    &["java/lang/Object"]
}

/// `callback instanceof wanted`: its class implements its interfaces and
/// their superinterfaces, and is a final hidden class extending `Object`,
/// so it is no other class (`String`, `Number`: an exact `false`). `None`
/// where an interface's supertypes are not known (a game or library
/// interface the run does not hold): the caller counts it.
fn instance_of(handle: &HostRef, wanted: &str, hierarchy: &dyn ClassHierarchy) -> Option<bool> {
    if wanted == "java/lang/Object" {
        return Some(true);
    }
    let mut queue: Vec<String> = interfaces(handle.key())
        .into_iter()
        .map(str::to_owned)
        .collect();
    let mut unknown = queue.is_empty();
    let mut seen = std::collections::BTreeSet::new();
    while let Some(interface) = queue.pop() {
        if seen.len() >= 64 || !seen.insert(interface.clone()) {
            continue;
        }
        if interface == wanted || hierarchy.is_instance(&interface, wanted) == Some(true) {
            return Some(true);
        }
        if let Some(supers) = hierarchy.declared_interfaces(&interface) {
            queue.extend(supers);
        } else if !crate::runtime::jdk_closure::in_jdk_package(&interface)
            || JdkHierarchy.is_instance(&interface, wanted).is_none()
        {
            unknown = true;
        }
    }
    if hierarchy.is_interface(wanted) == Some(false) {
        return Some(false);
    }
    (!unknown).then_some(false)
}

/// A callback's object methods: identity `equals`; `toString` and
/// `hashCode` name its hidden class and identity hash, which no run
/// reproduces (counted).
pub(crate) const METHODS: Methods = Methods {
    to_string: |_, _| Ok(None),
    hash_code: |_, _| Ok(None),
    equals: |handle, other, _| Ok(crate::runtime::host_path::identity_equals(handle, other)),
    instance_of,
    // Its target is a class of the jar that made it.
    crosses_jars: |_, _| false,
    compare_to: None,
};

/// A live callback naming `name` and nothing a call could run, for tests
/// that hand a host a listener without a jar behind it.
#[doc(hidden)]
pub fn test_token(name: &str) -> Value {
    handle(format!("{}{name}", prefix()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_this_runs_tag_makes_a_callback() {
        let live = make(
            "demo/Mod#init",
            4,
            "java/lang/Runnable",
            "demo/Mod#lambda$0()V",
            "i5",
        );
        assert!(is_callback(&live));
        assert_eq!(interfaces(&live), ["java/lang/Runnable"]);
        let parsed = parse(&live).unwrap();
        assert_eq!(parsed.target.name, "lambda$0");
        assert_eq!(parsed.captures, [Value::Int(5)]);
        // The same text with any other tag is a `String`.
        let forged = live.replacen(tag(), "x", 1);
        assert!(!is_callback(&forged) && parse(&forged).is_err());
        assert!(!is_callback("@indy:demo/Mod#init:4:demo/Mod#lambda$0()V:"));
    }

    #[test]
    fn a_nested_composite_reads_back_whole() {
        let left = test_token("left");
        let pipe = Value::Ref(Some("a|1,b\\".into()));
        let inner = composite("consumer.then", &[&left, &pipe]);
        let outer = composite("consumer.then", &[&inner, &left]);
        assert_eq!(
            composite_operands(&outer, "consumer.then"),
            Some(vec![inner.clone(), left.clone()])
        );
        assert_eq!(
            composite_operands(&inner, "consumer.then"),
            Some(vec![left, pipe])
        );
        assert_eq!(composite_operands(&outer, "function.then"), None);
        // A `String` spelled like a composite's key is no composite.
        let spelled = Value::Ref(Some(callback_text(&outer).unwrap().to_owned()));
        assert_eq!(composite_operands(&spelled, "consumer.then"), None);
    }
}
