//! Vanilla creative-tab keys used by Fabric's creative-tab events.
#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::{StdHost, Value};

const ORACLE: &[u8] = include_bytes!("fixtures/creative_mode_tabs_java25.txt");
const PROBE: &str = include_str!("fixtures/creative_mode_tabs/Probe.java");

const STUBS: &[(&str, &str)] = &[
    (
        "net/minecraft/resources/Identifier.java",
        r#"package net.minecraft.resources;
public final class Identifier {
  private final String value;
  private Identifier(String value) { this.value = value; }
  public static Identifier parse(String value) { return new Identifier(value); }
  public String toString() { return value; }
}"#,
    ),
    (
        "net/minecraft/resources/ResourceKey.java",
        r#"package net.minecraft.resources;
public final class ResourceKey<T> {
  private final ResourceKey<?> registry; private final Identifier location;
  private ResourceKey(ResourceKey<?> registry, Identifier location) { this.registry = registry; this.location = location; }
  public static <T> ResourceKey<T> createRegistryKey(Identifier id) { return new ResourceKey<>(null, id); }
  public static <T> ResourceKey<T> create(ResourceKey<?> registry, Identifier id) { return new ResourceKey<>(registry, id); }
  public Identifier location() { return location; }
  public boolean equals(Object other) { return other instanceof ResourceKey<?> key && location.toString().equals(key.location.toString()); }
}"#,
    ),
    (
        "net/minecraft/world/item/CreativeModeTab.java",
        "package net.minecraft.world.item; public final class CreativeModeTab {}",
    ),
    (
        "net/minecraft/world/item/CreativeModeTabs.java",
        r#"package net.minecraft.world.item;
import net.minecraft.resources.Identifier; import net.minecraft.resources.ResourceKey;
public final class CreativeModeTabs {
  private static final ResourceKey<CreativeModeTab> REGISTRY = ResourceKey.createRegistryKey(Identifier.parse("minecraft:creative_mode_tab"));
  public static final ResourceKey<CreativeModeTab> FUNCTIONAL_BLOCKS = ResourceKey.create(REGISTRY, Identifier.parse("minecraft:functional_blocks"));
  public static final ResourceKey<CreativeModeTab> COMBAT = ResourceKey.create(REGISTRY, Identifier.parse("minecraft:combat"));
}"#,
    ),
];

#[test]
fn creative_mode_tab_keys_match_java25_capture() {
    let fixture = javac_fixture::Fixture::build(
        "creative-mode-tabs",
        &[
            ("demo/Probe.java", PROBE),
            STUBS[0],
            STUBS[1],
            STUBS[2],
            STUBS[3],
        ],
    );
    let mut host = fixture.jar.seed_host(StdHost::new());
    let actual = fixture
        .jar
        .invoke_method(
            "demo/Probe",
            "rows",
            "()Ljava/lang/String;",
            None,
            &mut host,
        )
        .unwrap();
    let Value::Ref(Some(actual)) = actual else {
        panic!("expected text")
    };
    assert_eq!(actual.as_bytes(), ORACLE);
}
