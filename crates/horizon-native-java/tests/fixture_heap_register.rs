//! A register's `getEntries()` view held across collections (#182 on #226's collector).
//!
//! The view (one `UnmodifiableSet` per register, in the host's per-object view cache) and the
//! set it reads stay, with their holders, through three rounds of calls that each make well past
//! a collection's worth of garbage: the same object, the same holders, Java's text.
//!
//! `fixtures/heap_register/demo/HeapRegister.java` runs unchanged on both sides; its `main` on
//! Java 25, against the real 26.3 server jar and NeoForge 26.3.0.101, printed `java25.txt`. The
//! stubs exist only for `javac`.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::{StdHost, Value};

/// `java demo.HeapRegister` on Java 25 (Temurin 25.0.5, `javac --release 21`) with the 26.3
/// server jar, NeoForge 26.3.0.101 universal and their libraries, the same on two runs.
const ORACLE: &str = include_str!("fixtures/heap_register/java25.txt");

const STUBS: &[(&str, &str)] = &[
    (
        "net/minecraft/SharedConstants.java",
        "package net.minecraft;\npublic class SharedConstants { public static void tryDetectVersion() {} }\n",
    ),
    (
        "net/minecraft/server/Bootstrap.java",
        "package net.minecraft.server;\npublic class Bootstrap { public static void bootStrap() {} }\n",
    ),
    (
        "net/minecraft/resources/ResourceKey.java",
        "package net.minecraft.resources;\npublic class ResourceKey<T> {}\n",
    ),
    (
        "net/minecraft/core/registries/Registries.java",
        "package net.minecraft.core.registries;\n\
         public final class Registries { public static final net.minecraft.resources.ResourceKey<?> ITEM = null; }\n",
    ),
    (
        "net/neoforged/neoforge/registries/DeferredRegister.java",
        "package net.neoforged.neoforge.registries;\n\
         public class DeferredRegister<T> {\n\
         public static <T> DeferredRegister<T> create(net.minecraft.resources.ResourceKey<?> registry, String namespace) { return null; }\n\
         public <I extends T> DeferredHolder<T, I> register(String name, java.util.function.Supplier<? extends I> value) { return null; }\n\
         public java.util.Collection<DeferredHolder<T, ? extends T>> getEntries() { return null; }\n}\n",
    ),
    (
        "net/neoforged/neoforge/registries/DeferredHolder.java",
        "package net.neoforged.neoforge.registries;\npublic class DeferredHolder<R, T extends R> {}\n",
    ),
];

fn expected(name: &str) -> &'static str {
    ORACLE
        .lines()
        .find_map(|line| line.strip_prefix(name)?.strip_prefix('='))
        .unwrap_or_else(|| panic!("no oracle line for {name}"))
}

fn call(fixture: &javac_fixture::Fixture, host: &mut StdHost, method: &str) -> String {
    match fixture.jar.invoke_method_with_args(
        "demo/HeapRegister",
        method,
        "()Ljava/lang/String;",
        None,
        &[],
        host,
    ) {
        Ok(Value::Ref(Some(text))) => text,
        other => format!("{other:?}"),
    }
}

#[test]
fn a_held_get_entries_view_survives_collections() {
    let mut sources = STUBS.to_vec();
    sources.push((
        "demo/HeapRegister.java",
        include_str!("fixtures/heap_register/demo/HeapRegister.java"),
    ));
    let fixture = javac_fixture::Fixture::build("heap-register", &sources);
    let mut host = fixture.jar.seed_host(StdHost::new());
    assert_eq!(call(&fixture, &mut host, "setup"), expected("setup"));
    assert_eq!(call(&fixture, &mut host, "check"), expected("check"));
    let mark = host.call_log_mark();
    let mut freed = 0;
    for round in 1..=3 {
        for _ in 0..40 {
            call(&fixture, &mut host, "garbage");
            host.rewind_call_log(mark);
        }
        freed = freed.max(host.freed_object_ranges());
        assert_eq!(
            call(&fixture, &mut host, "check"),
            expected("check"),
            "round {round}"
        );
    }
    assert!(freed > 0, "no collection ran");
    assert!(
        host.unknown_host_calls().is_empty(),
        "{:?}",
        host.unknown_host_calls()
    );
}
