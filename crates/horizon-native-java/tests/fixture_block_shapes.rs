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
            "net/minecraft/world/level/block/state/properties/IntegerProperty.java",
            "package net.minecraft.world.level.block.state.properties; public final class IntegerProperty extends Property<Integer> { }",
        ),
        (
            "net/minecraft/world/level/block/state/properties/BlockStateProperties.java",
            r#"package net.minecraft.world.level.block.state.properties;
import net.minecraft.core.Direction;
public final class BlockStateProperties {
    public static final EnumProperty<Direction> HORIZONTAL_FACING = null;
    public static final BooleanProperty LIT = null;
    public static final BooleanProperty WATERLOGGED = null;
    public static final IntegerProperty AGE_3 = null;
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
import net.minecraft.core.BlockPos;
import net.minecraft.world.level.BlockGetter;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.state.properties.Property;
import net.minecraft.world.phys.shapes.CollisionContext;
import net.minecraft.world.phys.shapes.VoxelShape;
public class BlockState {
    public <T extends Comparable<T>> T getValue(Property<T> property) { return null; }
    public <T extends Comparable<T>> boolean hasProperty(Property<T> property) { return false; }
    public <T extends Comparable<T>> Optional<T> getOptionalValue(Property<T> property) { return Optional.empty(); }
    public <T extends Comparable<T>> T getValueOrElse(Property<T> property, T fallback) { return fallback; }
    public <T extends Comparable<T>> BlockState setValue(Property<T> property, T value) { return this; }
    public <T extends Comparable<T>> BlockState trySetValue(Property<T> property, T value) { return this; }
    public VoxelShape getShape(BlockGetter level, BlockPos pos) { return null; }
    public VoxelShape getShape(BlockGetter level, BlockPos pos, CollisionContext context) { return null; }
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
        public static Properties ofFullCopy(BlockBehaviour other) { return new Properties(); }
        public static Properties ofLegacyCopy(BlockBehaviour other) { return new Properties(); }
        public Properties noCollision() { return this; }
        public Properties noOcclusion() { return this; }
    }
    protected VoxelShape getShape(BlockState state, BlockGetter level, BlockPos pos, CollisionContext context) {
        return Shapes.block();
    }
    protected VoxelShape getCollisionShape(BlockState state, BlockGetter level, BlockPos pos, CollisionContext context) {
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
    assert_eq!(
        host.unknown_host_call_count(),
        unknown_before,
        "unexpected counted calls: {:?}",
        host.unknown_host_calls()
    );
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
    assert_eq!(
        host.unknown_host_call_count(),
        unknown_before,
        "unexpected counted calls: {:?}",
        host.unknown_host_calls()
    );
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
        assert_eq!(
            host.unknown_host_call_count(),
            unknown_before,
            "unexpected counted calls: {:?}",
            host.unknown_host_calls()
        );
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

        // Finding 1: identity & canonicalization against MC 26.3 `CaptureAllOracle`
        VoxelShape half1 = Block.box(0, 0, 0, 8, 16, 16);
        VoxelShape half2 = Block.box(8, 0, 0, 16, 16, 16);
        VoxelShape unoptFull = Shapes.joinUnoptimized(half1, half2, BooleanOp.OR);
        VoxelShape optFull = Shapes.or(half1, half2);
        VoxelShape unoptEmpty = Shapes.joinUnoptimized(half1, half2, BooleanOp.AND);
        VoxelShape optEmpty = Shapes.join(half1, half2, BooleanOp.AND);
        out.append("unoptFullIsBlock=").append(unoptFull == Shapes.block()).append(';');
        out.append("unoptFullClass=").append(unoptFull.getClass().getName()).append(';');
        out.append("unoptFullStr=").append(unoptFull.toString()).append(';');
        out.append("optFullIsBlock=").append(optFull == Shapes.block()).append(';');
        out.append("unoptEmptyIsEmptyRef=").append(unoptEmpty == Shapes.empty()).append(';');
        out.append("unoptEmptyIsEmpty=").append(unoptEmpty.isEmpty()).append(';');
        out.append("unoptEmptyClass=").append(unoptEmpty.getClass().getName()).append(';');
        out.append("unoptEmptyStr=").append(unoptEmpty.toString()).append(';');
        out.append("optEmptyIsEmptyRef=").append(optEmpty == Shapes.empty()).append(';');
        out.append("unoptSameOr=").append(Shapes.joinUnoptimized(half1, half1, BooleanOp.OR) == half1).append(';');
        out.append("unoptSameOnlyFirst=").append(Shapes.joinUnoptimized(half1, half1, BooleanOp.ONLY_FIRST) == Shapes.empty()).append(';');
        out.append("unoptEmptyOr=").append(Shapes.joinUnoptimized(Shapes.empty(), half1, BooleanOp.OR) == half1).append(';');
        out.append("unoptOrEmpty=").append(Shapes.joinUnoptimized(half1, Shapes.empty(), BooleanOp.OR) == half1).append(';');
        out.append("moveZeroIsBlock=").append(Shapes.block().move(0, 0, 0) == Shapes.block()).append(';');
        out.append("moveZeroClass=").append(Shapes.block().move(0, 0, 0).getClass().getName()).append(';');
        out.append("emptyMoveIsEmpty=").append(Shapes.empty().move(1, 2, 3) == Shapes.empty()).append(';');
        out.append("blockOptIsBlock=").append(Shapes.block().optimize() == Shapes.block()).append(';');
        out.append("emptyOptIsEmpty=").append(Shapes.empty().optimize() == Shapes.empty()).append(';');
        out.append("half1OptSame=").append(half1.optimize() == half1).append(';');
        try {
            Shapes.joinUnoptimized(half1, half2, BooleanOp.TRUE);
            out.append("joinTrue=ok;");
        } catch (IllegalArgumentException e) {
            out.append("joinTrue=").append(e.getMessage()).append(';');
        }

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
    assert_eq!(
        host.unknown_host_call_count(),
        unknown_before,
        "unexpected counted calls: {:?}",
        host.unknown_host_calls()
    );
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
             unoptFullIsBlock=false;\
             unoptFullClass=net.minecraft.world.phys.shapes.CubeVoxelShape;\
             unoptFullStr=VoxelShape[AABB[0.0, 0.0, 0.0] -> [1.0, 1.0, 1.0]];\
             optFullIsBlock=true;\
             unoptEmptyIsEmptyRef=false;\
             unoptEmptyIsEmpty=true;\
             unoptEmptyClass=net.minecraft.world.phys.shapes.CubeVoxelShape;\
             unoptEmptyStr=EMPTY;\
             optEmptyIsEmptyRef=true;\
             unoptSameOr=true;\
             unoptSameOnlyFirst=true;\
             unoptEmptyOr=true;\
             unoptOrEmpty=true;\
             moveZeroIsBlock=false;\
             moveZeroClass=net.minecraft.world.phys.shapes.ArrayVoxelShape;\
             emptyMoveIsEmpty=true;\
             blockOptIsBlock=true;\
             emptyOptIsEmpty=true;\
             half1OptSame=false;\
             joinTrue=null;\
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

/// Finding 2 (collision shapes, `Properties.noCollision()`, `Properties.ofFullCopy()`, separate
/// `getCollisionShape` overrides) and Finding 3 (any `BlockPos` / `BlockGetter` /
/// `CollisionContext` read during extraction refuses the block).
#[test]
fn collision_shapes_and_position_dependencies_follow_mc263_and_sunrise() {
    let mut sources = block_shape_stubs();
    sources.extend([
        (
            "demo/NoCollisionPlainBlock.java",
            r#"package demo;
import net.minecraft.world.level.block.Block;
public final class NoCollisionPlainBlock extends Block {
    public NoCollisionPlainBlock(Properties properties) { super(properties); }
}"#,
        ),
        (
            "demo/NoCollisionShapedBlock.java",
            r#"package demo;
import net.minecraft.core.BlockPos;
import net.minecraft.world.level.BlockGetter;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.phys.shapes.CollisionContext;
import net.minecraft.world.phys.shapes.VoxelShape;
public final class NoCollisionShapedBlock extends Block {
    private static final VoxelShape OUTLINE = Block.box(2.0, 0.0, 2.0, 14.0, 12.0, 14.0);
    public NoCollisionShapedBlock(Properties properties) { super(properties); }
    @Override
    protected VoxelShape getShape(BlockState state, BlockGetter level, BlockPos pos, CollisionContext context) {
        return OUTLINE;
    }
}"#,
        ),
        (
            "demo/SeparateCollisionBlock.java",
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
public final class SeparateCollisionBlock extends Block {
    private static final VoxelShape OUTLINE = Block.box(0.0, 0.0, 0.0, 16.0, 16.0, 16.0);
    private static final VoxelShape LOW_COLLISION = Block.box(0.0, 0.0, 0.0, 16.0, 14.0, 16.0);
    public SeparateCollisionBlock(Properties properties) { super(properties); }
    @Override
    protected void createBlockStateDefinition(StateDefinition.Builder<Block, BlockState> builder) {
        builder.add(BlockStateProperties.LIT);
    }
    @Override
    protected VoxelShape getShape(BlockState state, BlockGetter level, BlockPos pos, CollisionContext context) {
        return OUTLINE;
    }
    @Override
    protected VoxelShape getCollisionShape(BlockState state, BlockGetter level, BlockPos pos, CollisionContext context) {
        return state.getValue(BlockStateProperties.LIT) ? LOW_COLLISION : Shapes.empty();
    }
}"#,
        ),
        (
            "demo/DelegatingCollisionBlock.java",
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
public final class DelegatingCollisionBlock extends Block {
    private static final VoxelShape OUTLINE = Block.box(2.0, 0.0, 2.0, 14.0, 8.0, 14.0);
    public DelegatingCollisionBlock(Properties properties) { super(properties); }
    @Override
    protected void createBlockStateDefinition(StateDefinition.Builder<Block, BlockState> builder) {
        builder.add(BlockStateProperties.LIT);
    }
    @Override
    protected VoxelShape getShape(BlockState state, BlockGetter level, BlockPos pos, CollisionContext context) {
        return OUTLINE;
    }
    @Override
    protected VoxelShape getCollisionShape(BlockState state, BlockGetter level, BlockPos pos, CollisionContext context) {
        return state.getValue(BlockStateProperties.LIT) ? state.getShape(level, pos) : Shapes.empty();
    }
}"#,
        ),
        (
            "demo/PosGetterBlock.java",
            r#"package demo;
import net.minecraft.core.BlockPos;
import net.minecraft.world.level.BlockGetter;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.phys.shapes.CollisionContext;
import net.minecraft.world.phys.shapes.Shapes;
import net.minecraft.world.phys.shapes.VoxelShape;
public final class PosGetterBlock extends Block {
    public PosGetterBlock(Properties properties) { super(properties); }
    @Override
    protected VoxelShape getShape(BlockState state, BlockGetter level, BlockPos pos, CollisionContext context) {
        return pos.getY() > 64 ? Block.box(0, 0, 0, 16, 8, 16) : Shapes.block();
    }
}"#,
        ),
        (
            "demo/PosEqualsBlock.java",
            r#"package demo;
import net.minecraft.core.BlockPos;
import net.minecraft.world.level.BlockGetter;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.phys.shapes.CollisionContext;
import net.minecraft.world.phys.shapes.Shapes;
import net.minecraft.world.phys.shapes.VoxelShape;
public final class PosEqualsBlock extends Block {
    public PosEqualsBlock(Properties properties) { super(properties); }
    @Override
    protected VoxelShape getShape(BlockState state, BlockGetter level, BlockPos pos, CollisionContext context) {
        return pos.equals(BlockPos.ZERO) ? Block.box(0, 0, 0, 16, 8, 16) : Shapes.block();
    }
}"#,
        ),
        (
            "demo/PosRefEqBlock.java",
            r#"package demo;
import net.minecraft.core.BlockPos;
import net.minecraft.world.level.BlockGetter;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.phys.shapes.CollisionContext;
import net.minecraft.world.phys.shapes.Shapes;
import net.minecraft.world.phys.shapes.VoxelShape;
public final class PosRefEqBlock extends Block {
    public PosRefEqBlock(Properties properties) { super(properties); }
    @Override
    protected VoxelShape getShape(BlockState state, BlockGetter level, BlockPos pos, CollisionContext context) {
        return pos == BlockPos.ZERO ? Block.box(0, 0, 0, 16, 8, 16) : Shapes.block();
    }
}"#,
        ),
        (
            "demo/CollisionPosBlock.java",
            r#"package demo;
import net.minecraft.core.BlockPos;
import net.minecraft.world.level.BlockGetter;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.phys.shapes.CollisionContext;
import net.minecraft.world.phys.shapes.Shapes;
import net.minecraft.world.phys.shapes.VoxelShape;
public final class CollisionPosBlock extends Block {
    private static final VoxelShape OUTLINE = Block.box(2.0, 0.0, 2.0, 14.0, 12.0, 14.0);
    public CollisionPosBlock(Properties properties) { super(properties); }
    @Override
    protected VoxelShape getShape(BlockState state, BlockGetter level, BlockPos pos, CollisionContext context) {
        return OUTLINE;
    }
    @Override
    protected VoxelShape getCollisionShape(BlockState state, BlockGetter level, BlockPos pos, CollisionContext context) {
        return pos.getX() == 0 ? Shapes.empty() : OUTLINE;
    }
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
        NoCollisionPlainBlock plain = new NoCollisionPlainBlock(BlockBehaviour.Properties.of().noCollision());
        Registry.register(BuiltInRegistries.BLOCK, id("no_collision_plain"), plain);
        Registry.register(BuiltInRegistries.BLOCK, id("no_collision_shaped"),
            new NoCollisionShapedBlock(BlockBehaviour.Properties.of().noCollision()));
        Registry.register(BuiltInRegistries.BLOCK, id("separate_collision"),
            new SeparateCollisionBlock(BlockBehaviour.Properties.of()));
        Registry.register(BuiltInRegistries.BLOCK, id("delegating_collision"),
            new DelegatingCollisionBlock(BlockBehaviour.Properties.of()));
        Registry.register(BuiltInRegistries.BLOCK, id("copied_props_shaped"),
            new NoCollisionShapedBlock(BlockBehaviour.Properties.ofFullCopy(plain)));
        Registry.register(BuiltInRegistries.BLOCK, id("pos_getter"),
            new PosGetterBlock(BlockBehaviour.Properties.of()));
        Registry.register(BuiltInRegistries.BLOCK, id("pos_equals"),
            new PosEqualsBlock(BlockBehaviour.Properties.of()));
        Registry.register(BuiltInRegistries.BLOCK, id("pos_ref_eq"),
            new PosRefEqBlock(BlockBehaviour.Properties.of()));
        Registry.register(BuiltInRegistries.BLOCK, id("collision_pos"),
            new CollisionPosBlock(BlockBehaviour.Properties.of()));
    }
}"#,
        ),
    ]);

    let fixture = javac_fixture::Fixture::build("block-shapes-collision-pos", &sources);
    let mut host = fixture.jar.seed_host(StdHost::new());
    let entry = fixture.jar.discover_entrypoint("main").unwrap();
    fixture
        .jar
        .execute(&entry.class, entry.method.as_deref(), "main", &mut host)
        .unwrap();

    // NoCollisionPlainBlock: full-cube outline (unoverridden), empty collision, 0 counted calls.
    let no_col_plain_blk = registration_value(&host, "fixture:no_collision_plain");
    let before = host.unknown_host_call_count();
    let no_col_plain = fixture
        .jar
        .block_shapes(&mut host, &no_col_plain_blk)
        .unwrap();
    assert_eq!(host.unknown_host_call_count(), before);
    assert!(!no_col_plain.overridden);
    assert_eq!(
        no_col_plain.collision_states.as_ref().unwrap()[0].boxes,
        Vec::<[f64; 6]>::new()
    );

    // DelegatingCollisionBlock: calls `state.getShape(level, pos)` inside `getCollisionShape`, 0 counted.
    let deleg_blk = registration_value(&host, "fixture:delegating_collision");
    let before = host.unknown_host_call_count();
    let deleg = fixture.jar.block_shapes(&mut host, &deleg_blk).unwrap();
    assert_eq!(host.unknown_host_call_count(), before);
    assert!(deleg.overridden);
    assert!(deleg.collision_overridden);

    // Position reads (`getY`, `equals`, `==`) all count and refuse the block!
    for id in [
        "fixture:pos_getter",
        "fixture:pos_equals",
        "fixture:pos_ref_eq",
    ] {
        let blk = registration_value(&host, id);
        let before = host.unknown_host_call_count();
        let res = fixture.jar.block_shapes(&mut host, &blk).unwrap();
        assert!(
            host.unknown_host_call_count() > before,
            "{id}: BlockPos observation must be counted"
        );
        assert!(res.states.is_none(), "{id} must have no outline shape");
        assert_eq!(res.to_content_export(), (None, None));
    }

    // Copied properties (`ofFullCopy` without `.noCollision()`): unknown `hasCollision` -> counted,
    // `collision_states == None`, refuses exporting `shapes` too!
    let copied_blk = registration_value(&host, "fixture:copied_props_shaped");
    let before = host.unknown_host_call_count();
    let copied = fixture.jar.block_shapes(&mut host, &copied_blk).unwrap();
    assert!(host.unknown_host_call_count() > before);
    assert!(copied.states.is_some());
    assert!(copied.collision_states.is_none());
    assert_eq!(copied.to_content_export(), (None, None));

    // CollisionPosBlock: outline resolved cleanly, collision read `pos.getX()` -> counted, refuses BOTH!
    let col_pos_blk = registration_value(&host, "fixture:collision_pos");
    let before = host.unknown_host_call_count();
    let col_pos = fixture.jar.block_shapes(&mut host, &col_pos_blk).unwrap();
    assert!(host.unknown_host_call_count() > before);
    assert!(col_pos.states.is_some());
    assert!(col_pos.collision_states.is_none());
    assert_eq!(col_pos.to_content_export(), (None, None));

    // Full content export check
    fixture
        .jar
        .populate_block_shapes(&mut host, &ExecutionBudget::UNBOUNDED);
    let exported: BTreeMap<String, serde_json::Value> = content::declarations(host.registrations())
        .into_iter()
        .map(|decl| (decl.id().to_owned(), decl.file_json()))
        .collect();

    assert!(
        exported["fixture:no_collision_plain"]
            .get("shapes")
            .is_none()
    );
    assert_eq!(
        exported["fixture:no_collision_plain"]["collision_shapes"],
        json!({ "default": [] })
    );
    assert_eq!(
        exported["fixture:no_collision_plain"]["properties"]["occludes"],
        json!(false)
    );

    assert_eq!(
        exported["fixture:no_collision_shaped"]["shapes"],
        json!({ "default": [[0.125, 0.0, 0.125, 0.875, 0.75, 0.875]] })
    );
    assert_eq!(
        exported["fixture:no_collision_shaped"]["collision_shapes"],
        json!({ "default": [] })
    );

    assert_eq!(
        exported["fixture:separate_collision"]["shapes"],
        json!({ "default": [[0.0, 0.0, 0.0, 1.0, 1.0, 1.0]] })
    );
    assert_eq!(
        exported["fixture:separate_collision"]["collision_shapes"],
        json!({
            "properties": ["lit"],
            "values": {
                "false": [],
                "true": [[0.0, 0.0, 0.0, 1.0, 0.875, 1.0]],
            }
        })
    );

    assert_eq!(
        exported["fixture:delegating_collision"]["shapes"],
        json!({ "default": [[0.125, 0.0, 0.125, 0.875, 0.5, 0.875]] })
    );
    assert_eq!(
        exported["fixture:delegating_collision"]["collision_shapes"],
        json!({
            "properties": ["lit"],
            "values": {
                "false": [],
                "true": [[0.125, 0.0, 0.125, 0.875, 0.5, 0.875]],
            }
        })
    );

    for refused in [
        "fixture:copied_props_shaped",
        "fixture:pos_getter",
        "fixture:pos_equals",
        "fixture:pos_ref_eq",
        "fixture:collision_pos",
    ] {
        assert!(
            exported[refused].get("shapes").is_none(),
            "{refused} must not export shapes"
        );
        assert!(
            exported[refused].get("collision_shapes").is_none(),
            "{refused} must not export collision_shapes"
        );
    }
}

/// Finding 4: `BlockState.setValue`, `trySetValue`, and `getValue` validate property existence
/// and allowed values against `getPossibleValues()`, throwing `IllegalArgumentException` with the
/// exact MC 26.3 `StateHolder` messages.
#[test]
fn block_state_set_value_validates_property_and_allowed_values_like_mc263() {
    let mut sources = block_shape_stubs();
    sources.extend([
        (
            "demo/StateBlock.java",
            r#"package demo;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.block.state.StateDefinition;
import net.minecraft.world.level.block.state.properties.BlockStateProperties;
public final class StateBlock extends Block {
    public StateBlock(Properties properties) { super(properties); }
    @Override
    protected void createBlockStateDefinition(StateDefinition.Builder<Block, BlockState> builder) {
        builder.add(BlockStateProperties.HORIZONTAL_FACING, BlockStateProperties.AGE_3);
    }
}"#,
        ),
        (
            "demo/Probe.java",
            r#"package demo;
import net.fabricmc.api.ModInitializer;
import net.minecraft.core.Direction;
import net.minecraft.core.Registry;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.resources.Identifier;
import net.minecraft.world.level.block.state.BlockBehaviour;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.block.state.properties.BlockStateProperties;
public final class Probe implements ModInitializer {
    private static StateBlock BLOCK;
    @Override
    public void onInitialize() {
        BLOCK = Registry.register(
            BuiltInRegistries.BLOCK,
            Identifier.fromNamespaceAndPath("fixture", "state_block"),
            new StateBlock(BlockBehaviour.Properties.of())
        );
    }

    public static String check() {
        StringBuilder out = new StringBuilder();
        BlockState state = BLOCK.defaultBlockState();
        BlockState south = state.setValue(BlockStateProperties.HORIZONTAL_FACING, Direction.SOUTH);
        out.append("valid=").append(south.getValue(BlockStateProperties.HORIZONTAL_FACING) == Direction.SOUTH).append(';');
        try {
            state.setValue(BlockStateProperties.LIT, Boolean.TRUE);
            out.append("missingSet=ok;");
        } catch (IllegalArgumentException e) {
            out.append("missingSet=").append(e.getMessage()).append(';');
        }
        try {
            state.setValue(BlockStateProperties.HORIZONTAL_FACING, Direction.UP);
            out.append("badEnum=ok;");
        } catch (IllegalArgumentException e) {
            out.append("badEnum=").append(e.getMessage()).append(';');
        }
        try {
            state.setValue(BlockStateProperties.AGE_3, Integer.valueOf(99));
            out.append("badInt=ok;");
        } catch (IllegalArgumentException e) {
            out.append("badInt=").append(e.getMessage()).append(';');
        }
        try {
            state.getValue(BlockStateProperties.LIT);
            out.append("missingGet=ok;");
        } catch (IllegalArgumentException e) {
            out.append("missingGet=").append(e.getMessage()).append(';');
        }
        out.append("tryMissingSame=").append(state.trySetValue(BlockStateProperties.LIT, Boolean.TRUE) == state).append(';');
        try {
            state.trySetValue(BlockStateProperties.HORIZONTAL_FACING, Direction.UP);
            out.append("tryBad=ok;");
        } catch (IllegalArgumentException e) {
            out.append("tryBad=").append(e.getMessage()).append(';');
        }
        return out.toString();
    }
}"#,
        ),
    ]);

    let fixture = javac_fixture::Fixture::build("block-state-set-value", &sources);
    let mut host = fixture.jar.seed_host(StdHost::new());
    let entry = fixture.jar.discover_entrypoint("main").unwrap();
    fixture
        .jar
        .execute(&entry.class, entry.method.as_deref(), "main", &mut host)
        .unwrap();

    let before = host.unknown_host_call_count();
    let answer = fixture
        .jar
        .invoke_method_with_args(
            "demo/Probe",
            "check",
            "()Ljava/lang/String;",
            None,
            &[],
            &mut host,
        )
        .unwrap();
    assert_eq!(host.unknown_host_call_count(), before);
    assert_eq!(
        answer,
        Value::Ref(Some(
            "valid=true;\
             missingSet=Cannot set property BooleanProperty{name=lit, clazz=class java.lang.Boolean, values=[true, false]} as it does not exist in Block{fixture:state_block};\
             badEnum=Cannot set property EnumProperty{name=facing, clazz=class net.minecraft.core.Direction, values=[north, south, west, east]} to up on Block{fixture:state_block}, it is not an allowed value;\
             badInt=Cannot set property IntegerProperty{name=age, clazz=class java.lang.Integer, values=[0, 1, 2, 3]} to 99 on Block{fixture:state_block}, it is not an allowed value;\
             missingGet=Cannot get property BooleanProperty{name=lit, clazz=class java.lang.Boolean, values=[true, false]} as it does not exist in Block{fixture:state_block};\
             tryMissingSame=true;\
             tryBad=Cannot set property EnumProperty{name=facing, clazz=class net.minecraft.core.Direction, values=[north, south, west, east]} to up on Block{fixture:state_block}, it is not an allowed value;"
                .to_owned()
        ))
    );
}

/// Round-2 Finding 1: opaque `BlockGetter` / `BlockPos` / `CollisionContext` probes must count
/// EVERY observation (`equals`, `instanceof`, `checkcast`, `==`, `getClass`, `Objects.equals`) and
/// refuse static shape export, while forwarding probes through `super.getCollisionShape` /
/// `state.getShape` still exports cleanly with zero counted calls.
#[test]
fn opaque_probes_count_every_observation_and_refuse_export() {
    let mut sources = block_shape_stubs();
    sources.extend([
        (
            "demo/LevelEqualsNullBlock.java",
            r#"package demo;
import net.minecraft.core.BlockPos;
import net.minecraft.world.level.BlockGetter;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.phys.shapes.CollisionContext;
import net.minecraft.world.phys.shapes.Shapes;
import net.minecraft.world.phys.shapes.VoxelShape;
public final class LevelEqualsNullBlock extends Block {
    public LevelEqualsNullBlock(Properties p) { super(p); }
    @Override
    protected VoxelShape getShape(BlockState state, BlockGetter level, BlockPos pos, CollisionContext context) {
        return level.equals(null) ? Shapes.empty() : Block.box(0, 0, 0, 16, 8, 16);
    }
}"#,
        ),
        (
            "demo/LevelEqualsSelfBlock.java",
            r#"package demo;
import net.minecraft.core.BlockPos;
import net.minecraft.world.level.BlockGetter;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.phys.shapes.CollisionContext;
import net.minecraft.world.phys.shapes.Shapes;
import net.minecraft.world.phys.shapes.VoxelShape;
public final class LevelEqualsSelfBlock extends Block {
    public LevelEqualsSelfBlock(Properties p) { super(p); }
    @Override
    protected VoxelShape getShape(BlockState state, BlockGetter level, BlockPos pos, CollisionContext context) {
        return level.equals(level) ? Block.box(0, 0, 0, 16, 8, 16) : Shapes.empty();
    }
}"#,
        ),
        (
            "demo/LevelInstanceOfBlock.java",
            r#"package demo;
import net.minecraft.core.BlockPos;
import net.minecraft.world.level.BlockGetter;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.phys.shapes.CollisionContext;
import net.minecraft.world.phys.shapes.Shapes;
import net.minecraft.world.phys.shapes.VoxelShape;
public final class LevelInstanceOfBlock extends Block {
    public LevelInstanceOfBlock(Properties p) { super(p); }
    @Override
    protected VoxelShape getShape(BlockState state, BlockGetter level, BlockPos pos, CollisionContext context) {
        return (level instanceof BlockGetter) ? Block.box(0, 0, 0, 16, 8, 16) : Shapes.empty();
    }
}"#,
        ),
        (
            "demo/LevelCheckCastBlock.java",
            r#"package demo;
import net.minecraft.core.BlockPos;
import net.minecraft.world.level.BlockGetter;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.phys.shapes.CollisionContext;
import net.minecraft.world.phys.shapes.VoxelShape;
public final class LevelCheckCastBlock extends Block {
    public LevelCheckCastBlock(Properties p) { super(p); }
    private static Object erase(Object o) { return o; }
    @Override
    protected VoxelShape getShape(BlockState state, BlockGetter level, BlockPos pos, CollisionContext context) {
        BlockGetter g = (BlockGetter) erase(level);
        return g == null ? Block.box(0, 0, 0, 16, 4, 16) : Block.box(0, 0, 0, 16, 8, 16);
    }
}"#,
        ),
        (
            "demo/PosInstanceOfBlock.java",
            r#"package demo;
import net.minecraft.core.BlockPos;
import net.minecraft.world.level.BlockGetter;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.phys.shapes.CollisionContext;
import net.minecraft.world.phys.shapes.Shapes;
import net.minecraft.world.phys.shapes.VoxelShape;
public final class PosInstanceOfBlock extends Block {
    public PosInstanceOfBlock(Properties p) { super(p); }
    @Override
    protected VoxelShape getShape(BlockState state, BlockGetter level, BlockPos pos, CollisionContext context) {
        return (pos instanceof BlockPos) ? Block.box(0, 0, 0, 16, 8, 16) : Shapes.empty();
    }
}"#,
        ),
        (
            "demo/PosCheckCastBlock.java",
            r#"package demo;
import net.minecraft.core.BlockPos;
import net.minecraft.world.level.BlockGetter;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.phys.shapes.CollisionContext;
import net.minecraft.world.phys.shapes.VoxelShape;
public final class PosCheckCastBlock extends Block {
    public PosCheckCastBlock(Properties p) { super(p); }
    private static Object erase(Object o) { return o; }
    @Override
    protected VoxelShape getShape(BlockState state, BlockGetter level, BlockPos pos, CollisionContext context) {
        BlockPos p = (BlockPos) erase(pos);
        return p == null ? Block.box(0, 0, 0, 16, 4, 16) : Block.box(0, 0, 0, 16, 8, 16);
    }
}"#,
        ),
        (
            "demo/ContextInstanceOfBlock.java",
            r#"package demo;
import net.minecraft.core.BlockPos;
import net.minecraft.world.level.BlockGetter;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.phys.shapes.CollisionContext;
import net.minecraft.world.phys.shapes.Shapes;
import net.minecraft.world.phys.shapes.VoxelShape;
public final class ContextInstanceOfBlock extends Block {
    public ContextInstanceOfBlock(Properties p) { super(p); }
    @Override
    protected VoxelShape getShape(BlockState state, BlockGetter level, BlockPos pos, CollisionContext context) {
        return (context instanceof CollisionContext) ? Block.box(0, 0, 0, 16, 8, 16) : Shapes.empty();
    }
}"#,
        ),
        (
            "demo/ContextCheckCastBlock.java",
            r#"package demo;
import net.minecraft.core.BlockPos;
import net.minecraft.world.level.BlockGetter;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.phys.shapes.CollisionContext;
import net.minecraft.world.phys.shapes.VoxelShape;
public final class ContextCheckCastBlock extends Block {
    public ContextCheckCastBlock(Properties p) { super(p); }
    private static Object erase(Object o) { return o; }
    @Override
    protected VoxelShape getShape(BlockState state, BlockGetter level, BlockPos pos, CollisionContext context) {
        CollisionContext c = (CollisionContext) erase(context);
        return c == null ? Block.box(0, 0, 0, 16, 4, 16) : Block.box(0, 0, 0, 16, 8, 16);
    }
}"#,
        ),
        (
            "demo/ContextGetClassBlock.java",
            r#"package demo;
import net.minecraft.core.BlockPos;
import net.minecraft.world.level.BlockGetter;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.phys.shapes.CollisionContext;
import net.minecraft.world.phys.shapes.Shapes;
import net.minecraft.world.phys.shapes.VoxelShape;
public final class ContextGetClassBlock extends Block {
    public ContextGetClassBlock(Properties p) { super(p); }
    @Override
    protected VoxelShape getShape(BlockState state, BlockGetter level, BlockPos pos, CollisionContext context) {
        return ((Object) context.getClass()) == Object.class ? Shapes.empty() : Block.box(0, 0, 0, 16, 8, 16);
    }
}"#,
        ),
        (
            "demo/ObjectsEqualsNullBlock.java",
            r#"package demo;
import java.util.Objects;
import net.minecraft.core.BlockPos;
import net.minecraft.world.level.BlockGetter;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.phys.shapes.CollisionContext;
import net.minecraft.world.phys.shapes.Shapes;
import net.minecraft.world.phys.shapes.VoxelShape;
public final class ObjectsEqualsNullBlock extends Block {
    public ObjectsEqualsNullBlock(Properties p) { super(p); }
    @Override
    protected VoxelShape getShape(BlockState state, BlockGetter level, BlockPos pos, CollisionContext context) {
        return Objects.equals(level, null) ? Shapes.empty() : Block.box(0, 0, 0, 16, 8, 16);
    }
}"#,
        ),
        (
            "demo/ObjectsEqualsSelfBlock.java",
            r#"package demo;
import java.util.Objects;
import net.minecraft.core.BlockPos;
import net.minecraft.world.level.BlockGetter;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.phys.shapes.CollisionContext;
import net.minecraft.world.phys.shapes.Shapes;
import net.minecraft.world.phys.shapes.VoxelShape;
public final class ObjectsEqualsSelfBlock extends Block {
    public ObjectsEqualsSelfBlock(Properties p) { super(p); }
    @Override
    protected VoxelShape getShape(BlockState state, BlockGetter level, BlockPos pos, CollisionContext context) {
        return Objects.equals(level, level) ? Block.box(0, 0, 0, 16, 8, 16) : Shapes.empty();
    }
}"#,
        ),
        (
            "demo/ContextRefEqBlock.java",
            r#"package demo;
import net.minecraft.core.BlockPos;
import net.minecraft.world.level.BlockGetter;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.phys.shapes.CollisionContext;
import net.minecraft.world.phys.shapes.Shapes;
import net.minecraft.world.phys.shapes.VoxelShape;
public final class ContextRefEqBlock extends Block {
    public ContextRefEqBlock(Properties p) { super(p); }
    @Override
    protected VoxelShape getShape(BlockState state, BlockGetter level, BlockPos pos, CollisionContext context) {
        return context == CollisionContext.empty() ? Block.box(0, 0, 0, 16, 8, 16) : Shapes.empty();
    }
}"#,
        ),
        (
            "demo/LevelRefEqOtherBlock.java",
            r#"package demo;
import net.minecraft.core.BlockPos;
import net.minecraft.world.level.BlockGetter;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.phys.shapes.CollisionContext;
import net.minecraft.world.phys.shapes.Shapes;
import net.minecraft.world.phys.shapes.VoxelShape;
public final class LevelRefEqOtherBlock extends Block {
    public LevelRefEqOtherBlock(Properties p) { super(p); }
    @Override
    protected VoxelShape getShape(BlockState state, BlockGetter level, BlockPos pos, CollisionContext context) {
        return level == (Object) "x" ? Shapes.empty() : Block.box(0, 0, 0, 16, 8, 16);
    }
}"#,
        ),
        (
            "demo/CleanForwardingBlock.java",
            r#"package demo;
import net.minecraft.core.BlockPos;
import net.minecraft.world.level.BlockGetter;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.phys.shapes.CollisionContext;
import net.minecraft.world.phys.shapes.VoxelShape;
public final class CleanForwardingBlock extends Block {
    private static final VoxelShape SLAB = Block.box(0, 0, 0, 16, 8, 16);
    public CleanForwardingBlock(Properties p) { super(p); }
    @Override
    protected VoxelShape getShape(BlockState state, BlockGetter level, BlockPos pos, CollisionContext context) {
        return SLAB;
    }
    @Override
    protected VoxelShape getCollisionShape(BlockState state, BlockGetter level, BlockPos pos, CollisionContext context) {
        return super.getCollisionShape(state, level, pos, context);
    }
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
        Registry.register(BuiltInRegistries.BLOCK, id("level_eq_null"), new LevelEqualsNullBlock(BlockBehaviour.Properties.of()));
        Registry.register(BuiltInRegistries.BLOCK, id("level_eq_self"), new LevelEqualsSelfBlock(BlockBehaviour.Properties.of()));
        Registry.register(BuiltInRegistries.BLOCK, id("level_instanceof"), new LevelInstanceOfBlock(BlockBehaviour.Properties.of()));
        Registry.register(BuiltInRegistries.BLOCK, id("level_checkcast"), new LevelCheckCastBlock(BlockBehaviour.Properties.of()));
        Registry.register(BuiltInRegistries.BLOCK, id("pos_instanceof"), new PosInstanceOfBlock(BlockBehaviour.Properties.of()));
        Registry.register(BuiltInRegistries.BLOCK, id("pos_checkcast"), new PosCheckCastBlock(BlockBehaviour.Properties.of()));
        Registry.register(BuiltInRegistries.BLOCK, id("context_instanceof"), new ContextInstanceOfBlock(BlockBehaviour.Properties.of()));
        Registry.register(BuiltInRegistries.BLOCK, id("context_checkcast"), new ContextCheckCastBlock(BlockBehaviour.Properties.of()));
        Registry.register(BuiltInRegistries.BLOCK, id("context_getclass"), new ContextGetClassBlock(BlockBehaviour.Properties.of()));
        Registry.register(BuiltInRegistries.BLOCK, id("objects_eq_null"), new ObjectsEqualsNullBlock(BlockBehaviour.Properties.of()));
        Registry.register(BuiltInRegistries.BLOCK, id("objects_eq_self"), new ObjectsEqualsSelfBlock(BlockBehaviour.Properties.of()));
        Registry.register(BuiltInRegistries.BLOCK, id("context_ref_eq"), new ContextRefEqBlock(BlockBehaviour.Properties.of()));
        Registry.register(BuiltInRegistries.BLOCK, id("level_ref_eq_other"), new LevelRefEqOtherBlock(BlockBehaviour.Properties.of()));
        Registry.register(BuiltInRegistries.BLOCK, id("clean_forwarding"), new CleanForwardingBlock(BlockBehaviour.Properties.of()));
    }
}"#,
        ),
    ]);

    let fixture = javac_fixture::Fixture::build("block-shapes-opaque-probes", &sources);
    let mut host = fixture.jar.seed_host(StdHost::new());
    let entry = fixture.jar.discover_entrypoint("main").unwrap();
    fixture
        .jar
        .execute(&entry.class, entry.method.as_deref(), "main", &mut host)
        .unwrap();

    let refused_ids = [
        "fixture:level_eq_null",
        "fixture:level_eq_self",
        "fixture:level_instanceof",
        "fixture:level_checkcast",
        "fixture:pos_instanceof",
        "fixture:pos_checkcast",
        "fixture:context_instanceof",
        "fixture:context_checkcast",
        "fixture:context_getclass",
        "fixture:objects_eq_null",
        "fixture:objects_eq_self",
        "fixture:context_ref_eq",
        "fixture:level_ref_eq_other",
    ];
    for id in refused_ids {
        let blk = registration_value(&host, id);
        let before = host.unknown_host_call_count();
        let res = fixture.jar.block_shapes(&mut host, &blk).unwrap();
        assert!(
            host.unknown_host_call_count() > before,
            "{id}: probe observation must increment unknown_host_call_count"
        );
        assert!(
            res.states.is_none(),
            "{id}: shape must not resolve when probe was inspected"
        );
        assert_eq!(res.to_content_export(), (None, None), "{id}");
    }

    // Forwarding probes through `super.getCollisionShape` incurs 0 counted calls and exports cleanly.
    let clean_blk = registration_value(&host, "fixture:clean_forwarding");
    let before = host.unknown_host_call_count();
    let clean = fixture.jar.block_shapes(&mut host, &clean_blk).unwrap();
    assert_eq!(host.unknown_host_call_count(), before);
    assert!(clean.states.is_some());
    assert_eq!(clean.states, clean.collision_states);

    fixture
        .jar
        .populate_block_shapes(&mut host, &ExecutionBudget::UNBOUNDED);
    let exported: BTreeMap<String, serde_json::Value> = content::declarations(host.registrations())
        .into_iter()
        .map(|decl| (decl.id().to_owned(), decl.file_json()))
        .collect();
    for id in refused_ids {
        assert!(
            exported[id].get("shapes").is_none(),
            "{id} must not export shapes"
        );
        assert!(
            exported[id].get("collision_shapes").is_none(),
            "{id} must not export collision_shapes"
        );
    }
    assert_eq!(
        exported["fixture:clean_forwarding"]["shapes"],
        json!({ "default": [[0.0, 0.0, 0.0, 1.0, 0.5, 1.0]] })
    );
    assert!(
        exported["fixture:clean_forwarding"]
            .get("collision_shapes")
            .is_none()
    );
}

/// Round-2 Findings 2, 3, and 4:
/// * Canonical `StateHolder.setValue` / `trySetValue` identity (`==` matches MC 26.3 `neighbors`
///   table: setting current value returns `this`, repeated/round-trip/commuted transitions return
///   the exact same object).
/// * Exact bridge-method validation order (`checkcast` -> `ClassCastException`, null -> helpful
///   `NullPointerException`, real out-of-range -> `IllegalArgumentException`, and null property).
/// * `BlockBehaviour.Properties.ofFullCopy(null)` / `ofLegacyCopy(null)` throw the exact JDK 25
///   helpful `NullPointerException`.
#[test]
fn block_state_canonical_identity_type_checks_and_properties_copy_null() {
    let mut sources = block_shape_stubs();
    sources.extend([
        (
            "demo/IdentityBlock.java",
            r#"package demo;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.block.state.StateDefinition;
import net.minecraft.world.level.block.state.properties.BlockStateProperties;
public final class IdentityBlock extends Block {
    public IdentityBlock(Properties properties) { super(properties); }
    @Override
    protected void createBlockStateDefinition(StateDefinition.Builder<Block, BlockState> builder) {
        builder.add(
            BlockStateProperties.HORIZONTAL_FACING,
            BlockStateProperties.LIT,
            BlockStateProperties.AGE_3
        );
    }
}"#,
        ),
        (
            "demo/Probe.java",
            r#"package demo;
import net.fabricmc.api.ModInitializer;
import net.minecraft.core.Direction;
import net.minecraft.core.Registry;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.resources.Identifier;
import net.minecraft.world.level.block.state.BlockBehaviour;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.block.state.properties.BlockStateProperties;
import net.minecraft.world.level.block.state.properties.Property;
public final class Probe implements ModInitializer {
    private static IdentityBlock BLOCK;
    @Override
    public void onInitialize() {
        BLOCK = Registry.register(
            BuiltInRegistries.BLOCK,
            Identifier.fromNamespaceAndPath("fixture", "identity_block"),
            new IdentityBlock(BlockBehaviour.Properties.of())
        );
    }

    private static String catchText(Runnable r) {
        try {
            r.run();
            return "OK";
        } catch (Throwable t) {
            return t.getClass().getName() + ":" + t.getMessage();
        }
    }

    @SuppressWarnings({"unchecked", "rawtypes"})
    public static String check() {
        StringBuilder out = new StringBuilder();
        BlockState s0 = BLOCK.defaultBlockState();
        // Finding 2: canonical state identity (`==`)
        out.append("defSame=").append(s0 == BLOCK.defaultBlockState()).append('|');
        out.append("sameFacing=").append(s0.setValue(BlockStateProperties.HORIZONTAL_FACING, s0.getValue(BlockStateProperties.HORIZONTAL_FACING)) == s0).append('|');
        out.append("sameLit=").append(s0.setValue(BlockStateProperties.LIT, s0.getValue(BlockStateProperties.LIT)) == s0).append('|');
        BlockState litA = s0.setValue(BlockStateProperties.LIT, Boolean.FALSE);
        BlockState litB = s0.setValue(BlockStateProperties.LIT, Boolean.FALSE);
        out.append("repeat=").append(litA == litB).append('|');
        out.append("objRepeat=").append(((Object) litA) == ((Object) litB)).append('|');
        out.append("roundtrip=").append(litA.setValue(BlockStateProperties.LIT, Boolean.TRUE) == s0).append('|');
        BlockState ab = s0.setValue(BlockStateProperties.HORIZONTAL_FACING, Direction.EAST).setValue(BlockStateProperties.LIT, Boolean.FALSE);
        BlockState ba = s0.setValue(BlockStateProperties.LIT, Boolean.FALSE).setValue(BlockStateProperties.HORIZONTAL_FACING, Direction.EAST);
        out.append("commute=").append(ab == ba).append('|');
        out.append("trySame=").append(s0.trySetValue(BlockStateProperties.HORIZONTAL_FACING, s0.getValue(BlockStateProperties.HORIZONTAL_FACING)) == s0).append('|');
        out.append("tryRepeat=").append(s0.trySetValue(BlockStateProperties.LIT, Boolean.FALSE) == litA).append('|');

        // Finding 3: bridge-method checkcast CCE & null NPE order
        Property lit = BlockStateProperties.LIT;
        Property facing = BlockStateProperties.HORIZONTAL_FACING;
        Property age3 = BlockStateProperties.AGE_3;
        Property waterlogged = BlockStateProperties.WATERLOGGED;
        out.append("boolEnum=").append(catchText(() -> s0.setValue(lit, (Comparable) Direction.NORTH))).append('|');
        out.append("boolInt=").append(catchText(() -> s0.setValue(lit, (Comparable) Integer.valueOf(1)))).append('|');
        out.append("boolStr=").append(catchText(() -> s0.setValue(lit, (Comparable) "true"))).append('|');
        out.append("boolNull=").append(catchText(() -> s0.setValue(lit, (Comparable) null))).append('|');
        out.append("enumBool=").append(catchText(() -> s0.setValue(facing, (Comparable) Boolean.TRUE))).append('|');
        out.append("enumInt=").append(catchText(() -> s0.setValue(facing, (Comparable) Integer.valueOf(0)))).append('|');
        out.append("enumStr=").append(catchText(() -> s0.setValue(facing, (Comparable) "north"))).append('|');
        out.append("enumNull=").append(catchText(() -> s0.setValue(facing, (Comparable) null))).append('|');
        out.append("intBool=").append(catchText(() -> s0.setValue(age3, (Comparable) Boolean.TRUE))).append('|');
        out.append("intStr=").append(catchText(() -> s0.setValue(age3, (Comparable) "1"))).append('|');
        out.append("intNull=").append(catchText(() -> s0.setValue(age3, (Comparable) null))).append('|');
        out.append("tryMissingBadType=").append(s0.trySetValue(waterlogged, (Comparable) Direction.NORTH) == s0).append('|');
        out.append("tryPresentBadType=").append(catchText(() -> s0.trySetValue(lit, (Comparable) Direction.NORTH))).append('|');
        out.append("tryPresentNull=").append(catchText(() -> s0.trySetValue(facing, (Comparable) null))).append('|');
        out.append("setNullProp=").append(catchText(() -> s0.setValue(null, (Comparable) Boolean.TRUE))).append('|');
        out.append("getNullProp=").append(catchText(() -> s0.getValue(null))).append('|');
        out.append("tryNullProp=").append(s0.trySetValue(null, (Comparable) Boolean.TRUE) == s0).append('|');
        out.append("hasNullProp=").append(s0.hasProperty(null)).append('|');

        // Finding 4: Properties.ofFullCopy(null) / ofLegacyCopy(null) throw helpful NPE
        out.append("fullCopyNull=").append(catchText(() -> BlockBehaviour.Properties.ofFullCopy(null))).append('|');
        out.append("legacyCopyNull=").append(catchText(() -> BlockBehaviour.Properties.ofLegacyCopy(null)));
        return out.toString();
    }
}"#,
        ),
    ]);

    let fixture = javac_fixture::Fixture::build("block-state-identity-and-npe", &sources);
    let mut host = fixture.jar.seed_host(StdHost::new());
    let entry = fixture.jar.discover_entrypoint("main").unwrap();
    fixture
        .jar
        .execute(&entry.class, entry.method.as_deref(), "main", &mut host)
        .unwrap();

    let before = host.unknown_host_call_count();
    let answer = fixture
        .jar
        .invoke_method_with_args(
            "demo/Probe",
            "check",
            "()Ljava/lang/String;",
            None,
            &[],
            &mut host,
        )
        .unwrap();
    assert_eq!(host.unknown_host_call_count(), before);

    // Every single expected string below was captured verbatim from OpenJDK 25.0.5+8 against
    // Minecraft 26.3 (`CaptureAllOracle.java`).
    let expected = [
        "defSame=true",
        "sameFacing=true",
        "sameLit=true",
        "repeat=true",
        "objRepeat=true",
        "roundtrip=true",
        "commute=true",
        "trySame=true",
        "tryRepeat=true",
        "boolEnum=java.lang.ClassCastException:class net.minecraft.core.Direction cannot be cast to class java.lang.Boolean (net.minecraft.core.Direction is in unnamed module of loader 'app'; java.lang.Boolean is in module java.base of loader 'bootstrap')",
        "boolInt=java.lang.ClassCastException:class java.lang.Integer cannot be cast to class java.lang.Boolean (java.lang.Integer and java.lang.Boolean are in module java.base of loader 'bootstrap')",
        "boolStr=java.lang.ClassCastException:class java.lang.String cannot be cast to class java.lang.Boolean (java.lang.String and java.lang.Boolean are in module java.base of loader 'bootstrap')",
        "boolNull=java.lang.NullPointerException:Cannot invoke \"java.lang.Boolean.booleanValue()\" because \"value\" is null",
        "enumBool=java.lang.ClassCastException:class java.lang.Boolean cannot be cast to class java.lang.Enum (java.lang.Boolean and java.lang.Enum are in module java.base of loader 'bootstrap')",
        "enumInt=java.lang.ClassCastException:class java.lang.Integer cannot be cast to class java.lang.Enum (java.lang.Integer and java.lang.Enum are in module java.base of loader 'bootstrap')",
        "enumStr=java.lang.ClassCastException:class java.lang.String cannot be cast to class java.lang.Enum (java.lang.String and java.lang.Enum are in module java.base of loader 'bootstrap')",
        "enumNull=java.lang.NullPointerException:Cannot invoke \"java.lang.Enum.ordinal()\" because \"value\" is null",
        "intBool=java.lang.ClassCastException:class java.lang.Boolean cannot be cast to class java.lang.Integer (java.lang.Boolean and java.lang.Integer are in module java.base of loader 'bootstrap')",
        "intStr=java.lang.ClassCastException:class java.lang.String cannot be cast to class java.lang.Integer (java.lang.String and java.lang.Integer are in module java.base of loader 'bootstrap')",
        "intNull=java.lang.NullPointerException:Cannot invoke \"java.lang.Integer.intValue()\" because \"value\" is null",
        "tryMissingBadType=true",
        "tryPresentBadType=java.lang.ClassCastException:class net.minecraft.core.Direction cannot be cast to class java.lang.Boolean (net.minecraft.core.Direction is in unnamed module of loader 'app'; java.lang.Boolean is in module java.base of loader 'bootstrap')",
        "tryPresentNull=java.lang.NullPointerException:Cannot invoke \"java.lang.Enum.ordinal()\" because \"value\" is null",
        "setNullProp=java.lang.IllegalArgumentException:Cannot set property null as it does not exist in Block{fixture:identity_block}",
        "getNullProp=java.lang.IllegalArgumentException:Cannot get property null as it does not exist in Block{fixture:identity_block}",
        "tryNullProp=true",
        "hasNullProp=false",
        "fullCopyNull=java.lang.NullPointerException:Cannot read field \"properties\" because \"block\" is null",
        "legacyCopyNull=java.lang.NullPointerException:Cannot read field \"properties\" because \"block\" is null",
    ]
    .join("|");
    assert_eq!(answer, Value::Ref(Some(expected)));
}

/// Round-3 Findings 1 & 2: structural allow-list for `HostKind::ShapeProbe` — every escape route
/// (`synchronized`, static field, instance field, host collection `add`, array store, lambda
/// capture, string concat, jar helper call, `ifnull`, return-the-probe, throw-the-probe, and
/// swapped-slot forwarding) must count as unknown and refuse exporting the block's shape.
#[test]
fn shape_probe_allow_list_counts_every_escape_route() {
    let mut sources = block_shape_stubs();
    sources.extend([
        (
            "demo/SyncBlock.java",
            r#"package demo;
import net.minecraft.core.BlockPos;
import net.minecraft.world.level.BlockGetter;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.phys.shapes.CollisionContext;
import net.minecraft.world.phys.shapes.VoxelShape;
public final class SyncBlock extends Block {
    private static final VoxelShape SLAB = Block.box(0, 0, 0, 16, 8, 16);
    public SyncBlock(Properties p) { super(p); }
    @Override
    protected VoxelShape getShape(BlockState state, BlockGetter level, BlockPos pos, CollisionContext context) {
        synchronized (level) {
            return SLAB;
        }
    }
}"#,
        ),
        (
            "demo/StaticFieldBlock.java",
            r#"package demo;
import net.minecraft.core.BlockPos;
import net.minecraft.world.level.BlockGetter;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.phys.shapes.CollisionContext;
import net.minecraft.world.phys.shapes.VoxelShape;
public final class StaticFieldBlock extends Block {
    private static final VoxelShape SLAB = Block.box(0, 0, 0, 16, 8, 16);
    public static Object LEAKED;
    public StaticFieldBlock(Properties p) { super(p); }
    @Override
    protected VoxelShape getShape(BlockState state, BlockGetter level, BlockPos pos, CollisionContext context) {
        LEAKED = level;
        return SLAB;
    }
}"#,
        ),
        (
            "demo/InstanceFieldBlock.java",
            r#"package demo;
import net.minecraft.core.BlockPos;
import net.minecraft.world.level.BlockGetter;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.phys.shapes.CollisionContext;
import net.minecraft.world.phys.shapes.VoxelShape;
public final class InstanceFieldBlock extends Block {
    private static final VoxelShape SLAB = Block.box(0, 0, 0, 16, 8, 16);
    private Object leaked;
    public InstanceFieldBlock(Properties p) { super(p); }
    @Override
    protected VoxelShape getShape(BlockState state, BlockGetter level, BlockPos pos, CollisionContext context) {
        this.leaked = pos;
        return SLAB;
    }
}"#,
        ),
        (
            "demo/ListAddBlock.java",
            r#"package demo;
import java.util.ArrayList;
import java.util.List;
import net.minecraft.core.BlockPos;
import net.minecraft.world.level.BlockGetter;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.phys.shapes.CollisionContext;
import net.minecraft.world.phys.shapes.VoxelShape;
public final class ListAddBlock extends Block {
    private static final VoxelShape SLAB = Block.box(0, 0, 0, 16, 8, 16);
    private static final List<Object> SEEN = new ArrayList<>();
    public ListAddBlock(Properties p) { super(p); }
    @Override
    protected VoxelShape getShape(BlockState state, BlockGetter level, BlockPos pos, CollisionContext context) {
        SEEN.add(level);
        return SLAB;
    }
}"#,
        ),
        (
            "demo/ArrayStoreBlock.java",
            r#"package demo;
import net.minecraft.core.BlockPos;
import net.minecraft.world.level.BlockGetter;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.phys.shapes.CollisionContext;
import net.minecraft.world.phys.shapes.VoxelShape;
public final class ArrayStoreBlock extends Block {
    private static final VoxelShape SLAB = Block.box(0, 0, 0, 16, 8, 16);
    private static final Object[] ARR = new Object[1];
    public ArrayStoreBlock(Properties p) { super(p); }
    @Override
    protected VoxelShape getShape(BlockState state, BlockGetter level, BlockPos pos, CollisionContext context) {
        ARR[0] = pos;
        return SLAB;
    }
}"#,
        ),
        (
            "demo/LambdaCaptureBlock.java",
            r#"package demo;
import java.util.function.Supplier;
import net.minecraft.core.BlockPos;
import net.minecraft.world.level.BlockGetter;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.phys.shapes.CollisionContext;
import net.minecraft.world.phys.shapes.VoxelShape;
public final class LambdaCaptureBlock extends Block {
    private static final VoxelShape SLAB = Block.box(0, 0, 0, 16, 8, 16);
    public static Supplier<Object> CAPTURED;
    public LambdaCaptureBlock(Properties p) { super(p); }
    @Override
    protected VoxelShape getShape(BlockState state, BlockGetter level, BlockPos pos, CollisionContext context) {
        CAPTURED = () -> context;
        return SLAB;
    }
}"#,
        ),
        (
            "demo/StringConcatBlock.java",
            r#"package demo;
import net.minecraft.core.BlockPos;
import net.minecraft.world.level.BlockGetter;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.phys.shapes.CollisionContext;
import net.minecraft.world.phys.shapes.VoxelShape;
public final class StringConcatBlock extends Block {
    private static final VoxelShape SLAB = Block.box(0, 0, 0, 16, 8, 16);
    public StringConcatBlock(Properties p) { super(p); }
    @Override
    protected VoxelShape getShape(BlockState state, BlockGetter level, BlockPos pos, CollisionContext context) {
        String s = "p=" + level;
        return s.length() > 0 ? SLAB : SLAB;
    }
}"#,
        ),
        (
            "demo/JarHelperBlock.java",
            r#"package demo;
import net.minecraft.core.BlockPos;
import net.minecraft.world.level.BlockGetter;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.phys.shapes.CollisionContext;
import net.minecraft.world.phys.shapes.VoxelShape;
public final class JarHelperBlock extends Block {
    private static final VoxelShape SLAB = Block.box(0, 0, 0, 16, 8, 16);
    public JarHelperBlock(Properties p) { super(p); }
    private static VoxelShape helper(BlockGetter level) {
        return SLAB;
    }
    @Override
    protected VoxelShape getShape(BlockState state, BlockGetter level, BlockPos pos, CollisionContext context) {
        return helper(level);
    }
}"#,
        ),
        (
            "demo/NullCheckBlock.java",
            r#"package demo;
import net.minecraft.core.BlockPos;
import net.minecraft.world.level.BlockGetter;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.phys.shapes.CollisionContext;
import net.minecraft.world.phys.shapes.Shapes;
import net.minecraft.world.phys.shapes.VoxelShape;
public final class NullCheckBlock extends Block {
    private static final VoxelShape SLAB = Block.box(0, 0, 0, 16, 8, 16);
    public NullCheckBlock(Properties p) { super(p); }
    @Override
    protected VoxelShape getShape(BlockState state, BlockGetter level, BlockPos pos, CollisionContext context) {
        return level == null ? Shapes.empty() : SLAB;
    }
}"#,
        ),
        (
            "demo/ReturnProbeBlock.java",
            r#"package demo;
import net.minecraft.core.BlockPos;
import net.minecraft.world.level.BlockGetter;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.phys.shapes.CollisionContext;
import net.minecraft.world.phys.shapes.VoxelShape;
public final class ReturnProbeBlock extends Block {
    public ReturnProbeBlock(Properties p) { super(p); }
    @Override
    protected VoxelShape getShape(BlockState state, BlockGetter level, BlockPos pos, CollisionContext context) {
        Object o = level;
        return (VoxelShape) o;
    }
}"#,
        ),
        (
            "demo/ThrowProbeBlock.java",
            r#"package demo;
import net.minecraft.core.BlockPos;
import net.minecraft.world.level.BlockGetter;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.phys.shapes.CollisionContext;
import net.minecraft.world.phys.shapes.VoxelShape;
public final class ThrowProbeBlock extends Block {
    private static final VoxelShape SLAB = Block.box(0, 0, 0, 16, 8, 16);
    public ThrowProbeBlock(Properties p) { super(p); }
    @Override
    protected VoxelShape getShape(BlockState state, BlockGetter level, BlockPos pos, CollisionContext context) {
        try {
            Object o = level;
            throw (RuntimeException) o;
        } catch (Throwable ignored) {
            return SLAB;
        }
    }
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
        Registry.register(BuiltInRegistries.BLOCK, id("sync"), new SyncBlock(BlockBehaviour.Properties.of()));
        Registry.register(BuiltInRegistries.BLOCK, id("static_field"), new StaticFieldBlock(BlockBehaviour.Properties.of()));
        Registry.register(BuiltInRegistries.BLOCK, id("instance_field"), new InstanceFieldBlock(BlockBehaviour.Properties.of()));
        Registry.register(BuiltInRegistries.BLOCK, id("list_add"), new ListAddBlock(BlockBehaviour.Properties.of()));
        Registry.register(BuiltInRegistries.BLOCK, id("array_store"), new ArrayStoreBlock(BlockBehaviour.Properties.of()));
        Registry.register(BuiltInRegistries.BLOCK, id("lambda_capture"), new LambdaCaptureBlock(BlockBehaviour.Properties.of()));
        Registry.register(BuiltInRegistries.BLOCK, id("string_concat"), new StringConcatBlock(BlockBehaviour.Properties.of()));
        Registry.register(BuiltInRegistries.BLOCK, id("jar_helper"), new JarHelperBlock(BlockBehaviour.Properties.of()));
        Registry.register(BuiltInRegistries.BLOCK, id("null_check"), new NullCheckBlock(BlockBehaviour.Properties.of()));
        Registry.register(BuiltInRegistries.BLOCK, id("return_probe"), new ReturnProbeBlock(BlockBehaviour.Properties.of()));
        Registry.register(BuiltInRegistries.BLOCK, id("throw_probe"), new ThrowProbeBlock(BlockBehaviour.Properties.of()));
    }
}"#,
        ),
    ]);

    let fixture = javac_fixture::Fixture::build("block-shapes-probe-escapes", &sources);
    let mut host = fixture.jar.seed_host(StdHost::new());
    let entry = fixture.jar.discover_entrypoint("main").unwrap();
    fixture
        .jar
        .execute(&entry.class, entry.method.as_deref(), "main", &mut host)
        .unwrap();

    let escape_cases = [
        ("fixture:sync", "java/lang/Object#<monitorenter>()V"),
        ("fixture:static_field", "java/lang/Object#<putstatic>()V"),
        ("fixture:instance_field", "java/lang/Object#<putfield>()V"),
        (
            "fixture:list_add",
            "java/util/List#add(Ljava/lang/Object;)Z",
        ),
        ("fixture:array_store", "java/lang/Object#<aastore>()V"),
        (
            "fixture:lambda_capture",
            "java/lang/Object#<invokedynamic>()V",
        ),
        (
            "fixture:string_concat",
            "java/lang/String#valueOf(Ljava/lang/Object;)Ljava/lang/String;",
        ),
        (
            "fixture:jar_helper",
            "demo/JarHelperBlock#helper(Lnet/minecraft/world/level/BlockGetter;)Lnet/minecraft/world/phys/shapes/VoxelShape;",
        ),
        ("fixture:null_check", "java/lang/Object#<ifnonnull>()V"),
        ("fixture:return_probe", "java/lang/Object#<areturn>()V"),
        ("fixture:throw_probe", "java/lang/Object#<athrow>()V"),
    ];
    for (id, expected_call) in escape_cases {
        let blk = registration_value(&host, id);
        let before = host.unknown_host_call_count();
        let before_count = host
            .unknown_host_calls()
            .into_iter()
            .find_map(|(name, count)| (name == expected_call).then_some(count))
            .unwrap_or(0);
        let res = fixture.jar.block_shapes(&mut host, &blk).unwrap();
        let after_count = host
            .unknown_host_calls()
            .into_iter()
            .find_map(|(name, count)| (name == expected_call).then_some(count))
            .unwrap_or(0);
        assert!(
            host.unknown_host_call_count() > before,
            "{id}: probe escape must increment unknown_host_call_count"
        );
        assert!(
            after_count > before_count,
            "{id}: expected choke-point unknown call {expected_call} to increment (got calls: {:?})",
            host.unknown_host_calls()
        );
        assert!(res.states.is_none(), "{id}: shape must not resolve");
        assert_eq!(res.to_content_export(), (None, None), "{id}");
    }

    fixture
        .jar
        .populate_block_shapes(&mut host, &ExecutionBudget::UNBOUNDED);
    let exported: BTreeMap<String, serde_json::Value> = content::declarations(host.registrations())
        .into_iter()
        .map(|decl| (decl.id().to_owned(), decl.file_json()))
        .collect();
    for (id, _) in escape_cases {
        assert!(
            exported[id].get("shapes").is_none(),
            "{id} must not export shapes"
        );
        assert!(
            exported[id].get("collision_shapes").is_none(),
            "{id} must not export collision_shapes"
        );
    }
}

/// Round-3 Finding 3: canonical `BlockState` markers stay registered in `StdHost.block_states`
/// across shape extractions, so if `getShape` stores `state` in a static field, a later call
/// reading `getValue` / `getBlock` / `==` on that stored state gets the exact Java answer with
/// zero counted calls.
#[test]
fn canonical_block_states_survive_shape_extraction_when_stored_in_static() {
    let mut sources = block_shape_stubs();
    sources.extend([
        (
            "demo/CachingStateBlock.java",
            r#"package demo;
import net.minecraft.core.BlockPos;
import net.minecraft.core.Direction;
import net.minecraft.world.level.BlockGetter;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.block.state.StateDefinition;
import net.minecraft.world.level.block.state.properties.BlockStateProperties;
import net.minecraft.world.phys.shapes.CollisionContext;
import net.minecraft.world.phys.shapes.VoxelShape;
public final class CachingStateBlock extends Block {
    private static final VoxelShape SLAB = Block.box(0, 0, 0, 16, 8, 16);
    public static BlockState LAST_EAST_STATE;
    public CachingStateBlock(Properties p) { super(p); }
    @Override
    protected void createBlockStateDefinition(StateDefinition.Builder<Block, BlockState> builder) {
        builder.add(BlockStateProperties.HORIZONTAL_FACING);
    }
    @Override
    protected VoxelShape getShape(BlockState state, BlockGetter level, BlockPos pos, CollisionContext context) {
        if (state.getValue(BlockStateProperties.HORIZONTAL_FACING) == Direction.EAST) {
            LAST_EAST_STATE = state;
        }
        return SLAB;
    }
}"#,
        ),
        (
            "demo/Probe.java",
            r#"package demo;
import net.fabricmc.api.ModInitializer;
import net.minecraft.core.Direction;
import net.minecraft.core.Registry;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.resources.Identifier;
import net.minecraft.world.level.block.state.BlockBehaviour;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.block.state.properties.BlockStateProperties;
public final class Probe implements ModInitializer {
    private static CachingStateBlock BLOCK;
    @Override
    public void onInitialize() {
        BLOCK = Registry.register(
            BuiltInRegistries.BLOCK,
            Identifier.fromNamespaceAndPath("fixture", "caching_state"),
            new CachingStateBlock(BlockBehaviour.Properties.of())
        );
    }

    public static String readStoredState() {
        BlockState stored = CachingStateBlock.LAST_EAST_STATE;
        Direction facing = stored.getValue(BlockStateProperties.HORIZONTAL_FACING);
        boolean sameBlock = stored.getBlock() == BLOCK;
        boolean sameCanonical = stored == BLOCK.defaultBlockState().setValue(BlockStateProperties.HORIZONTAL_FACING, Direction.EAST);
        return facing.name() + "|" + sameBlock + "|" + sameCanonical;
    }
}"#,
        ),
    ]);

    let fixture = javac_fixture::Fixture::build("block-shapes-stored-state", &sources);
    let mut host = fixture.jar.seed_host(StdHost::new());
    let entry = fixture.jar.discover_entrypoint("main").unwrap();
    fixture
        .jar
        .execute(&entry.class, entry.method.as_deref(), "main", &mut host)
        .unwrap();

    let before = host.unknown_host_call_count();
    fixture
        .jar
        .populate_block_shapes(&mut host, &ExecutionBudget::UNBOUNDED);
    assert_eq!(host.unknown_host_call_count(), before);

    let answer = fixture
        .jar
        .invoke_method_with_args(
            "demo/Probe",
            "readStoredState",
            "()Ljava/lang/String;",
            None,
            &[],
            &mut host,
        )
        .unwrap();
    assert_eq!(host.unknown_host_call_count(), before);
    assert_eq!(answer, Value::Ref(Some("EAST|true|true".to_owned())));
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
        "evaluating RuneCraftingBlock.getShape and getCollisionShape across all states must incur 0 counted calls"
    );
    assert!(shapes.overridden);
    assert!(!shapes.collision_overridden);
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
    let collision_states = shapes
        .collision_states
        .as_ref()
        .expect("altar collision states resolved");
    assert_eq!(states, collision_states);
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
        file_json.get("collision_shapes").is_none(),
        "collision_shapes equals outline shapes, so Sunrise fallback applies"
    );
    assert!(
        file_json
            .get("properties")
            .and_then(|p| p.get("shapes"))
            .is_none(),
        "shapes must be top-level, never inside properties"
    );
}
