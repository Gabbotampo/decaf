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
        r#"package com.mojang.serialization; public interface Codec<T> { com.mojang.serialization.MapCodec<T> fieldOf(String s); }"#,
    ),
    (
        "com/mojang/serialization/MapCodec.java",
        r#"package com.mojang.serialization; public interface MapCodec<T> extends Codec<T> {}"#,
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
    let mut f = STUBS.to_vec();
    f.push(("demo/Probe.java", PROBE));
    let x = Fixture::build("vanilla-static", &f);
    let mut h = x.jar.seed_host(StdHost::new());
    assert_eq!(x.text_on(&mut h, "rows"), ORACLE);
    assert!(h.unknown_host_calls().is_empty());
}
#[test]
fn vanilla_static_field_of_is_counted_on_fresh_host() {
    let mut f = STUBS.to_vec();
    f.push(("demo/Probe.java", PROBE));
    let x = Fixture::build("vanilla-static-counted", &f);
    let mut h = x.jar.seed_host(StdHost::new());
    assert_eq!(x.text_on(&mut h, "counted"), "true");
    assert_eq!(h.unknown_host_calls(),vec![("com/mojang/serialization/Codec#fieldOf(Ljava/lang/String;)Lcom/mojang/serialization/MapCodec;".to_owned(),1)]);
}
