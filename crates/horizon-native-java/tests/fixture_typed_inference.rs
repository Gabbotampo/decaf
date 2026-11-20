//! A source-built Fabric jar registers an item and a block the ordinary way,
//! with no `META-INF/horizon-native.json`.  The host sees the property-builder
//! calls the stub classes receive, so `Registry.register` can infer a bounded
//! [`horizon_native_java::RegistryDeclaration`] for a value it would otherwise
//! record as opaque.
//!
//! The stubs are compiled onto a separate classpath and deliberately *not*
//! packaged in the jar, mirroring a real install where the game is a host
//! surface: every builder call crosses `Host` and is observable on `StdHost`.

#[path = "support/scratch.rs"]
mod scratch;

use horizon_native_java::{Host, MethodRef, NativeJar, RegistryDeclaration, StdHost, Value};
use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
};
use zip::write::SimpleFileOptions;

const ITEM_PROPERTIES: &str = "net/minecraft/world/item/Item$Properties";
const ITEM: &str = "net/minecraft/world/item/Item";
const REGISTER: &str = "(Lnet/minecraft/core/Registry;Lnet/minecraft/resources/Identifier;\
                       Ljava/lang/Object;)Ljava/lang/Object;";

/// A Java source tree for [`build_jar`]: relative path => source text.
macro_rules! sources {
    ($($path:literal => $body:expr),* $(,)?) => {{
        let mut map: BTreeMap<&str, &str> = BTreeMap::new();
        $(map.insert($path, $body);)*
        map
    }};
}

fn javac() -> PathBuf {
    std::env::var_os("JAVA_HOME")
        .map(|home| PathBuf::from(home).join("bin/javac"))
        .unwrap_or_else(|| "javac".into())
}

fn files_under(root: &Path) -> Vec<PathBuf> {
    fs::read_dir(root)
        .unwrap()
        .flat_map(|entry| {
            let path = entry.unwrap().path();
            if path.is_dir() {
                files_under(&path)
            } else {
                vec![path]
            }
        })
        .collect()
}

fn scratch_root(name: &str) -> scratch::Scratch {
    scratch::Scratch::new(&format!("typed-inference-{name}"))
}

fn run_on_large_stack(test: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .name("typed-inference".into())
        .stack_size(8 << 20)
        .spawn(test)
        .unwrap()
        .join()
        .unwrap();
}

fn write_sources(root: &Path, tree: &BTreeMap<&str, &str>) -> Vec<PathBuf> {
    tree.iter()
        .map(|(relative, body)| {
            let path = root.join(relative);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, body).unwrap();
            path
        })
        .collect()
}

fn compile(out: &Path, classpath: Option<&Path>, sources: &[PathBuf], label: &str) {
    fs::create_dir_all(out).unwrap();
    let mut command = Command::new(javac());
    command.arg("--release").arg("17").arg("-d").arg(out);
    if let Some(classpath) = classpath {
        command.arg("-cp").arg(classpath);
    }
    let status = command.args(sources).status().unwrap();
    assert!(status.success(), "javac failed for {label}");
}

/// Compile API stubs onto a classpath, compile the mod against them, and
/// package only the mod's own class files plus its loader metadata.
fn build_jar(
    name: &str,
    stubs: &BTreeMap<&str, &str>,
    sources: &BTreeMap<&str, &str>,
    metadata: &[u8],
    native: Option<&[u8]>,
) -> (scratch::Scratch, PathBuf) {
    let root = scratch_root(name);
    let stub_classes = root.join("stub-classes");
    let classes = root.join("classes");
    let stub_paths = write_sources(&root.join("stub-src"), stubs);
    compile(&stub_classes, None, &stub_paths, name);
    let paths = write_sources(&root.join("src"), sources);
    compile(&classes, Some(&stub_classes), &paths, name);

    let jar_path = root.join(format!("{name}.jar"));
    let mut zip = zip::ZipWriter::new(fs::File::create(&jar_path).unwrap());
    zip.start_file("fabric.mod.json", SimpleFileOptions::default())
        .unwrap();
    zip.write_all(metadata).unwrap();
    if let Some(native) = native {
        zip.start_file("META-INF/horizon-native.json", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(native).unwrap();
    }
    for class in files_under(&classes) {
        let entry = class
            .strip_prefix(&classes)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        zip.start_file(entry, SimpleFileOptions::default()).unwrap();
        zip.write_all(&fs::read(class).unwrap()).unwrap();
    }
    zip.finish().unwrap();
    (root, jar_path)
}

/// The game surface a small Fabric content mod compiles against: the two
/// property builders, the classes they build, the registry and its handles.
fn game_stubs() -> BTreeMap<&'static str, &'static str> {
    sources! {
        "net/fabricmc/api/ModInitializer.java" =>
            "package net.fabricmc.api; public interface ModInitializer { void onInitialize(); }",
        "net/minecraft/resources/Identifier.java" => r#"package net.minecraft.resources;
public final class Identifier {
    private Identifier() { }
    public static Identifier fromNamespaceAndPath(String namespace, String path) { return null; }
}"#,
        "net/minecraft/world/item/Rarity.java" => r#"package net.minecraft.world.item;
    public enum Rarity { COMMON, UNCOMMON, RARE, EPIC }"#,
        "net/minecraft/world/item/Item.java" => r#"package net.minecraft.world.item;
public class Item {
    public Item(Properties properties) { }
    public static class Properties {
        public Properties() { }
        public Properties stacksTo(int count) { return this; }
        public Properties durability(int maxDamage) { return this; }
        public Properties rarity(Rarity rarity) { return this; }
        public Properties fireResistant() { return this; }
        public Properties craftingRemainingItem(Item item) { return this; }
    }
}"#,
        "net/minecraft/world/level/block/state/BlockBehaviour.java" => r#"package net.minecraft.world.level.block.state;
public class BlockBehaviour {
    public static class Properties {
        public Properties() { }
        public static Properties of() { return new Properties(); }
        public Properties strength(float destroyTime) { return this; }
        public Properties strength(float destroyTime, float explosionResistance) { return this; }
        public Properties friction(float friction) { return this; }
        public Properties speedFactor(float factor) { return this; }
        public Properties jumpFactor(float factor) { return this; }
        public Properties randomTicks() { return this; }
        public Properties noOcclusion() { return this; }
        public Properties requiresCorrectToolForDrops() { return this; }
        public Properties instabreak() { return this; }
    }
}"#,
        "net/minecraft/world/level/block/Block.java" => r#"package net.minecraft.world.level.block;
import net.minecraft.world.level.block.state.BlockBehaviour;
public class Block {
    public Block(BlockBehaviour.Properties properties) { }
}"#,
        "net/minecraft/core/Registry.java" => r#"package net.minecraft.core;
import net.minecraft.resources.Identifier;
public interface Registry<T> {
    static <V, T extends V> T register(Registry<V> registry, Identifier id, T value) {
        return value;
    }
}"#,
        "net/minecraft/core/registries/Registries.java" => r#"package net.minecraft.core.registries;
import net.minecraft.core.Registry;
import net.minecraft.world.item.Item;
import net.minecraft.world.level.block.Block;
public final class Registries {
    public static final Registry<Item> ITEM = null;
    public static final Registry<Block> BLOCK = null;
    public static final Registry<Object> SOUND_EVENT = null;
}"#,
    }
}

/// The mod: an item built from a chained `Item.Properties`, a block built from
/// a chained `BlockBehaviour.Properties`, and one registration in a registry
/// the host has no template for.
fn typed_mod() -> BTreeMap<&'static str, &'static str> {
    sources! {
        "demo/TypedInferenceMod.java" => r#"
package demo;
import net.fabricmc.api.ModInitializer;
import net.minecraft.core.Registry;
import net.minecraft.core.registries.Registries;
import net.minecraft.resources.Identifier;
import net.minecraft.world.item.Item;
import net.minecraft.world.item.Rarity;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.state.BlockBehaviour;

public final class TypedInferenceMod implements ModInitializer {
    @Override
    public void onInitialize() {
        // 16 per stack, 32 durability, rare: only max_stack_size and rarity
        // have a content-loader key, so durability stays in the host record.
        Item wand = new Item(new Item.Properties().stacksTo(16).durability(32).rarity(Rarity.RARE));
        Registry.register(Registries.ITEM, Identifier.fromNamespaceAndPath("fixture", "wand"), wand);

        Block lamp = new Block(BlockBehaviour.Properties.of()
                .strength(2.0F, 6.0F).requiresCorrectToolForDrops());
        Registry.register(Registries.BLOCK, Identifier.fromNamespaceAndPath("fixture", "lamp"), lamp);

        // No template vocabulary for a sound event: the row must stay opaque.
        Registry.register(Registries.SOUND_EVENT,
                Identifier.fromNamespaceAndPath("fixture", "chime"),
                new Item(new Item.Properties()));
    }
}
"#,
    }
}

fn construct(
    host: &mut StdHost,
    owner: &str,
    marker: &str,
    name: &str,
    descriptor: &str,
    args: &[Value],
) -> Value {
    host.invoke_instance(
        &MethodRef {
            owner: owner.into(),
            name: name.into(),
            descriptor: descriptor.into(),
        },
        &Value::Ref(Some(marker.into())),
        args,
    )
    .unwrap()
}

/// The registry `BuiltInRegistries.<field>` answers (a host handle).
fn built_in_registry(field: &str) -> Value {
    let field = MethodRef {
        owner: "net/minecraft/core/registries/BuiltInRegistries".into(),
        name: field.into(),
        descriptor: "Lnet/minecraft/core/DefaultedRegistry;".into(),
    };
    StdHost::new().get_static(&field).unwrap()
}

/// The `Identifier` `Identifier.parse(text)` answers (a host handle).
fn parsed_id(text: &str) -> Value {
    let parse = MethodRef {
        owner: "net/minecraft/resources/Identifier".into(),
        name: "parse".into(),
        descriptor: "(Ljava/lang/String;)Lnet/minecraft/resources/Identifier;".into(),
    };
    StdHost::new()
        .invoke(&parse, &[Value::Ref(Some(text.into()))])
        .unwrap()
}

#[test]
fn fabric_fixture_infers_typed_item_and_block_declarations() {
    let (root, jar_path) = build_jar(
        "typed-inference",
        &game_stubs(),
        &typed_mod(),
        br#"{"id":"fixture-typed","version":"1.0","entrypoints":{"main":"demo.TypedInferenceMod"}}"#,
        None,
    );
    let jar = NativeJar::from_path(&jar_path).unwrap();
    assert!(
        jar.native_declarations.is_empty(),
        "no metadata declarations"
    );
    let entry = jar.discover_entrypoint("main").unwrap();
    let mut host = jar.seed_host(StdHost::new());
    jar.execute(&entry.class, entry.method.as_deref(), "main", &mut host)
        .unwrap();

    let keys = host
        .registrations()
        .iter()
        .map(|entry| entry.key())
        .collect::<Vec<_>>();
    assert_eq!(
        keys,
        [
            "minecraft:item/fixture:wand",
            "minecraft:block/fixture:lamp",
            "minecraft:sound_event/fixture:chime",
        ]
    );

    let wand = &host.registrations()[0];
    assert_eq!(
        wand.declaration,
        Some(RegistryDeclaration::Item {
            template: "minecraft:stick".into(),
            properties: BTreeMap::from([
                ("max_stack_size".into(), "16".into()),
                ("rarity".into(), "rare".into()),
            ]),
        })
    );
    // The full chain is kept: `durability` has no loader key, so it is not
    // exported and cannot fail the declaration for an unknown property.
    // The item the jar `new`ed: a handle keyed `class#n`.
    let Value::Host(wand_handle) = &wand.value else {
        panic!("item value is not an object: {:?}", wand.value);
    };
    let wand_marker = wand_handle.key();
    assert_eq!(
        host.item_properties(wand_marker),
        Some(&BTreeMap::from([
            ("durability".into(), "32".into()),
            ("rarity".into(), "rare".into()),
            ("stacksTo".into(), "16".into()),
        ]))
    );

    let lamp = &host.registrations()[1];
    assert_eq!(
        lamp.declaration,
        Some(RegistryDeclaration::Block {
            template: "minecraft:stone".into(),
            properties: BTreeMap::from([
                ("explosion_resistance".into(), "6".into()),
                ("hardness".into(), "2".into()),
                ("requires_tool".into(), "true".into()),
            ]),
            item: true,
        })
    );
    let Value::Host(lamp_handle) = &lamp.value else {
        panic!("block value is not an object: {:?}", lamp.value);
    };
    let lamp_marker = lamp_handle.key();
    assert_eq!(
        host.block_properties(lamp_marker),
        Some(&BTreeMap::from([
            ("destroyTime".into(), "2".into()),
            ("explosionResistance".into(), "6".into()),
            ("requiresCorrectToolForDrops".into(), "true".into()),
        ]))
    );

    // The host has no item or block template for a sound event, so the row it
    // saw a builder for still stays opaque rather than inventing content.
    assert_eq!(host.registrations()[2].declaration, None);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn explicit_declaration_replaces_an_inferred_one() {
    let mut host = StdHost::new();
    construct(
        &mut host,
        ITEM_PROPERTIES,
        "net/minecraft/world/item/Item$Properties#0",
        "<init>",
        "()V",
        &[],
    );
    construct(
        &mut host,
        ITEM_PROPERTIES,
        "net/minecraft/world/item/Item$Properties#0",
        "stacksTo",
        "(I)Lnet/minecraft/world/item/Item$Properties;",
        &[Value::Int(16)],
    );
    construct(
        &mut host,
        ITEM,
        "net/minecraft/world/item/Item#0",
        "<init>",
        "(Lnet/minecraft/world/item/Item$Properties;)V",
        &[Value::Ref(Some(
            "net/minecraft/world/item/Item$Properties#0".into(),
        ))],
    );
    let register = MethodRef {
        owner: "net/minecraft/core/Registry".into(),
        name: "register".into(),
        descriptor: REGISTER.into(),
    };
    host.invoke(
        &register,
        &[
            built_in_registry("ITEM"),
            parsed_id("ruby:wand"),
            Value::Ref(Some("net/minecraft/world/item/Item#0".into())),
        ],
    )
    .unwrap();
    assert_eq!(
        host.registrations()[0].declaration,
        Some(RegistryDeclaration::Item {
            template: "minecraft:stick".into(),
            properties: BTreeMap::from([("max_stack_size".into(), "16".into())]),
        })
    );

    // An explicit adapter (metadata or a loader row) stays authoritative over
    // what the host inferred from the builder chain.
    let explicit = RegistryDeclaration::Item {
        template: "minecraft:stick".into(),
        properties: BTreeMap::new(),
    };
    host.attach_registration_declaration("minecraft:item", "ruby:wand", explicit.clone())
        .unwrap();
    assert_eq!(host.registrations()[0].declaration, Some(explicit.clone()));

    // An opaque registration keeps the all-or-nothing path: the first explicit
    // declaration lands, a second one cannot silently replace it.
    host.invoke(
        &register,
        &[
            built_in_registry("ITEM"),
            parsed_id("ruby:plain"),
            Value::Ref(Some("@indy:plain-item".into())),
        ],
    )
    .unwrap();
    host.attach_registration_declaration("minecraft:item", "ruby:plain", explicit.clone())
        .unwrap();
    let error = host
        .attach_registration_declaration("minecraft:item", "ruby:plain", explicit)
        .unwrap_err();
    assert!(
        error.contains("already has a declaration"),
        "unexpected error: {error}"
    );
}

#[test]
fn inference_records_a_plain_item_with_no_properties() {
    let mut host = StdHost::new();
    let properties = "net/minecraft/world/item/Item$Properties#0";
    construct(&mut host, ITEM_PROPERTIES, properties, "<init>", "()V", &[]);
    construct(
        &mut host,
        ITEM,
        "net/minecraft/world/item/Item#0",
        "<init>",
        "(Lnet/minecraft/world/item/Item$Properties;)V",
        &[Value::Ref(Some(properties.into()))],
    );
    host.invoke(
        &MethodRef {
            owner: "net/minecraft/core/Registry".into(),
            name: "register".into(),
            descriptor: REGISTER.into(),
        },
        &[
            built_in_registry("ITEM"),
            parsed_id("ruby:plain"),
            Value::Ref(Some("net/minecraft/world/item/Item#0".into())),
        ],
    )
    .unwrap();

    // A builder with no settings is still a declaration: the mod said "an
    // item", and its empty chain is not the same as an opaque marker.
    assert_eq!(
        host.registrations()[0].declaration,
        Some(RegistryDeclaration::Item {
            template: "minecraft:stick".into(),
            properties: BTreeMap::new(),
        })
    );
    // A second builder keeps its own chain, and a marker no `new` site or
    // factory named stays unknown.
    let second = "net/minecraft/world/item/Item$Properties#1";
    construct(&mut host, ITEM_PROPERTIES, second, "<init>", "()V", &[]);
    construct(
        &mut host,
        ITEM_PROPERTIES,
        second,
        "stacksTo",
        "(I)Lnet/minecraft/world/item/Item$Properties;",
        &[Value::Int(1)],
    );
    assert_eq!(host.item_properties(properties), Some(&BTreeMap::new()));
    assert_eq!(
        host.item_properties(second),
        Some(&BTreeMap::from([("stacksTo".into(), "1".into())]))
    );
    assert_eq!(
        host.item_properties("net/minecraft/world/item/Item$Properties#9"),
        None
    );
}

#[test]
fn metadata_declaration_keeps_that_row_opaque_to_inference() {
    let (root, jar_path) = build_jar(
        "typed-inference-metadata",
        &game_stubs(),
        &typed_mod(),
        br#"{"id":"fixture-typed","version":"1.0","entrypoints":{"main":"demo.TypedInferenceMod"}}"#,
        Some(
            br#"{"id":"fixture-typed","registrations":[{"registry":"minecraft:item","id":"fixture:wand","kind":"item","template":"minecraft:stick"}]}"#,
        ),
    );
    let jar = NativeJar::from_path(&jar_path).unwrap();
    assert_eq!(jar.native_declarations.len(), 1);
    let entry = jar.discover_entrypoint("main").unwrap();
    let mut host = jar.seed_host(StdHost::new());
    jar.execute(&entry.class, entry.method.as_deref(), "main", &mut host)
        .unwrap();

    let row = |host: &StdHost, id: &str| {
        host.registrations()
            .iter()
            .find(|entry| entry.id == id)
            .unwrap_or_else(|| panic!("{id} was not recorded"))
            .declaration
            .clone()
    };
    // The jar's own metadata is the explicit adapter for `fixture:wand`, so
    // the host leaves it opaque and the metadata merge stays authoritative.
    assert_eq!(row(&host, "fixture:wand"), None);
    // The block has no metadata row, so inference still supplies one.
    assert!(matches!(
        row(&host, "fixture:lamp"),
        Some(RegistryDeclaration::Block { .. })
    ));

    let _ = fs::remove_dir_all(root);
}

/// A vanilla `CraftingTableBlock` the jar `new`s is declared with the crafting table's template, so
/// the game gives it the 3x3 grid. A jar's own subclass (Runes' crafting altar) reaches the same
/// constructor through `super(...)`, but its own `useWithoutItem` opens its own menu: it keeps the
/// inferred template rather than borrowing the vanilla grid.
#[test]
fn only_a_vanilla_crafting_table_gets_the_crafting_table_template() {
    run_on_large_stack(only_a_vanilla_crafting_table_gets_the_crafting_table_template_body);
}

fn only_a_vanilla_crafting_table_gets_the_crafting_table_template_body() {
    let mut stubs = game_stubs();
    stubs.insert(
        "net/minecraft/world/level/block/CraftingTableBlock.java",
        r#"package net.minecraft.world.level.block;
import net.minecraft.world.level.block.state.BlockBehaviour;
public class CraftingTableBlock extends Block {
    public CraftingTableBlock(BlockBehaviour.Properties properties) { super(properties); }
}"#,
    );
    let mod_sources = sources! {
        "demo/Altar.java" => r#"package demo;
import net.minecraft.world.level.block.CraftingTableBlock;
import net.minecraft.world.level.block.state.BlockBehaviour;
public final class Altar extends CraftingTableBlock {
    public Altar(BlockBehaviour.Properties properties) { super(properties); }
}"#,
        "demo/TableMod.java" => r#"package demo;
import net.fabricmc.api.ModInitializer;
import net.minecraft.core.Registry;
import net.minecraft.core.registries.Registries;
import net.minecraft.resources.Identifier;
import net.minecraft.world.level.block.CraftingTableBlock;
import net.minecraft.world.level.block.state.BlockBehaviour;
public final class TableMod implements ModInitializer {
    @Override
    public void onInitialize() {
        Registry.register(Registries.BLOCK, Identifier.fromNamespaceAndPath("fixture", "table"),
                new CraftingTableBlock(BlockBehaviour.Properties.of().strength(2.5F)));
        Registry.register(Registries.BLOCK, Identifier.fromNamespaceAndPath("fixture", "altar"),
                new Altar(BlockBehaviour.Properties.of().strength(2.0F)));
    }
}"#,
    };
    let (root, jar_path) = build_jar(
        "typed-inference-crafting-table",
        &stubs,
        &mod_sources,
        br#"{"id":"fixture-table","version":"1.0","entrypoints":{"main":"demo.TableMod"}}"#,
        None,
    );
    let jar = NativeJar::from_path(&jar_path).unwrap();
    let entry = jar.discover_entrypoint("main").unwrap();
    let mut host = jar.seed_host(StdHost::new());
    jar.execute(&entry.class, entry.method.as_deref(), "main", &mut host)
        .unwrap();
    let template = |index: usize| match &host.registrations()[index].declaration {
        Some(RegistryDeclaration::Block { template, .. }) => template.clone(),
        other => panic!("row {index} is not a typed block: {other:?}"),
    };
    assert_eq!(template(0), "minecraft:crafting_table");
    assert_eq!(template(1), "minecraft:stone");
    let _ = fs::remove_dir_all(root);
}

/// A jar block's `createBlockStateDefinition` runs when its `super(...)` reaches any vanilla
/// block constructor, as `Block.<init>` calls it in the JVM. The properties it adds become the
/// declaration's state properties, with the values of the real 26.3 `BlockStateProperties`
/// (captured in `src/vanilla_block_state_properties.rs`), and the default is every property's
/// first value (`stateDefinition.any()`), `true` for a `BooleanProperty`. The stubs' fields are
/// declared with their real types, so the `getstatic` descriptors are the ones a real jar uses.
#[test]
fn a_jar_blocks_state_definition_runs_through_any_vanilla_constructor() {
    run_on_large_stack(a_jar_blocks_state_definition_runs_through_any_vanilla_constructor_body);
}

fn a_jar_blocks_state_definition_runs_through_any_vanilla_constructor_body() {
    let mut stubs = game_stubs();
    stubs.insert(
        "net/minecraft/world/level/block/Block.java",
        r#"package net.minecraft.world.level.block;
import net.minecraft.world.level.block.state.BlockBehaviour;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.block.state.StateDefinition;
public class Block {
    public Block(BlockBehaviour.Properties properties) { }
    protected void createBlockStateDefinition(StateDefinition.Builder<Block, BlockState> builder) { }
    protected BlockState defaultBlockState() { return new BlockState(); }
    protected void registerDefaultState(BlockState state) { }
}"#,
    );
    stubs.insert(
        "net/minecraft/world/level/block/CraftingTableBlock.java",
        r#"package net.minecraft.world.level.block;
import net.minecraft.world.level.block.state.BlockBehaviour;
public class CraftingTableBlock extends Block {
    public CraftingTableBlock(BlockBehaviour.Properties properties) { super(properties); }
}"#,
    );
    stubs.insert(
        "net/minecraft/world/level/block/piston/PistonBaseBlock.java",
        r#"package net.minecraft.world.level.block.piston;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.state.BlockBehaviour;
public class PistonBaseBlock extends Block {
    public PistonBaseBlock(BlockBehaviour.Properties properties) { super(properties); }
}"#,
    );
    stubs.insert(
        "net/minecraft/world/level/block/FurnaceBlock.java",
        r#"package net.minecraft.world.level.block;
import net.minecraft.world.level.block.state.BlockBehaviour;
public class FurnaceBlock extends Block {
    public FurnaceBlock(BlockBehaviour.Properties properties) { super(properties); }
}"#,
    );
    stubs.insert(
        "net/minecraft/world/level/block/state/BlockState.java",
        r#"package net.minecraft.world.level.block.state;
import net.minecraft.world.level.block.state.properties.Property;
public class BlockState {
    public <T> BlockState setValue(Property<T> property, T value) { return this; }
}"#,
    );
    stubs.insert(
        "net/minecraft/world/level/block/state/StateDefinition.java",
        r#"package net.minecraft.world.level.block.state;
import net.minecraft.world.level.block.state.properties.Property;
public class StateDefinition<O, S> {
    public static class Builder<O, S> {
        public Builder<O, S> add(Property<?>... properties) { return this; }
    }
}"#,
    );
    stubs.insert(
        "net/minecraft/world/level/block/state/properties/Property.java",
        "package net.minecraft.world.level.block.state.properties; public abstract class Property<T> { }",
    );
    stubs.insert(
        "net/minecraft/world/level/block/state/properties/EnumProperty.java",
        "package net.minecraft.world.level.block.state.properties; public final class EnumProperty<T> extends Property<T> { }",
    );
    stubs.insert(
        "net/minecraft/world/level/block/state/properties/BooleanProperty.java",
        "package net.minecraft.world.level.block.state.properties; public final class BooleanProperty extends Property<Boolean> { }",
    );
    stubs.insert(
        "net/minecraft/world/level/block/state/properties/BlockStateProperties.java",
        r#"package net.minecraft.world.level.block.state.properties;
public class BlockStateProperties {
    public static final EnumProperty<Object> HORIZONTAL_FACING = null;
    public static final BooleanProperty LIT = null;
}"#,
    );
    let mod_sources = sources! {
        // Runes' shape: a vanilla `CraftingTableBlock` subclass that adds `facing`.
        "demo/Altar.java" => r#"package demo;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.CraftingTableBlock;
import net.minecraft.world.level.block.state.BlockBehaviour;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.block.state.StateDefinition;
import net.minecraft.world.level.block.state.properties.BlockStateProperties;
public final class Altar extends CraftingTableBlock {
    public Altar(BlockBehaviour.Properties properties) { super(properties); }
    @Override
    protected void createBlockStateDefinition(StateDefinition.Builder<Block, BlockState> builder) {
        builder.add(BlockStateProperties.HORIZONTAL_FACING);
    }
}"#,
        // An override a jar subclass inherits from its jar superclass.
        "demo/LampBase.java" => r#"package demo;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.state.BlockBehaviour;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.block.state.StateDefinition;
import net.minecraft.world.level.block.state.properties.BlockStateProperties;
public class LampBase extends Block {
    public LampBase(BlockBehaviour.Properties properties) { super(properties); }
    @Override
    protected void createBlockStateDefinition(StateDefinition.Builder<Block, BlockState> builder) {
        builder.add(BlockStateProperties.LIT);
    }
    { registerDefaultState(defaultBlockState().setValue(BlockStateProperties.LIT, Boolean.valueOf(false))); }
}"#,
        "demo/Lamp.java" => r#"package demo;
import net.minecraft.world.level.block.state.BlockBehaviour;
public final class Lamp extends LampBase {
    public Lamp(BlockBehaviour.Properties properties) { super(properties); }
}"#,
        // No jar override, and 26.3's `CraftingTableBlock` inherits `Block`'s empty
        // `createBlockStateDefinition`: exactly no state property, nothing counted.
        "demo/Inherited.java" => r#"package demo;
import net.minecraft.world.level.block.CraftingTableBlock;
import net.minecraft.world.level.block.state.BlockBehaviour;
public final class Inherited extends CraftingTableBlock {
    public Inherited(BlockBehaviour.Properties properties) { super(properties); }
}"#,
        // No jar override, but 26.3's `FurnaceBlock` inherits `AbstractFurnaceBlock`'s definition
        // (FACING, LIT), which the runtime does not run: counted, and no state set is claimed.
        // A vanilla block class in a subpackage (`block/piston/`) that declares its own
        // definition (FACING, EXTENDED in 26.3): counted the same way.
        "demo/Pusher.java" => r#"package demo;
import net.minecraft.world.level.block.piston.PistonBaseBlock;
import net.minecraft.world.level.block.state.BlockBehaviour;
public final class Pusher extends PistonBaseBlock {
    public Pusher(BlockBehaviour.Properties properties) { super(properties); }
}"#,
        "demo/Smelter.java" => r#"package demo;
import net.minecraft.world.level.block.FurnaceBlock;
import net.minecraft.world.level.block.state.BlockBehaviour;
public final class Smelter extends FurnaceBlock {
    public Smelter(BlockBehaviour.Properties properties) { super(properties); }
}"#,
        "demo/StateMod.java" => r#"package demo;
import net.fabricmc.api.ModInitializer;
import net.minecraft.core.Registry;
import net.minecraft.core.registries.Registries;
import net.minecraft.resources.Identifier;
import net.minecraft.world.level.block.state.BlockBehaviour;
public final class StateMod implements ModInitializer {
    @Override
    public void onInitialize() {
        Registry.register(Registries.BLOCK, Identifier.fromNamespaceAndPath("fixture", "altar"),
                new Altar(BlockBehaviour.Properties.of().strength(2.0F)));
        Registry.register(Registries.BLOCK, Identifier.fromNamespaceAndPath("fixture", "lamp"),
                new Lamp(BlockBehaviour.Properties.of()));
        Registry.register(Registries.BLOCK, Identifier.fromNamespaceAndPath("fixture", "inherited"),
                new Inherited(BlockBehaviour.Properties.of()));
        Registry.register(Registries.BLOCK, Identifier.fromNamespaceAndPath("fixture", "smelter"),
                new Smelter(BlockBehaviour.Properties.of()));
        Registry.register(Registries.BLOCK, Identifier.fromNamespaceAndPath("fixture", "pusher"),
                new Pusher(BlockBehaviour.Properties.of()));
    }
}"#,
    };
    let (root, jar_path) = build_jar(
        "typed-inference-block-states",
        &stubs,
        &mod_sources,
        br#"{"id":"fixture-states","version":"1.0","entrypoints":{"main":"demo.StateMod"}}"#,
        None,
    );
    let jar = NativeJar::from_path(&jar_path).unwrap();
    let entry = jar.discover_entrypoint("main").unwrap();
    let mut host = jar.seed_host(StdHost::new());
    jar.execute(&entry.class, entry.method.as_deref(), "main", &mut host)
        .unwrap();
    let properties = |index: usize| match &host.registrations()[index].declaration {
        Some(RegistryDeclaration::Block { properties, .. }) => properties.clone(),
        other => panic!("row {index} is not a typed block: {other:?}"),
    };
    let altar = properties(0);
    assert_eq!(
        altar.get("state_property:facing").map(String::as_str),
        Some("enum|north,south,west,east")
    );
    assert_eq!(
        altar.get("default_state:facing").map(String::as_str),
        Some("north")
    );
    let lamp = properties(1);
    assert_eq!(
        lamp.get("state_property:lit").map(String::as_str),
        Some("boolean|true,false")
    );
    assert_eq!(
        lamp.get("default_state:lit").map(String::as_str),
        Some("false")
    );
    let inherited = properties(2);
    assert!(
        !inherited
            .keys()
            .any(|key| key.starts_with("state_property:"))
    );
    // `FurnaceBlock`'s own constructor is counted (vanilla subclass constructors other than the
    // crafting table's stay unmodelled), so its row stays opaque: no declaration, no state set.
    assert!(host.registrations()[3].declaration.is_none());
    let counted: Vec<String> = host
        .unknown_host_calls()
        .into_iter()
        .map(|(key, _)| key)
        .filter(|key| key.contains("createBlockStateDefinition"))
        .collect();
    assert_eq!(
        counted,
        [
            "net/minecraft/world/level/block/FurnaceBlock#createBlockStateDefinition(Lnet/minecraft/world/level/block/state/StateDefinition$Builder;)V",
            "net/minecraft/world/level/block/piston/PistonBaseBlock#createBlockStateDefinition(Lnet/minecraft/world/level/block/state/StateDefinition$Builder;)V",
        ],
        "only the FurnaceBlock and PistonBaseBlock chains add properties the runtime does not run"
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn state_definition_builder_duplicate_is_counted() {
    run_on_large_stack(|| {
        let mut stubs = game_stubs();
        stubs.insert(
            "net/minecraft/world/level/block/Block.java",
            r#"package net.minecraft.world.level.block;
import net.minecraft.world.level.block.state.BlockBehaviour;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.block.state.StateDefinition;
public class Block {
    public Block(BlockBehaviour.Properties properties) { }
    protected void createBlockStateDefinition(StateDefinition.Builder<Block, BlockState> builder) { }
}"#,
        );
        stubs.insert(
            "net/minecraft/world/level/block/state/BlockState.java",
            "package net.minecraft.world.level.block.state; public class BlockState { }",
        );
        stubs.insert(
            "net/minecraft/world/level/block/state/StateDefinition.java",
            r#"package net.minecraft.world.level.block.state;
import net.minecraft.world.level.block.state.properties.Property;
public class StateDefinition<O, S> {
    public static class Builder<O, S> {
        public Builder<O, S> add(Property<?>... properties) { return this; }
    }
}"#,
        );
        stubs.insert(
            "net/minecraft/world/level/block/state/properties/Property.java",
            "package net.minecraft.world.level.block.state.properties; public abstract class Property<T> { }",
        );
        stubs.insert(
            "net/minecraft/world/level/block/state/properties/BooleanProperty.java",
            "package net.minecraft.world.level.block.state.properties; public final class BooleanProperty extends Property<Boolean> { }",
        );
        stubs.insert(
            "net/minecraft/world/level/block/state/properties/BlockStateProperties.java",
            r#"package net.minecraft.world.level.block.state.properties;
public class BlockStateProperties { public static final BooleanProperty LIT = null; }"#,
        );
        let mod_sources = sources! {
            "demo/Bad.java" => r#"package demo;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.state.BlockBehaviour;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.block.state.StateDefinition;
import net.minecraft.world.level.block.state.properties.BlockStateProperties;
public final class Bad extends Block {
    public Bad(BlockBehaviour.Properties properties) { super(properties); }
    @Override protected void createBlockStateDefinition(StateDefinition.Builder<Block, BlockState> builder) {
        builder.add(BlockStateProperties.LIT, BlockStateProperties.LIT);
    }
}"#,
            "demo/BadMod.java" => r#"package demo;
import net.fabricmc.api.ModInitializer;
import net.minecraft.world.level.block.state.BlockBehaviour;
public final class BadMod implements ModInitializer {
    @Override public void onInitialize() {
        try { new Bad(BlockBehaviour.Properties.of()); }
        catch (IllegalArgumentException error) { System.out.println(error.getMessage()); }
    }
}"#,
        };
        let (root, jar_path) = build_jar(
            "typed-inference-builder-validation",
            &stubs,
            &mod_sources,
            br#"{"id":"fixture-builder","version":"1.0","entrypoints":{"main":"demo.BadMod"}}"#,
            None,
        );
        let jar = NativeJar::from_path(&jar_path).unwrap();
        let entry = jar.discover_entrypoint("main").unwrap();
        let mut host = jar.seed_host(StdHost::new());
        jar.execute(&entry.class, entry.method.as_deref(), "main", &mut host)
            .unwrap();
        // Java throws `IllegalArgumentException` whose message names the block's `toString`
        // mid-construction, which the host cannot spell: the duplicate is counted instead of an
        // exception with an invented message (so nothing is printed here).
        assert!(host.unknown_host_calls().iter().any(|(call, _)| call
            == "net/minecraft/world/level/block/state/StateDefinition$Builder#validateProperty duplicate lit"));
        let _ = fs::remove_dir_all(root);
    });
}
