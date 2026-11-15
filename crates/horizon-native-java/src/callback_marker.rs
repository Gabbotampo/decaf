//! A lambda or method reference as the interpreter names it (`invokedynamic`
//! through `LambdaMetafactory`, and a reflective `metafactory` call): one text
//! form, one constructor, one parser, one classifier (#156 P15a).
//!
//! `@indy:<tag>:<class#method>:<cp index>:<SAM interface>:<target>:<captures>`
//!
//! * `<tag>` is a per-process random value. A jar `String` cannot spell a
//!   live callback, because the tag is never printed: every path that would
//!   show a callback's text answers [`CALLBACK_TEXT`] and counts it (Java's
//!   text is the hidden class and an identity hash, which no run
//!   reproduces). A `String` spelled `@indy:…` without the tag is the
//!   `String` it is: never a callback, never cast to `Runnable` and run.
//! * `<SAM interface>` is the call site's own type (the `invokedynamic`
//!   descriptor's return type): a lambda's class implements exactly that
//!   interface (javap and HotSpot 25: `instanceof Runnable` true,
//!   `instanceof Comparable`/`Serializable` false), so `instanceof` and casts
//!   answer from it (`instance_of`).
//! * `<captures>` are escaped value markers (`value_marker`), split back by
//!   `split_capture_markers`.
//!
//! P15b moves callbacks onto `HostKind::Callback` handles; until then this
//! module is the only place that knows the text form.

use crate::{MethodRef, NativeError, Value};

/// What [`crate::split_capture_markers`] returns for one callback.
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

/// `@indy:<tag>:`, the start of every live callback's text.
fn prefix() -> String {
    format!("@indy:{}:", tag())
}

/// What a callback prints where its text is counted (`toString`, string
/// concatenation, a collection's text): never its marker, which would hand
/// the tag to the jar.
pub(crate) const CALLBACK_TEXT: &str = "null";

/// The text of a new callback from `source` (`class#method`), call site
/// `index`, making `sam`, implemented by `target` (`owner#name(desc)`, or
/// `?` when the bootstrap names none), with `captures` already escaped.
pub(crate) fn make(source: &str, index: u16, sam: &str, target: &str, captures: &str) -> String {
    format!("{}{source}:{index}:{sam}:{target}:{captures}", prefix())
}

/// Whether `text` is a live callback of this process.
pub(crate) fn is_callback(text: &str) -> bool {
    text.starts_with(&prefix())
}

/// The text of `value` when it is a live callback.
pub(crate) fn callback_text(value: &Value) -> Option<&str> {
    match value {
        Value::Ref(Some(text)) if is_callback(text) => Some(text),
        _ => None,
    }
}

/// Whether `text` holds a live callback anywhere (a composition marker
/// embeds them): such text is never printed.
pub(crate) fn holds_callback(text: &str) -> bool {
    text.contains(&prefix())
}

/// The functional interface a live callback's call site made.
pub(crate) fn sam(text: &str) -> Option<&str> {
    let rest = text.strip_prefix(&prefix())?;
    let sam = rest.split(':').nth(2)?;
    (!sam.is_empty()).then_some(sam)
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
    let sam = parts.next().ok_or_else(|| malformed("interface"))?;
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
        crate::split_capture_markers(captures)?
            .into_iter()
            .map(|capture| crate::parse_value_marker(&capture))
            .collect::<Result<Vec<_>, _>>()?
    };
    Ok(Callback {
        target: MethodRef {
            owner: owner.to_owned(),
            name: method_and_descriptor[..descriptor_at].to_owned(),
            descriptor: method_and_descriptor[descriptor_at..].to_owned(),
        },
        sam: sam.to_owned(),
        captures,
    })
}

impl<H: crate::Host> crate::Runtime<'_, H> {
    /// `callback instanceof wanted` for a live callback's `text`: its class
    /// implements its call site's functional interface and that interface's
    /// superinterfaces, and nothing else (HotSpot 25: a `Runnable` lambda is
    /// no `Comparable` and no `Serializable`). `None` where the interface's
    /// supertypes are not known (a game or library interface the run does
    /// not hold): the caller counts it.
    pub(crate) fn callback_instance_of(&self, text: &str, wanted: &str) -> Option<bool> {
        let sam = sam(text)?;
        if wanted == "java/lang/Object" {
            return Some(true);
        }
        let mut queue = vec![sam.to_owned()];
        let mut seen = std::collections::BTreeSet::new();
        while let Some(interface) = queue.pop() {
            if seen.len() >= 64 || !seen.insert(interface.clone()) {
                continue;
            }
            if interface == wanted {
                return Some(true);
            }
            if let Some(file) = self.class(&interface) {
                queue.extend(file.interface_names());
            } else if crate::jdk_closure::in_jdk_package(&interface) {
                match crate::host_ref::ClassHierarchy::is_instance(
                    &crate::host_ref::JdkHierarchy,
                    &interface,
                    wanted,
                ) {
                    Some(true) => return Some(true),
                    Some(false) => {}
                    None => return None,
                }
            } else {
                return None;
            }
        }
        Some(false)
    }
}

/// The text of a two-sided combinator's result (`p.and(q)`, `c.andThen(d)`,
/// `f.compose(g)`): `@fn:<op>:<n>:<left><right>`, `n` the byte length of
/// `left`. The length makes a side that is itself a composite, or holds a
/// `|`, read back whole: the old `left|right` form cut
/// `c1.andThen(c2).andThen(c3)` at the inner composite's bar, so the chain
/// ran nothing (#258). P15c holds the sides by reference instead.
pub(crate) fn composite(op: &str, left: &str, right: &str) -> String {
    format!("@fn:{op}:{}:{left}{right}", left.len())
}

/// The two sides of a [`composite`] whose combinator is `op`.
pub(crate) fn composite_sides<'a>(text: &'a str, op: &str) -> Option<(&'a str, &'a str)> {
    let rest = text
        .strip_prefix("@fn:")?
        .strip_prefix(op)?
        .strip_prefix(':')?;
    let (length, sides) = rest.split_once(':')?;
    let length: usize = length.parse().ok()?;
    Some((sides.get(..length)?, sides.get(length..)?))
}

/// A live callback token naming `name` and nothing a call could run, for
/// tests that hand a host a listener without a jar behind it.
#[doc(hidden)]
pub fn test_token(name: &str) -> String {
    format!("{}{name}", prefix())
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
        assert_eq!(sam(&live), Some("java/lang/Runnable"));
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
        let inner = composite("consumer.then", "a|1", "b");
        let outer = composite("consumer.then", &inner, "c");
        assert_eq!(
            composite_sides(&outer, "consumer.then"),
            Some((inner.as_str(), "c"))
        );
        assert_eq!(composite_sides(&inner, "consumer.then"), Some(("a|1", "b")));
        assert_eq!(composite_sides(&outer, "function.then"), None);
        assert_eq!(
            composite_sides("@fn:consumer.then:9:ab", "consumer.then"),
            None
        );
    }
}
