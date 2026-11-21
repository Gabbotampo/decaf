//! End-to-end fixtures for `VoxelShape` modelling and post-load `Block.getShape` evaluation
//! across every state of a block.
//!
//! Expected box arrays were captured from the real Minecraft 26.3 server jar on OpenJDK 25.0.5+8
//! (`Block.box`, `Shapes.or`, `VoxelShape.toAabbs`, `VoxelShape.bounds`, `VoxelShape.move`,
//! `Shapes.join`, and `BlockBehaviour.getShape`).

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use std::collections::BTreeMap;
use std::path::PathBuf;

use horizon_native_java::{ExecutionBudget, NativeJar, StdHost, Value, content};
use serde_json::json;

/// Minimal Minecraft 26.3 stubs compiled onto the classpath and NOT packaged into the fixture
/// jar (`Fixture::build` packs only `demo/`), so every call crosses the host runtime boundary.
fn block_shape_stubs() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            "net/fabricmc/api/ModInitializer.java",
            "package net.fabricmc.api; public interface ModInitializer { void onInitialize(); }",
        ),
        (
            "net/minecraft/resources/Identifier.java",
            r#"package net.minecraft.resources;
public final class Identifier {
    private Identifier() { }
    public static Identifier fromNamespaceAndPath(String namespace, String path) { return null; }
}"#,
        ),
        (
            "net/minecraft/core/Registry.java",
            r#"package net.minecraft.core;
import net.minecraft.resources.Identifier;
public interface Registry<T> {
    static <V, T extends V> T register(Registry<V> registry, Identifier id, T value) { return value; }
}"#,
        ),
        (
            "net/minecraft/core/registries/BuiltInRegistries.java",
            r#"package net.minecraft.core.registries;
import net.minecraft.core.Registry;
import net.minecraft.world.level.block.Block;
public final class BuiltInRegistries {
    public static final Registry<Block> BLOCK = null;
}"#,
        ),
        (
            "net/minecraft/core/Direction.java",
            r#"package net.minecraft.core;
public enum Direction {
    DOWN, UP, NORTH, SOUTH, WEST, EAST;
    public enum Axis {
        X, Y, Z;
        public boolean isHorizontal() { return false; }
        public boolean isVertical() { return false; }
        public String getName() { return null; }
        public String getSerializedName() { return null; }
    }
    public int get3DDataValue() { return 0; }
    public int get2DDataValue() { return 0; }
    public Direction getOpposite() { return null; }
    public Direction getClockWise() { return null; }
    public Direction getCounterClockWise() { return null; }
    public Axis getAxis() { return null; }
    public int getStepX() { return 0; }
    public int getStepY() { return 0; }
    public int getStepZ() { return 0; }
    public String getName() { return null; }
    public String getSerializedName() { return null; }
    public static Direction from3DDataValue(int i) { return null; }
    public static Direction from2DDataValue(int i) { return null; }
}"#,
        ),
        (
            "net/minecraft/core/BlockPos.java",
            r#"package net.minecraft.core;
public class BlockPos {
    public static final BlockPos ZERO = new BlockPos();
    public int getX() { return 0; }
    public int getY() { return 0; }
    public int getZ() { return 0; }
}"#,
        ),
        (
            "net/minecraft/world/level/BlockGetter.java",
            r#"package net.minecraft.world.level;
import net.minecraft.core.BlockPos;
import net.minecraft.world.level.block.state.BlockState;
public interface BlockGetter {
    BlockState getBlockState(BlockPos pos);
}"#,
        ),
        (
            "net/minecraft/world/phys/AABB.java",
            r#"package net.minecraft.world.phys;
public class AABB {
    public final double minX;
    public final double minY;
    public final double minZ;
    public final double maxX;
    public final double maxY;
    public final double maxZ;
    public AABB(double x0, double y0, double z0, double x1, double y1, double z1) {
        this.minX = x0; this.minY = y0; this.minZ = z0;
        this.maxX = x1; this.maxY = y1; this.maxZ = z1;
    }
    public AABB move(double dx, double dy, double dz) { return null; }
}"#,
        ),
        (
            "net/minecraft/world/phys/shapes/BooleanOp.java",
            r#"package net.minecraft.world.phys.shapes;
public interface BooleanOp {
    BooleanOp FALSE = null;
    BooleanOp NOT_OR = null;
    BooleanOp ONLY_SECOND = null;
    BooleanOp NOT_FIRST = null;
    BooleanOp ONLY_FIRST = null;
    BooleanOp NOT_SECOND = null;
    BooleanOp NOT_SAME = null;
    BooleanOp NOT_AND = null;
    BooleanOp AND = null;
    BooleanOp SAME = null;
    BooleanOp SECOND = null;
    BooleanOp CAUSES = null;
    BooleanOp FIRST = null;
    BooleanOp CAUSED_BY = null;
    BooleanOp OR = null;
    BooleanOp TRUE = null;
}"#,
        ),
        (
            "net/minecraft/world/phys/shapes/CollisionContext.java",
            r#"package net.minecraft.world.phys.shapes;
public interface CollisionContext {
    static CollisionContext empty() { return null; }
    boolean isDescending();
}"#,
        ),
        (
            "net/minecraft/world/phys/shapes/VoxelShape.java",
            r#"package net.minecraft.world.phys.shapes;
import java.util.List;
import net.minecraft.core.BlockPos;
import net.minecraft.core.Direction;
import net.minecraft.world.phys.AABB;
public abstract class VoxelShape {
    public boolean isEmpty() { return false; }
    public AABB bounds() { return null; }
    public List<AABB> toAabbs() { return null; }
    public VoxelShape optimize() { return this; }
    public VoxelShape move(double dx, double dy, double dz) { return this; }
    public double min(Direction.Axis axis) { return 0.0; }
    public double max(Direction.Axis axis) { return 0.0; }
    public Object clip(Object from, Object to, BlockPos pos) { return null; }
}"#,
        ),
        (
            "net/minecraft/world/phys/shapes/Shapes.java",
            r#"package net.minecraft.world.phys.shapes;
import net.minecraft.world.phys.AABB;
public final class Shapes {
    public static VoxelShape empty() { return null; }
    public static VoxelShape block() { return null; }
    public static VoxelShape box(double x0, double y0, double z0, double x1, double y1, double z1) { return null; }
    public static VoxelShape create(double x0, double y0, double z0, double x1, double y1, double z1) { return null; }
    public static VoxelShape create(AABB aabb) { return null; }
    public static VoxelShape or(VoxelShape first, VoxelShape second) { return null; }
    public static VoxelShape or(VoxelShape first, VoxelShape... others) { return null; }
    public static VoxelShape join(VoxelShape first, VoxelShape second, BooleanOp op) { return null; }
    public static VoxelShape joinUnoptimized(VoxelShape first, VoxelShape second, BooleanOp op) { return null; }
}"#,
        ),
        (
            "net/minecraft/world/level/block/state/properties/Property.java",
            "package net.minecraft.world.level.block.state.properties; public abstract class Property<T extends Comparable<T>> { }",
        ),
        (
            "net/minecraft/world/level/block/state/properties/EnumProperty.java",
            "package net.minecraft.world.level.block.state.properties; public final class EnumProperty<T extends Enum<T>> extends Property<T> { }",
        ),
        (
            "net/minecraft/world/level/block/state/properties/BooleanProperty.java",
            "package net.minecraft.world.level.block.state.properties; public final class BooleanProperty extends Property<Boolean> { }",
        ),
        (
            "net/minecraft/world/level/block/state/properties/BlockStateProperties.java",
            r#"package net.minecraft.world.level.block.state.properties;
import net.minecraft.core.Direction;
public final class BlockStateProperties {
    public static final EnumProperty<Direction> HORIZONTAL_FACING = null;
    public static final BooleanProperty LIT = null;
}"#,
        ),
        (
            "net/minecraft/world/level/block/state/StateDefinition.java",
            r#"package net.minecraft.world.level.block.state;
import net.minecraft.world.level.block.state.properties.Property;
public class StateDefinition<O, S> {
    public static class Builder<O, S> {
        public Builder<O, S> add(Property<?>... properties) { return this; }
    }
}"#,
        ),
        (
            "net/minecraft/world/level/block/state/BlockState.java",
            r#"package net.minecraft.world.level.block.state;
import java.util.Optional;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.state.properties.Property;
public class BlockState {
    public <T extends Comparable<T>> T getValue(Property<T> property) { return null; }
    public <T extends Comparable<T>> boolean hasProperty(Property<T> property) { return false; }
    public <T extends Comparable<T>> Optional<T> getOptionalValue(Property<T> property) { return Optional.empty(); }
    public <T extends Comparable<T>> T getValueOrElse(Property<T> property, T fallback) { return fallback; }
    public <T extends Comparable<T>> BlockState setValue(Property<T> property, T value) { return this; }
    public Block getBlock() { return null; }
}"#,
        ),
        (
            "net/minecraft/world/level/block/state/BlockBehaviour.java",
            r#"package net.minecraft.world.level.block.state;
import net.minecraft.core.BlockPos;
import net.minecraft.world.level.BlockGetter;
import net.minecraft.world.phys.shapes.CollisionContext;
import net.minecraft.world.phys.shapes.Shapes;
import net.minecraft.world.phys.shapes.VoxelShape;
public abstract class BlockBehaviour {
    public static class Properties {
        public static Properties of() { return new Properties(); }
    }
    protected VoxelShape getShape(BlockState state, BlockGetter level, BlockPos pos, CollisionContext context) {
        return Shapes.block();
    }
}"#,
        ),
        (
            "net/minecraft/world/level/block/Block.java",
            r#"package net.minecraft.world.level.block;
import net.minecraft.world.level.block.state.BlockBehaviour;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.block.state.StateDefinition;
import net.minecraft.world.phys.shapes.VoxelShape;
public class Block extends BlockBehaviour {
    public Block(Properties properties) { }
    public static VoxelShape box(double x0, double y0, double z0, double x1, double y1, double z1) { return null; }
    protected void createBlockStateDefinition(StateDefinition.Builder<Block, BlockState> builder) { }
    public final BlockState defaultBlockState() { return null; }
    protected final void registerDefaultState(BlockState state) { }
}"#,
        ),
        (
            "net/minecraft/world/level/block/CraftingTableBlock.java",
            r#"package net.minecraft.world.level.block;
public class CraftingTableBlock extends Block {
    public CraftingTableBlock(Properties properties) { super(properties); }
}"#,
        ),
        (
            "net/minecraft/world/level/block/EnchantingTableBlock.java",
            r#"package net.minecraft.world.level.block;
public class EnchantingTableBlock extends Block {
    public EnchantingTableBlock(Properties properties) { super(properties); }
}"#,
        ),
    ]
}

fn registration_value(host: &StdHost, id: &str) -> Value {
    host.registrations()
        .iter()
        .find(|entry| entry.registry == "minecraft:block" && entry.id == id)
        .unwrap_or_else(|| panic!("missing block registration {id}"))
        .value
        .clone()
}

/// Three primary blocks plus negative/counted cases:
/// 1. `StaticOrBlock`: static `Shapes.or(Block.box(...), Block.box(...))` returned from `getShape`.
/// 2. `FacingArrayBlock`: facing-dependent shapes chosen inside `getShape` from a static
///    `VoxelShape[]` indexed by `state.getValue(FACING).get2DDataValue()`.
/// 3. `PlainBlock` & `InheritedTableBlock`: no `getShape` override; inherits MC 26.3
///    `BlockBehaviour.getShape` (`Shapes.block()`, unit cube) without invoking jar bytecode.
/// 4. `WorldReadingBlock` & `ContextReadingBlock`: read unmodelled `BlockGetter` /
///    `CollisionContext` methods -> counted on `host` and receive `states: None`.
/// 5. `VanillaOverridingSuperBlock`: extends `EnchantingTableBlock` (overrides `getShape` in
///    vanilla 26.3) without a jar override -> counted on `host` and receives `states: None`.
#[test]
fn block_shapes_from_get_shape_across_all_states() {
    let mut sources = block_shape_stubs();
    sources.extend([
        (
            "demo/StaticOrBlock.java",
            r#"package demo;
import net.minecraft.core.BlockPos;
import net.minecraft.world.level.BlockGetter;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.block.state.StateDefinition;
import net.minecraft.world.level.block.state.properties.BlockStateProperties;
import net.minecraft.world.phys.shapes.CollisionContext;
import net.minecraft.world.phys.shapes.Shapes;
import net.minecraft.world.phys.shapes.VoxelShape;
public final class StaticOrBlock extends Block {
    private static final VoxelShape BASE = Block.box(2.0, 0.0, 2.0, 14.0, 4.0, 14.0);
    private static final VoxelShape PILLAR = Block.box(5.0, 4.0, 5.0, 11.0, 12.0, 11.0);
    private static final VoxelShape SHAPE = Shapes.or(BASE, PILLAR);
    public StaticOrBlock(Properties properties) { super(properties); }
    @Override
    protected void createBlockStateDefinition(StateDefinition.Builder<Block, BlockState> builder) {
        builder.add(BlockStateProperties.HORIZONTAL_FACING);
    }
    @Override
    protected VoxelShape getShape(BlockState state, BlockGetter level, BlockPos pos, CollisionContext context) {
        return SHAPE;
    }
}"#,
        ),
        (
            "demo/FacingArrayBlock.java",
            r#"package demo;
import net.minecraft.core.BlockPos;
import net.minecraft.core.Direction;
import net.minecraft.world.level.BlockGetter;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.block.state.StateDefinition;
import net.minecraft.world.level.block.state.properties.BlockStateProperties;
import net.minecraft.world.level.block.state.properties.EnumProperty;
import net.minecraft.world.phys.shapes.CollisionContext;
import net.minecraft.world.phys.shapes.VoxelShape;
public final class FacingArrayBlock extends Block {
    private static final EnumProperty<Direction> FACING = BlockStateProperties.HORIZONTAL_FACING;
    // Indexed by Direction.get2DDataValue(): SOUTH=0, WEST=1, NORTH=2, EAST=3 (MC 26.3).
    private static final VoxelShape[] SHAPES = new VoxelShape[] {
        Block.box(2.0, 0.0, 10.0, 14.0, 12.0, 16.0),
        Block.box(0.0, 0.0, 2.0, 6.0, 12.0, 14.0),
        Block.box(2.0, 0.0, 0.0, 14.0, 12.0, 6.0),
        Block.box(10.0, 0.0, 2.0, 16.0, 12.0, 14.0),
    };
    public FacingArrayBlock(Properties properties) { super(properties); }
    @Override
    protected void createBlockStateDefinition(StateDefinition.Builder<Block, BlockState> builder) {
        builder.add(FACING);
    }
    @Override
    protected VoxelShape getShape(BlockState state, BlockGetter level, BlockPos pos, CollisionContext context) {
        return SHAPES[state.getValue(FACING).get2DDataValue()];
    }
}"#,
        ),
        (
            "demo/PlainBlock.java",
            r#"package demo;
import net.minecraft.world.level.block.Block;
public final class PlainBlock extends Block {
    public PlainBlock(Properties properties) { super(properties); }
}"#,
        ),
        (
            "demo/InheritedTableBlock.java",
            r#"package demo;
import net.minecraft.world.level.block.CraftingTableBlock;
public final class InheritedTableBlock extends CraftingTableBlock {
    public InheritedTableBlock(Properties properties) { super(properties); }
}"#,
        ),
        (
            "demo/WorldReadingBlock.java",
            r#"package demo;
import net.minecraft.core.BlockPos;
import net.minecraft.world.level.BlockGetter;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.phys.shapes.CollisionContext;
import net.minecraft.world.phys.shapes.Shapes;
import net.minecraft.world.phys.shapes.VoxelShape;
public final class WorldReadingBlock extends Block {
    public WorldReadingBlock(Properties properties) { super(properties); }
    @Override
    protected VoxelShape getShape(BlockState state, BlockGetter level, BlockPos pos, CollisionContext context) {
        BlockState neighbor = level.getBlockState(pos);
        return neighbor == null ? Shapes.empty() : Shapes.block();
    }
}"#,
        ),
        (
            "demo/ContextReadingBlock.java",
            r#"package demo;
import net.minecraft.core.BlockPos;
import net.minecraft.world.level.BlockGetter;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.phys.shapes.CollisionContext;
import net.minecraft.world.phys.shapes.Shapes;
import net.minecraft.world.phys.shapes.VoxelShape;
public final class ContextReadingBlock extends Block {
    public ContextReadingBlock(Properties properties) { super(properties); }
    @Override
    protected VoxelShape getShape(BlockState state, BlockGetter level, BlockPos pos, CollisionContext context) {
        return context.isDescending() ? Block.box(0, 0, 0, 16, 8, 16) : Shapes.block();
    }
}"#,
        ),
        (
            "demo/VanillaOverridingSuperBlock.java",
            r#"package demo;
import net.minecraft.world.level.block.EnchantingTableBlock;
public final class VanillaOverridingSuperBlock extends EnchantingTableBlock {
    public VanillaOverridingSuperBlock(Properties properties) { super(properties); }
}"#,
        ),
        (
            "demo/Probe.java",
            r#"package demo;
import net.fabricmc.api.ModInitializer;
import net.minecraft.core.Registry;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.resources.Identifier;
import net.minecraft.world.level.block.state.BlockBehaviour;
public final class Probe implements ModInitializer {
    private static Identifier id(String path) { return Identifier.fromNamespaceAndPath("fixture", path); }
    @Override
    public void onInitialize() {
        Registry.register(BuiltInRegistries.BLOCK, id("static_or"), new StaticOrBlock(BlockBehaviour.Properties.of()));
        Registry.register(BuiltInRegistries.BLOCK, id("facing_array"), new FacingArrayBlock(BlockBehaviour.Properties.of()));
        Registry.register(BuiltInRegistries.BLOCK, id("plain"), new PlainBlock(BlockBehaviour.Properties.of()));
        Registry.register(BuiltInRegistries.BLOCK, id("inherited_table"), new InheritedTableBlock(BlockBehaviour.Properties.of()));
        Registry.register(BuiltInRegistries.BLOCK, id("world_reading"), new WorldReadingBlock(BlockBehaviour.Properties.of()));
        Registry.register(BuiltInRegistries.BLOCK, id("context_reading"), new ContextReadingBlock(BlockBehaviour.Properties.of()));
        Registry.register(BuiltInRegistries.BLOCK, id("vanilla_overriding"), new VanillaOverridingSuperBlock(BlockBehaviour.Properties.of()));
    }
}"#,
        ),
    ]);

    let fixture = javac_fixture::Fixture::build("block-shapes-states", &sources);
    let mut host = fixture.jar.seed_host(StdHost::new());
    let entry = fixture.jar.discover_entrypoint("main").unwrap();
    fixture
        .jar
        .execute(&entry.class, entry.method.as_deref(), "main", &mut host)
        .unwrap();

    // 1. Static Shapes.or block: all 4 facing states agree on MC 26.3's exact 2 boxes, 0 counted calls.
    let expected_static_or = vec![
        [0.125, 0.0, 0.125, 0.875, 0.25, 0.875],
        [0.3125, 0.25, 0.3125, 0.6875, 0.75, 0.6875],
    ];
    let static_or_block = registration_value(&host, "fixture:static_or");
    let unknown_before = host.unknown_host_call_count();
    let static_or = fixture
        .jar
        .block_shapes(&mut host, &static_or_block)
        .unwrap();
    assert_eq!(host.unknown_host_call_count(), unknown_before);
    assert!(static_or.overridden);
    assert_eq!(static_or.properties, vec!["facing"]);
    let static_states = static_or.states.as_ref().expect("static_or shapes");
    assert_eq!(static_states.len(), 4);
    for row in static_states {
        assert_eq!(row.boxes, expected_static_or);
    }

    // 2. Facing-dependent array block: 4 distinct shapes matching MC 26.3 captures, 0 counted calls.
    let expected_facing = BTreeMap::from([
        (
            "north".to_owned(),
            vec![[0.125, 0.0, 0.0, 0.875, 0.75, 0.375]],
        ),
        (
            "south".to_owned(),
            vec![[0.125, 0.0, 0.625, 0.875, 0.75, 1.0]],
        ),
        (
            "west".to_owned(),
            vec![[0.0, 0.0, 0.125, 0.375, 0.75, 0.875]],
        ),
        (
            "east".to_owned(),
            vec![[0.625, 0.0, 0.125, 1.0, 0.75, 0.875]],
        ),
    ]);
    let facing_block = registration_value(&host, "fixture:facing_array");
    let unknown_before = host.unknown_host_call_count();
    let facing = fixture.jar.block_shapes(&mut host, &facing_block).unwrap();
    assert_eq!(host.unknown_host_call_count(), unknown_before);
    assert!(facing.overridden);
    assert_eq!(facing.properties, vec!["facing"]);
    let facing_states = facing.states.as_ref().expect("facing_array shapes");
    assert_eq!(facing_states.len(), 4);
    let actual_facing: BTreeMap<String, Vec<[f64; 6]>> = facing_states
        .iter()
        .map(|row| (row.state["facing"].clone(), row.boxes.clone()))
        .collect();
    assert_eq!(actual_facing, expected_facing);

    // 3. No getShape override (Block and CraftingTableBlock both inherit BlockBehaviour.getShape):
    // full unit cube, overridden == false, 0 counted calls.
    for id in ["fixture:plain", "fixture:inherited_table"] {
        let block = registration_value(&host, id);
        let unknown_before = host.unknown_host_call_count();
        let plain = fixture.jar.block_shapes(&mut host, &block).unwrap();
        assert_eq!(host.unknown_host_call_count(), unknown_before);
        assert!(!plain.overridden);
        let states = plain.states.as_ref().expect("inherited full cube");
        assert_eq!(states.len(), 1);
        assert_eq!(states[0].boxes, vec![[0.0, 0.0, 0.0, 1.0, 1.0, 1.0]]);
        assert!(plain.to_content_shapes().is_none());
    }

    // 4. Unmodelled BlockGetter read -> counted and no shape.
    let world_block = registration_value(&host, "fixture:world_reading");
    let unknown_before = host.unknown_host_call_count();
    let world_reading = fixture.jar.block_shapes(&mut host, &world_block).unwrap();
    assert!(
        host.unknown_host_call_count() > unknown_before,
        "BlockGetter.getBlockState must be counted"
    );
    assert!(world_reading.overridden);
    assert!(world_reading.states.is_none());

    // 5. Unmodelled CollisionContext read -> counted and no shape.
    let ctx_block = registration_value(&host, "fixture:context_reading");
    let unknown_before = host.unknown_host_call_count();
    let ctx_reading = fixture.jar.block_shapes(&mut host, &ctx_block).unwrap();
    assert!(
        host.unknown_host_call_count() > unknown_before,
        "CollisionContext.isDescending must be counted"
    );
    assert!(ctx_reading.overridden);
    assert!(ctx_reading.states.is_none());

    // 6. Vanilla superclass that overrides getShape (EnchantingTableBlock) -> counted, no shape.
    let vanilla_block = registration_value(&host, "fixture:vanilla_overriding");
    let unknown_before = host.unknown_host_call_count();
    let vanilla_override = fixture.jar.block_shapes(&mut host, &vanilla_block).unwrap();
    assert!(
        host.unknown_host_call_count() > unknown_before,
        "EnchantingTableBlock.getShape must be counted, not guessed as full cube"
    );
    assert!(!vanilla_override.overridden);
    assert!(vanilla_override.states.is_none());

    // 7. Export through content::declarations (same path as tools/bake.py and mod/src/content.rs).
    fixture
        .jar
        .populate_block_shapes(&mut host, &ExecutionBudget::UNBOUNDED);
    let exported: BTreeMap<String, serde_json::Value> = content::declarations(host.registrations())
        .into_iter()
        .map(|decl| (decl.id().to_owned(), decl.file_json()))
        .collect();

    assert_eq!(
        exported["fixture:static_or"]["shapes"],
        json!({ "default": expected_static_or })
    );
    assert!(
        exported["fixture:static_or"]
            .get("properties")
            .and_then(|p| p.get("shapes"))
            .is_none(),
        "shapes must be top-level, never inside properties: {}",
        exported["fixture:static_or"]
    );

    assert_eq!(
        exported["fixture:facing_array"]["shapes"],
        json!({
            "properties": ["facing"],
            "values": {
                "east": [[0.625, 0.0, 0.125, 1.0, 0.75, 0.875]],
                "north": [[0.125, 0.0, 0.0, 0.875, 0.75, 0.375]],
                "south": [[0.125, 0.0, 0.625, 0.875, 0.75, 1.0]],
                "west": [[0.0, 0.0, 0.125, 0.375, 0.75, 0.875]],
            }
        })
    );
    assert!(exported["fixture:plain"].get("shapes").is_none());
    assert!(exported["fixture:inherited_table"].get("shapes").is_none());
    assert!(exported["fixture:world_reading"].get("shapes").is_none());
    assert!(exported["fixture:context_reading"].get("shapes").is_none());
    assert!(
        exported
            .get("fixture:vanilla_overriding")
            .and_then(|file| file.get("shapes"))
            .is_none()
    );
}

/// Mod-observable queries on `VoxelShape` and `Shapes` match real MC 26.3 captures (or are
/// counted when unmodelled, such as `VoxelShape.clip`).
#[test]
fn voxel_shape_observable_queries_match_mc263_and_count_unmodelled() {
    let mut sources = block_shape_stubs();
    sources.push((
        "demo/Probe.java",
        r#"package demo;
import java.util.List;
import net.minecraft.core.BlockPos;
import net.minecraft.core.Direction;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.phys.AABB;
import net.minecraft.world.phys.shapes.BooleanOp;
import net.minecraft.world.phys.shapes.Shapes;
import net.minecraft.world.phys.shapes.VoxelShape;
public final class Probe {
    public static String checkModelled() {
        StringBuilder out = new StringBuilder();
        VoxelShape full = Block.box(0, 0, 0, 16, 16, 16);
        VoxelShape empty = Block.box(0, 0, 0, 0, 0, 0);
        out.append("fullEq=").append(full == Shapes.block()).append(';');
        out.append("emptyEq=").append(empty == Shapes.empty()).append(';');
        out.append("emptyIsEmpty=").append(empty.isEmpty()).append(';');
        out.append("fullIsEmpty=").append(full.isEmpty()).append(';');
        out.append("emptyStr=").append(empty.toString()).append(';');
        out.append("fullStr=").append(full.toString()).append(';');

        VoxelShape s1 = Block.box(0, 0, 0, 10, 10, 10);
        VoxelShape s2 = Block.box(6, 0, 0, 16, 10, 10);
        VoxelShape and = Shapes.join(s1, s2, BooleanOp.AND);
        AABB ab = and.bounds();
        out.append("andBounds=")
            .append(ab.minX).append(',').append(ab.minY).append(',').append(ab.minZ).append(',')
            .append(ab.maxX).append(',').append(ab.maxY).append(',').append(ab.maxZ).append(';');

        VoxelShape moved = Block.box(0, 0, 0, 8, 8, 8).move(0.5, 0.25, 0.0).optimize();
        List<AABB> aabbs = moved.toAabbs();
        AABB mb = aabbs.get(0);
        out.append("moved=")
            .append(aabbs.size()).append(':')
            .append(mb.minX).append(',').append(mb.minY).append(',').append(mb.minZ).append(',')
            .append(mb.maxX).append(',').append(mb.maxY).append(',').append(mb.maxZ).append(';');
        out.append("minX=").append(moved.min(Direction.Axis.X))
            .append(",maxY=").append(moved.max(Direction.Axis.Y)).append(';');

        try {
            Shapes.box(1.0, 0.0, 0.0, 0.5, 1.0, 1.0);
            out.append("badBox=ok;");
        } catch (IllegalArgumentException e) {
            out.append("badBox=").append(e.getMessage()).append(';');
        }
        try {
            Shapes.empty().bounds();
            out.append("emptyBounds=ok;");
        } catch (UnsupportedOperationException e) {
            out.append("emptyBounds=").append(e.getMessage()).append(';');
        }
        return out.toString();
    }

    public static Object checkUnmodelledClip() {
        return Shapes.block().clip(null, null, BlockPos.ZERO);
    }
}"#,
    ));

    let fixture = javac_fixture::Fixture::build("voxel-shape-queries", &sources);
    let mut host = fixture.jar.seed_host(StdHost::new());

    let unknown_before = host.unknown_host_call_count();
    let answer = fixture
        .jar
        .invoke_method_with_args(
            "demo/Probe",
            "checkModelled",
            "()Ljava/lang/String;",
            None,
            &[],
            &mut host,
        )
        .unwrap();
    assert_eq!(host.unknown_host_call_count(), unknown_before);
    assert_eq!(
        answer,
        Value::Ref(Some(
            "fullEq=true;\
             emptyEq=true;\
             emptyIsEmpty=true;\
             fullIsEmpty=false;\
             emptyStr=EMPTY;\
             fullStr=VoxelShape[AABB[0.0, 0.0, 0.0] -> [1.0, 1.0, 1.0]];\
             andBounds=0.375,0.0,0.0,0.625,0.625,0.625;\
             moved=1:0.5,0.25,0.0,1.0,0.75,0.5;\
             minX=0.5,maxY=0.75;\
             badBox=The min values need to be smaller or equals to the max values;\
             emptyBounds=No bounds for empty shape.;"
                .to_owned()
        ))
    );

    let unknown_before = host.unknown_host_call_count();
    let _ = fixture
        .jar
        .invoke_method_with_args(
            "demo/Probe",
            "checkUnmodelledClip",
            "()Ljava/lang/Object;",
            None,
            &[],
            &mut host,
        )
        .unwrap();
    assert!(
        host.unknown_host_call_count() > unknown_before,
        "unmodelled VoxelShape.clip must be counted"
    );
}

/// Real-jar verification against `runes-fabric-1.3.2+26.3.jar`:
///
/// `net/runes/block/RuneCraftingBlock` (`runes-javap.txt` lines 1113-1116, 1196-1232):
/// * `<clinit>` offset 85..102: `TOP_SHAPE    = Block.box(1.0, 12.0, 1.0, 15.0, 16.0, 15.0)`
/// * `<clinit>` offset 105..126: `MIDDLE_SHAPE = Block.box(4.0, 3.0, 4.0, 12.0, 12.0, 12.0)`
/// * `<clinit>` offset 129..144: `BOTTOM_SHAPE = Block.box(1.0, 0.0, 1.0, 15.0, 3.0, 15.0)`
/// * `<clinit>` offset 147..169: `SHAPE        = Shapes.or(TOP_SHAPE, MIDDLE_SHAPE, BOTTOM_SHAPE)`
/// * `getShape` offset 0..3: returns `SHAPE`.
/// * `createBlockStateDefinition` adds `HORIZONTAL_FACING` (`north`, `south`, `west`, `east`).
///
/// Expected boxes below were captured directly from real MC 26.3 `SHAPE.toAabbs()` on JDK 25.0.5+8.
#[test]
fn real_jar_runes_altar_shapes_match_mc263_capture_with_zero_counted_calls() {
    std::thread::Builder::new()
        .name("real-jar-runes-block-shapes".into())
        .stack_size(16 << 20)
        .spawn(real_jar_runes_altar_shapes_body)
        .unwrap()
        .join()
        .unwrap();
}

fn real_jar_runes_altar_shapes_body() {
    let Some(jar_path) = std::env::var_os("RUNES_JAR").map(PathBuf::from) else {
        eprintln!("RUNES_JAR is unset; real Runes test skipped");
        return;
    };
    assert!(
        jar_path.is_file(),
        "RUNES_JAR does not exist: {}",
        jar_path.display()
    );

    let jar = NativeJar::from_path(&jar_path).unwrap();
    let mut host = jar.seed_host(StdHost::new());
    let entry = jar.discover_entrypoint("main").unwrap();
    jar.execute(&entry.class, entry.method.as_deref(), "main", &mut host)
        .unwrap();

    let altar = registration_value(&host, "runes:crafting_altar");
    let unknown_before = host.unknown_host_call_count();
    let shapes = jar.block_shapes(&mut host, &altar).unwrap();
    let unknown_after = host.unknown_host_call_count();

    assert_eq!(
        unknown_after - unknown_before,
        0,
        "evaluating RuneCraftingBlock.getShape across all states must incur 0 counted calls"
    );
    assert!(shapes.overridden);
    assert_eq!(shapes.properties, vec!["facing"]);

    // Exact 6-box output of real MC 26.3 `Shapes.or(TOP_SHAPE, MIDDLE_SHAPE, BOTTOM_SHAPE).toAabbs()`.
    let expected_boxes = vec![
        [0.0625, 0.0, 0.0625, 0.9375, 0.1875, 0.9375],
        [0.25, 0.1875, 0.25, 0.75, 1.0, 0.75],
        [0.0625, 0.75, 0.0625, 0.25, 1.0, 0.9375],
        [0.25, 0.75, 0.0625, 0.9375, 1.0, 0.25],
        [0.25, 0.75, 0.75, 0.9375, 1.0, 0.9375],
        [0.75, 0.75, 0.25, 0.9375, 1.0, 0.75],
    ];
    let states = shapes.states.as_ref().expect("altar states resolved");
    assert_eq!(states.len(), 4);
    let seen_facings: Vec<&str> = states
        .iter()
        .map(|row| row.state["facing"].as_str())
        .collect();
    assert_eq!(seen_facings, vec!["north", "south", "west", "east"]);
    for row in states {
        assert_eq!(
            row.boxes, expected_boxes,
            "altar boxes for facing={}",
            row.state["facing"]
        );
    }

    // Export through the full content declaration pipeline (`content.rs` -> `bake.py` / `mod/`).
    jar.populate_block_shapes(&mut host, &ExecutionBudget::UNBOUNDED);
    let altar_decl = content::declarations(host.registrations())
        .into_iter()
        .find(|decl| decl.id() == "runes:crafting_altar")
        .expect("runes:crafting_altar content declaration");
    let file_json = altar_decl.file_json();
    assert_eq!(file_json["shapes"], json!({ "default": expected_boxes }));
    assert!(
        file_json
            .get("properties")
            .and_then(|p| p.get("shapes"))
            .is_none(),
        "shapes must be top-level, never inside properties"
    );
}
