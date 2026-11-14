//! Host singletons the runtime answers by their owner's calls, as handles
//! (#167's review). They used to be plain text (`system-out`, `fml-loader`...)
//! that `is_plain_string` read as a `String`, so a cast of one named
//! `java.lang.String` where Java names the object's class.
//!
//! * [`HostKind::StdStream`]: `System.out` and `System.err`, keyed
//!   `system-out`/`system-err` (the in-memory sink a modelled writer
//!   appends to). Their class is exactly `java.io.PrintStream`, so
//!   `instanceof` and casts are exact (`(PrintStream) (Object) System.out`
//!   passes; a jar interface throws naming `PrintStream`).
//! * [`HostKind::LoaderSingleton`]: NeoForge's (and legacy Forge's) `ModList`,
//!   `FMLLoader` and `LoadingModList`, keyed by the declared class. Their
//!   runtime class is the loader's, which the runtime does not model, so a
//!   cast's message is counted (`ClassRule::Unknown`).

use crate::runtime::Value;
use crate::runtime::host_path::identity_equals;
use crate::runtime::host_ref::{HostKind, HostRef, Methods, instance_of_class};

/// `System.out` (`err` false) or `System.err`.
pub(crate) fn std_stream(err: bool) -> Value {
    let key = if err { "system-err" } else { "system-out" };
    Value::Host(HostRef::new(HostKind::StdStream, key))
}

/// The sink key a standard stream's handle names (`system-out`), for the
/// writer model that reads receivers by text.
pub(crate) fn stream_marker(value: &Value) -> Option<&String> {
    match value {
        Value::Host(handle) if handle.kind() == HostKind::StdStream => Some(handle.key_text()),
        _ => None,
    }
}

/// The loader classes a [`HostKind::LoaderSingleton`] is keyed by.
const LOADER_CLASSES: &[&str] = &[
    "net/neoforged/fml/ModList",
    "net/minecraftforge/fml/ModList",
    "net/neoforged/fml/loading/FMLLoader",
    "net/neoforged/fml/loading/LoadingModList",
];

/// The loader's one object of `class` (one of [`LOADER_CLASSES`]).
pub(crate) fn loader_singleton(class: &str) -> Value {
    debug_assert!(LOADER_CLASSES.contains(&class), "{class}");
    Value::Host(HostRef::new(HostKind::LoaderSingleton, class))
}

/// The type a loader singleton surely is: its key's class.
pub(crate) fn loader_singleton_types(key: &str) -> &'static [&'static str] {
    LOADER_CLASSES
        .iter()
        .position(|class| *class == key)
        .map_or(&[], |at| &LOADER_CLASSES[at..=at])
}

/// One object per name: identity `equals`; `hashCode` and `toString` are
/// `Object`'s identity ones (counted).
pub(crate) const METHODS: Methods = Methods {
    to_string: |_, _| Ok(None),
    hash_code: |_, _| Ok(None),
    equals: |handle, other, _| Ok(identity_equals(handle, other)),
    instance_of: instance_of_class,
    crosses_jars: |_, _| true,
    compare_to: None,
};
