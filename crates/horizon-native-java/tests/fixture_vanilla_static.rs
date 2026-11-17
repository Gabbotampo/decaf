//! Java 25 oracle fixture for CommonComponents.EMPTY and Identifier.CODEC.
#[path = "support/javac_fixture.rs"]
mod javac_fixture;
use horizon_native_java::StdHost;
use javac_fixture::Fixture;
const STUBS: &[(&str, &str)] = &[
    (
        "net/minecraft/network/chat/Component.java",
        r#"package net.minecraft.network.chat; public interface Component { static MutableComponent empty(){throw new AssertionError();} String getString(); }"#,
    ),
    (
        "net/minecraft/network/chat/MutableComponent.java",
        r#"package net.minecraft.network.chat; public interface MutableComponent extends Component {}"#,
    ),
    (
        "net/minecraft/network/chat/CommonComponents.java",
        r#"package net.minecraft.network.chat; public final class CommonComponents { public static final Component EMPTY=null; }"#,
    ),
    (
        "com/mojang/serialization/Codec.java",
        r#"package com.mojang.serialization; public interface Codec<T> { Codec<T> fieldOf(String s); }"#,
    ),
    (
        "net/minecraft/resources/Identifier.java",
        r#"package net.minecraft.resources; import com.mojang.serialization.Codec; public final class Identifier { public static final Codec<Identifier> CODEC=null; }"#,
    ),
];
const PROBE: &str = include_str!("fixtures/vanilla_static/demo/Probe.java");
const ORACLE: &str = include_str!("fixtures/vanilla_static/java25.txt");
#[test]
fn vanilla_static_rows_match_java25() {
    let mut files = STUBS.to_vec();
    files.push(("demo/Probe.java", PROBE));
    let f = Fixture::build("vanilla-static", &files);
    let mut h = f.jar.seed_host(StdHost::new());
    assert_eq!(f.text_on(&mut h, "rows"), ORACLE);
    assert_eq!(h.unknown_host_calls(), vec![("com/mojang/serialization/Codec#fieldOf(Ljava/lang/String;)Lcom/mojang/serialization/MapCodec;".to_owned(), 1)]);
}
