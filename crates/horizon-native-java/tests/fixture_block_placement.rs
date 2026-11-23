//! `Block.getStateForPlacement` runs a jar override through a real virtual
//! dispatch with a host-owned `BlockPlaceContext` snapshot, verified against
//! the real JDK 25 / Minecraft 26.3 oracle (`tests/oracles/block_placement/`).

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::{BlockPlaceContextData, HostKind, HostRef, Placer, StdHost, Value};

const ORACLE: &str = include_str!("fixtures/block_placement/placement_java25.txt");

fn direction(name: &str) -> Value {
    Value::Ref(Some(format!("enum/net/minecraft/core/Direction/{name}")))
}

fn player_context(
    yaw: f32,
    pitch: f32,
    face: &str,
    replace_clicked: bool,
    can_place: bool,
    secondary_use: bool,
) -> BlockPlaceContextData {
    BlockPlaceContextData {
        clicked_pos: [4, 5, 6],
        clicked_face: direction(face),
        click_location: [4.5, 5.25, 6.75],
        inside: false,
        replacing_clicked_on_block: replace_clicked,
        can_place,
        placer: Placer::Player {
            player: Value::Host(HostRef::new(HostKind::GameEntity, "game/player/test")),
            yaw,
            pitch,
            secondary_use,
        },
        item_in_hand: StdHost::empty_item_stack(),
        level: Value::Host(HostRef::new(HostKind::GameLevel, "game/level/overworld")),
    }
}

fn null_player_context(face: &str, replace_clicked: bool) -> BlockPlaceContextData {
    BlockPlaceContextData {
        clicked_pos: [4, 5, 6],
        clicked_face: direction(face),
        click_location: [4.5, 5.25, 6.75],
        inside: false,
        replacing_clicked_on_block: replace_clicked,
        can_place: true,
        placer: Placer::None { direction: None },
        item_in_hand: StdHost::empty_item_stack(),
        level: Value::Host(HostRef::new(HostKind::GameLevel, "game/level/overworld")),
    }
}

fn directional_context(
    dir: &str,
    face: &str,
    can_place: bool,
    replace_clicked: bool,
) -> BlockPlaceContextData {
    BlockPlaceContextData {
        clicked_pos: [4, 5, 6],
        clicked_face: direction(face),
        click_location: [4.5, 5.0, 6.5],
        inside: false,
        replacing_clicked_on_block: replace_clicked,
        can_place,
        placer: Placer::None {
            direction: Some(direction(dir)),
        },
        item_in_hand: StdHost::empty_item_stack(),
        level: Value::Host(HostRef::new(HostKind::GameLevel, "game/level/overworld")),
    }
}

fn unknown_placer_context(face: &str, replace_clicked: bool) -> BlockPlaceContextData {
    BlockPlaceContextData {
        clicked_pos: [4, 5, 6],
        clicked_face: direction(face),
        click_location: [4.5, 5.25, 6.75],
        inside: true,
        replacing_clicked_on_block: replace_clicked,
        can_place: true,
        placer: Placer::Unknown,
        item_in_hand: StdHost::empty_item_stack(),
        level: Value::Host(HostRef::new(HostKind::GameLevel, "game/level/overworld")),
    }
}

fn parse_f32_hex(hex: &str) -> f32 {
    f32::from_bits(u32::from_str_radix(hex, 16).expect("valid f32 hex bits"))
}

fn sources() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            "net/minecraft/core/Direction.java",
            "package net.minecraft.core; public enum Direction { DOWN, UP, NORTH, SOUTH, WEST, EAST; public Direction getOpposite() { return switch(this) { case DOWN -> UP; case UP -> DOWN; case NORTH -> SOUTH; case SOUTH -> NORTH; case WEST -> EAST; case EAST -> WEST; }; } }",
        ),
        (
            "net/minecraft/world/level/block/state/properties/Property.java",
            "package net.minecraft.world.level.block.state.properties; public class Property<T extends Comparable<T>> {}",
        ),
        (
            "net/minecraft/world/level/block/state/properties/EnumProperty.java",
            "package net.minecraft.world.level.block.state.properties; public class EnumProperty<T extends Enum<T>> extends Property<T> {}",
        ),
        (
            "net/minecraft/world/level/block/state/properties/BlockStateProperties.java",
            "package net.minecraft.world.level.block.state.properties; import net.minecraft.core.Direction; public final class BlockStateProperties { public static final EnumProperty<Direction> HORIZONTAL_FACING = null; }",
        ),
        (
            "net/minecraft/world/level/block/state/BlockState.java",
            "package net.minecraft.world.level.block.state; import net.minecraft.world.level.block.Block; import net.minecraft.world.level.block.state.properties.Property; public class BlockState { public Block getBlock() { return null; } public boolean is(Block b) { return false; } public boolean isAir() { return false; } public <T extends Comparable<T>> BlockState setValue(Property<T> p, T v) { return this; } public <T extends Comparable<T>> T getValue(Property<T> p) { return null; } }",
        ),
        (
            "net/minecraft/world/level/block/state/StateDefinition.java",
            "package net.minecraft.world.level.block.state; import net.minecraft.world.level.block.state.properties.Property; public class StateDefinition<O,S> { public static class Builder<O,S> { public Builder<O,S> add(Property<?>... p){return this;} } }",
        ),
        (
            "net/minecraft/world/entity/player/Player.java",
            "package net.minecraft.world.entity.player; public class Player {}",
        ),
        (
            "net/minecraft/world/item/ItemStack.java",
            "package net.minecraft.world.item; public class ItemStack { public boolean isEmpty(){return true;} public static final ItemStack EMPTY = new ItemStack(); }",
        ),
        (
            "net/minecraft/world/level/Level.java",
            "package net.minecraft.world.level; public class Level {}",
        ),
        (
            "net/minecraft/world/phys/Vec3.java",
            "package net.minecraft.world.phys; public class Vec3 {}",
        ),
        (
            "net/minecraft/world/item/context/BlockPlaceContext.java",
            "package net.minecraft.world.item.context; import net.minecraft.core.*; import net.minecraft.world.entity.player.Player; import net.minecraft.world.item.ItemStack; import net.minecraft.world.level.Level; import net.minecraft.world.phys.Vec3; public class BlockPlaceContext { public Direction getHorizontalDirection(){return null;} public Direction getClickedFace(){return null;} public BlockPos getClickedPos(){return null;} public boolean canPlace(){return true;} public boolean replacingClickedOnBlock(){return false;} public Direction getNearestLookingDirection(){return null;} public Direction getNearestLookingVerticalDirection(){return null;} public Direction[] getNearestLookingDirections(){return null;} public float getRotation(){return 0;} public boolean isSecondaryUseActive(){return false;} public Player getPlayer(){return null;} public ItemStack getItemInHand(){return null;} public Level getLevel(){return null;} public Vec3 getClickLocation(){return null;} public boolean isInside(){return false;} public Object getHand(){return null;} }",
        ),
        (
            "net/minecraft/core/BlockPos.java",
            "package net.minecraft.core; public class BlockPos { public int getX(){return 0;} public int getY(){return 0;} public int getZ(){return 0;} }",
        ),
        (
            "net/minecraft/world/level/block/Block.java",
            "package net.minecraft.world.level.block; import net.minecraft.world.level.block.state.*; import net.minecraft.world.level.block.state.properties.*; public class Block { public BlockState defaultBlockState(){return new BlockState();} protected void createBlockStateDefinition(StateDefinition.Builder<Block,BlockState> b){} public BlockState getStateForPlacement(net.minecraft.world.item.context.BlockPlaceContext c){return null;} }",
        ),
        (
            "net/minecraft/world/level/block/Blocks.java",
            "package net.minecraft.world.level.block; public final class Blocks { public static final Block STONE = null; public static final Block AIR = null; public static final Block DIRT = null; }",
        ),
        (
            "demo/PlacementBlock.java",
            "package demo; import net.minecraft.core.*; import net.minecraft.world.level.block.*; import net.minecraft.world.level.block.state.*; import net.minecraft.world.level.block.state.properties.*; import net.minecraft.world.item.context.*; public class PlacementBlock extends Block { public static final EnumProperty<Direction> FACING=BlockStateProperties.HORIZONTAL_FACING; @Override protected void createBlockStateDefinition(StateDefinition.Builder<Block,BlockState> b){b.add(FACING);} @Override public BlockState getStateForPlacement(BlockPlaceContext c) { c.getHand(); if (!c.canPlace()) return null; return defaultBlockState().setValue(FACING, c.getHorizontalDirection().getOpposite()); } }",
        ),
        (
            "demo/StonePlacingBlock.java",
            "package demo; import net.minecraft.world.level.block.*; import net.minecraft.world.level.block.state.*; import net.minecraft.world.item.context.*; public class StonePlacingBlock extends Block { @Override public BlockState getStateForPlacement(BlockPlaceContext c) { return Blocks.STONE.defaultBlockState(); } }",
        ),
        (
            "demo/UnregisteredPlacingBlock.java",
            "package demo; import net.minecraft.world.level.block.*; import net.minecraft.world.level.block.state.*; import net.minecraft.world.item.context.*; public class UnregisteredPlacingBlock extends Block { private static final Block UNREGISTERED = new Block(); @Override public BlockState getStateForPlacement(BlockPlaceContext c) { return UNREGISTERED.defaultBlockState(); } }",
        ),
        // #635: what a placement leaves behind. `KeepingBlock` keeps its
        // context, the `Direction[]` it answered and its hand item in
        // statics, as a mod may; `ThrowingBlock` fails mid-placement.
        // `LifetimeProbe` holds the blocks (so a collection keeps them) and
        // reads what was kept; `Probe` stays as the oracle tests use it.
        (
            "demo/Tag.java",
            "package demo; public class Tag { public int n; }",
        ),
        (
            "demo/KeepingBlock.java",
            "package demo; import java.util.ArrayList; import java.util.List; import net.minecraft.core.Direction; import net.minecraft.world.item.context.BlockPlaceContext; import net.minecraft.world.level.block.Block; import net.minecraft.world.level.block.state.BlockState; public class KeepingBlock extends Block { public static BlockPlaceContext KEPT; public static Direction[] KEPT_DIRS; public static Object KEPT_ITEM; public static final List<BlockPlaceContext> SEEN = new ArrayList<>(); @Override public BlockState getStateForPlacement(BlockPlaceContext c) { KEPT = c; SEEN.add(c); KEPT_DIRS = c.getNearestLookingDirections(); KEPT_ITEM = c.getItemInHand(); return defaultBlockState(); } }",
        ),
        (
            "demo/ThrowingBlock.java",
            "package demo; import net.minecraft.world.item.context.BlockPlaceContext; import net.minecraft.world.level.block.Block; import net.minecraft.world.level.block.state.BlockState; public class ThrowingBlock extends Block { @Override public BlockState getStateForPlacement(BlockPlaceContext c) { c.getNearestLookingDirections(); throw new IllegalStateException(\"refused mid-placement\"); } }",
        ),
        (
            "demo/LifetimeProbe.java",
            r#"package demo;
import net.minecraft.core.Direction;
import net.minecraft.world.item.context.BlockPlaceContext;
public class LifetimeProbe {
  public static final KeepingBlock KEEPING = new KeepingBlock();
  public static final PlacementBlock FACING_BLOCK = new PlacementBlock();
  public static final ThrowingBlock THROWING = new ThrowingBlock();
  public static Object block(int which) {
    return switch (which) {
      case 0 -> KEEPING;
      case 1 -> FACING_BLOCK;
      default -> THROWING;
    };
  }
  public static Object makeTag(int n) {
    Tag t = new Tag();
    t.n = n;
    return t;
  }
  public static String readTag(Object o) {
    return "tag" + ((Tag) o).n;
  }
  public static String keptDirs() {
    Direction[] d = KeepingBlock.KEPT_DIRS;
    return d[0] + "," + d[1] + "," + d[2] + "," + d[3] + "," + d[4] + "," + d[5];
  }
  public static String keptItem() {
    return readTag(KeepingBlock.KEPT_ITEM);
  }
  public static String kept() {
    return (KeepingBlock.KEPT != null) + "|" + KeepingBlock.SEEN.size();
  }
  public static String touchKept(int which) {
    BlockPlaceContext c = KeepingBlock.KEPT;
    switch (which) {
      case 0 -> c.getClickedPos();
      case 1 -> c.getClickedFace();
      case 2 -> c.getClickLocation();
      case 3 -> c.isInside();
      case 4 -> c.getItemInHand();
      case 5 -> c.getLevel();
      case 6 -> c.canPlace();
      case 7 -> c.replacingClickedOnBlock();
      case 8 -> c.getPlayer();
      case 9 -> c.getHorizontalDirection();
      case 10 -> c.isSecondaryUseActive();
      case 11 -> c.getRotation();
      case 12 -> c.getNearestLookingDirection();
      case 13 -> c.getNearestLookingVerticalDirection();
      case 14 -> c.getNearestLookingDirections();
      default -> throw new IllegalArgumentException();
    }
    return "ok";
  }
  // Past the heap's collection trigger (`heap.rs`: 1,024 new entries past
  // twice what the last collection kept), so this call's store collects.
  public static String garbage() {
    for (int i = 0; i < 3000; i++) {
      Tag t = new Tag();
      t.n = i;
    }
    return "g";
  }
}
"#,
        ),
        (
            "demo/Probe.java",
            r#"package demo;
import net.minecraft.core.BlockPos;
import net.minecraft.core.Direction;
import net.minecraft.world.item.context.BlockPlaceContext;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.level.block.state.BlockState;
public class Probe {
  private static String hex(float f) {
    return String.format("%08x", Float.floatToRawIntBits(f));
  }
  private static String fmtDirs(Direction[] d) {
    return d[0] + "," + d[1] + "," + d[2] + "," + d[3] + "," + d[4] + "," + d[5];
  }
  public static String place(demo.PlacementBlock b, BlockPlaceContext c) {
    BlockState s = b.getStateForPlacement(c);
    return s == null ? "null" : String.valueOf(s.getValue(demo.PlacementBlock.FACING));
  }
  public static String playerRow(BlockPlaceContext c) {
    Direction[] d = c.getNearestLookingDirections();
    return c.getHorizontalDirection() + "|" + hex(c.getRotation()) + "|"
        + c.getNearestLookingDirection() + "|" + c.getNearestLookingVerticalDirection()
        + "|" + fmtDirs(d);
  }
  public static String dirs(BlockPlaceContext c) {
    return fmtDirs(c.getNearestLookingDirections());
  }
  public static String safeRow(BlockPlaceContext c) {
    String n1;
    try { n1 = "OK:" + c.getNearestLookingDirection(); }
    catch (Throwable t) { n1 = "EX:" + t.getClass().getName() + ":" + t.getMessage(); }
    String n2;
    try { n2 = "OK:" + c.getNearestLookingVerticalDirection(); }
    catch (Throwable t) { n2 = "EX:" + t.getClass().getName() + ":" + t.getMessage(); }
    String n3;
    try { n3 = "OK:" + fmtDirs(c.getNearestLookingDirections()); }
    catch (Throwable t) { n3 = "EX:" + t.getClass().getName() + ":" + t.getMessage(); }
    return (c.getPlayer() == null) + "|" + c.getHorizontalDirection() + "|"
        + hex(c.getRotation()) + "|" + c.isSecondaryUseActive() + "|"
        + n1 + "|" + n2 + "|" + n3;
  }
  public static String directionalRow(BlockPlaceContext c) {
    String vert;
    try { vert = "OK:" + c.getNearestLookingVerticalDirection(); }
    catch (Throwable t) { vert = "EX:" + t.getClass().getName() + ":" + t.getMessage(); }
    return (c.getPlayer() == null) + "|" + c.getHorizontalDirection() + "|"
        + hex(c.getRotation()) + "|" + c.isSecondaryUseActive() + "|"
        + c.getNearestLookingDirection() + "|" + vert + "|"
        + fmtDirs(c.getNearestLookingDirections());
  }
  public static String nonPlayerAccessors(BlockPlaceContext c) {
    BlockPos pos = c.getClickedPos();
    return pos.getX() + "," + pos.getY() + "," + pos.getZ() + "|"
        + c.getClickedFace() + "|" + c.isInside() + "|" + c.canPlace() + "|"
        + c.replacingClickedOnBlock() + "|" + c.getItemInHand().isEmpty() + "|"
        + (c.getLevel() != null) + "|" + (c.getClickLocation() != null);
  }
  public static String callUnknown(BlockPlaceContext c, int which) {
    return switch (which) {
      case 0 -> String.valueOf(c.getPlayer());
      case 1 -> String.valueOf(c.getHorizontalDirection());
      case 2 -> String.valueOf(c.getRotation());
      case 3 -> String.valueOf(c.isSecondaryUseActive());
      case 4 -> String.valueOf(c.getNearestLookingDirection());
      case 5 -> String.valueOf(c.getNearestLookingVerticalDirection());
      case 6 -> String.valueOf(c.getNearestLookingDirections().length);
      default -> "bad";
    };
  }
  public static String vanillaStoneStateChecks() {
    BlockState s1 = Blocks.STONE.defaultBlockState();
    BlockState s2 = Blocks.STONE.defaultBlockState();
    BlockState d1 = Blocks.DIRT.defaultBlockState();
    BlockState a1 = Blocks.AIR.defaultBlockState();
    return (s1 == s2) + "|" + (s1 == d1) + "|" + (s1.getBlock() == Blocks.STONE) + "|"
        + s1.is(Blocks.STONE) + "|" + s1.is(Blocks.DIRT) + "|" + s1.isAir() + "|" + a1.isAir();
  }
}
"#,
        ),
    ]
}

#[test]
fn placement_override_gets_real_context_and_null_refuses() {
    let fixture = javac_fixture::Fixture::build("block-placement", &sources());
    let mut host = fixture.jar.seed_host(StdHost::new());
    let block = Value::Host(HostRef::new(HostKind::JarObject, "demo/PlacementBlock#0"));
    fixture
        .jar
        .invoke_method(
            "demo/PlacementBlock",
            "<init>",
            "()V",
            Some(block.clone()),
            &mut host,
        )
        .unwrap();
    let desc = "(Ldemo/PlacementBlock;Lnet/minecraft/world/item/context/BlockPlaceContext;)Ljava/lang/String;";
    for (yaw, expected) in [
        (0.0, "NORTH"),
        (90.0, "EAST"),
        (180.0, "SOUTH"),
        (-90.0, "WEST"),
    ] {
        for face in ["DOWN", "UP", "NORTH", "SOUTH", "WEST", "EAST"] {
            let context =
                host.new_block_place_context(player_context(yaw, 0.0, face, false, true, false));
            let value = fixture
                .jar
                .invoke_method_with_args(
                    "demo/Probe",
                    "place",
                    desc,
                    None,
                    &[block.clone(), context],
                    &mut host,
                )
                .unwrap();
            assert_eq!(value, Value::Ref(Some(expected.into())));
        }
    }
    let context = host.new_block_place_context(player_context(0.0, 0.0, "UP", false, false, false));
    let value = fixture
        .jar
        .invoke_method_with_args(
            "demo/Probe",
            "place",
            desc,
            None,
            &[block, context],
            &mut host,
        )
        .unwrap();
    assert_eq!(value, Value::Ref(Some("null".into())));
    assert!(
        host.unknown_host_call_count() >= 4,
        "getHand is an unmodelled context call"
    );
}

#[test]
fn placement_context_matches_real_mc263_java25_oracle() {
    let fixture = javac_fixture::Fixture::build("block-placement-oracle", &sources());
    let mut host = fixture.jar.seed_host(StdHost::new());
    let desc = "(Lnet/minecraft/world/item/context/BlockPlaceContext;)Ljava/lang/String;";
    let mut checked_player = 0usize;
    let mut checked_place_dirs = 0usize;
    let mut checked_null_player = 0usize;
    let mut checked_directional = 0usize;

    for (line_no, raw_line) in ORACLE.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let cols: Vec<&str> = line.split('|').collect();
        match cols[0] {
            "PLAYER" => {
                assert_eq!(cols.len(), 8, "malformed PLAYER line {}", line_no + 1);
                let yaw = parse_f32_hex(cols[1]);
                let pitch = parse_f32_hex(cols[2]);
                let expected = format!(
                    "{}|{}|{}|{}|{}",
                    cols[3], cols[4], cols[5], cols[6], cols[7]
                );
                // `replaceClicked = true` returns raw `Direction.orderedByNearest(player)`.
                let ctx = host
                    .new_block_place_context(player_context(yaw, pitch, "UP", true, true, false));
                let actual = fixture
                    .jar
                    .invoke_method_with_args(
                        "demo/Probe",
                        "playerRow",
                        desc,
                        None,
                        &[ctx],
                        &mut host,
                    )
                    .unwrap();
                assert_eq!(
                    actual,
                    Value::Ref(Some(expected)),
                    "PLAYER mismatch at line {} (yaw={yaw}, pitch={pitch})",
                    line_no + 1
                );
                checked_player += 1;
            }
            "PLACE_DIRS" => {
                assert_eq!(cols.len(), 6, "malformed PLACE_DIRS line {}", line_no + 1);
                let yaw = parse_f32_hex(cols[1]);
                let pitch = parse_f32_hex(cols[2]);
                let face = cols[3];
                let replace_clicked = cols[4] == "true";
                let expected = cols[5];
                let ctx = host.new_block_place_context(player_context(
                    yaw,
                    pitch,
                    face,
                    replace_clicked,
                    true,
                    false,
                ));
                let actual = fixture
                    .jar
                    .invoke_method_with_args("demo/Probe", "dirs", desc, None, &[ctx], &mut host)
                    .unwrap();
                assert_eq!(
                    actual,
                    Value::Ref(Some(expected.into())),
                    "PLACE_DIRS mismatch at line {} (yaw={yaw}, pitch={pitch}, face={face}, replace={replace_clicked})",
                    line_no + 1
                );
                checked_place_dirs += 1;
            }
            "NULL_PLAYER" => {
                assert_eq!(cols.len(), 10, "malformed NULL_PLAYER line {}", line_no + 1);
                let face = cols[1];
                let replace_clicked = cols[2] == "true";
                let expected = cols[3..].join("|");
                let ctx = host.new_block_place_context(null_player_context(face, replace_clicked));
                let actual = fixture
                    .jar
                    .invoke_method_with_args("demo/Probe", "safeRow", desc, None, &[ctx], &mut host)
                    .unwrap();
                assert_eq!(
                    actual,
                    Value::Ref(Some(expected)),
                    "NULL_PLAYER mismatch at line {} (face={face}, replace={replace_clicked})",
                    line_no + 1
                );
                checked_null_player += 1;
            }
            "DIRECTIONAL" => {
                assert_eq!(cols.len(), 10, "malformed DIRECTIONAL line {}", line_no + 1);
                let dir = cols[1];
                let face = cols[2];
                let expected = cols[3..].join("|");
                let ctx = host.new_block_place_context(directional_context(dir, face, true, false));
                let actual = fixture
                    .jar
                    .invoke_method_with_args(
                        "demo/Probe",
                        "directionalRow",
                        desc,
                        None,
                        &[ctx],
                        &mut host,
                    )
                    .unwrap();
                assert_eq!(
                    actual,
                    Value::Ref(Some(expected)),
                    "DIRECTIONAL mismatch at line {} (dir={dir}, face={face})",
                    line_no + 1
                );
                checked_directional += 1;
            }
            other => panic!(
                "unexpected oracle record kind {other:?} at line {}",
                line_no + 1
            ),
        }
    }

    assert_eq!(checked_player, 153);
    assert_eq!(checked_place_dirs, 1836);
    assert_eq!(checked_null_player, 12);
    assert_eq!(checked_directional, 36);
    assert_eq!(
        host.unknown_host_call_count(),
        0,
        "every oracle row must be answered with zero unknown calls: {:?}",
        host.unknown_host_calls()
    );

    // DirectionalPlaceContext.replacingClickedOnBlock() delegates to canPlace().
    let can_true = host.new_block_place_context(directional_context("NORTH", "UP", true, false));
    let can_false = host.new_block_place_context(directional_context("NORTH", "UP", false, true));
    let t = fixture
        .jar
        .invoke_method_with_args(
            "demo/Probe",
            "nonPlayerAccessors",
            desc,
            None,
            &[can_true],
            &mut host,
        )
        .unwrap();
    let f = fixture
        .jar
        .invoke_method_with_args(
            "demo/Probe",
            "nonPlayerAccessors",
            desc,
            None,
            &[can_false],
            &mut host,
        )
        .unwrap();
    assert_eq!(
        t,
        Value::Ref(Some("4,5,6|UP|false|true|true|true|true|true".into()))
    );
    assert_eq!(
        f,
        Value::Ref(Some("4,5,6|UP|false|false|false|true|true|true".into()))
    );
    assert_eq!(host.unknown_host_call_count(), 0);
}

#[test]
fn unknown_placer_counts_every_player_and_rotation_accessor() {
    let fixture = javac_fixture::Fixture::build("block-placement-unknown-placer", &sources());
    let mut host = fixture.jar.seed_host(StdHost::new());
    let ctx = host.new_block_place_context(unknown_placer_context("EAST", true));

    // Non-player-dependent accessors stay exact with 0 unknowns.
    let non_player = fixture
        .jar
        .invoke_method_with_args(
            "demo/Probe",
            "nonPlayerAccessors",
            "(Lnet/minecraft/world/item/context/BlockPlaceContext;)Ljava/lang/String;",
            None,
            std::slice::from_ref(&ctx),
            &mut host,
        )
        .unwrap();
    assert_eq!(
        non_player,
        Value::Ref(Some("4,5,6|EAST|true|true|true|true|true|true".into()))
    );
    assert_eq!(host.unknown_host_call_count(), 0);

    // Every player- or rotation-dependent accessor MUST be counted when Placer::Unknown.
    let expected_methods = [
        "getPlayer()Lnet/minecraft/world/entity/player/Player;",
        "getHorizontalDirection()Lnet/minecraft/core/Direction;",
        "getRotation()F",
        "isSecondaryUseActive()Z",
        "getNearestLookingDirection()Lnet/minecraft/core/Direction;",
        "getNearestLookingVerticalDirection()Lnet/minecraft/core/Direction;",
        "getNearestLookingDirections()[Lnet/minecraft/core/Direction;",
    ];
    for (idx, method_sig) in expected_methods.iter().enumerate() {
        let before = host.unknown_host_call_count();
        fixture
            .jar
            .invoke_method_with_args(
                "demo/Probe",
                "callUnknown",
                "(Lnet/minecraft/world/item/context/BlockPlaceContext;I)Ljava/lang/String;",
                None,
                &[ctx.clone(), Value::Int(idx as i32)],
                &mut host,
            )
            .unwrap();
        assert_eq!(
            host.unknown_host_call_count(),
            before + 1,
            "accessor #{idx} ({method_sig}) must count as unknown under Placer::Unknown"
        );
        assert!(
            host.unknown_host_calls()
                .iter()
                .any(|(call, _)| call.ends_with(method_sig)),
            "expected {method_sig} in {:?}",
            host.unknown_host_calls()
        );
    }
}

#[test]
fn vanilla_blocks_default_state_and_unregistered_block_decode_honestly() {
    let fixture = javac_fixture::Fixture::build("block-placement-vanilla-state", &sources());
    let mut host = fixture.jar.seed_host(StdHost::new());

    // Vanilla Blocks.STONE.defaultBlockState() identity and block checks.
    let checks = fixture
        .jar
        .invoke_method(
            "demo/Probe",
            "vanillaStoneStateChecks",
            "()Ljava/lang/String;",
            None,
            &mut host,
        )
        .unwrap();
    assert_eq!(
        checks,
        Value::Ref(Some("true|false|true|true|false|false|true".into()))
    );
    assert_eq!(host.unknown_host_call_count(), 0);

    // Mod block whose getStateForPlacement returns Blocks.STONE.defaultBlockState().
    let stone_placer = Value::Host(HostRef::new(
        HostKind::JarObject,
        "demo/StonePlacingBlock#0",
    ));
    fixture
        .jar
        .invoke_method(
            "demo/StonePlacingBlock",
            "<init>",
            "()V",
            Some(stone_placer.clone()),
            &mut host,
        )
        .unwrap();
    let stone_state = fixture
        .jar
        .block_state_for_placement(stone_placer, unknown_placer_context("UP", false), &mut host)
        .unwrap()
        .expect("stone placement returns non-null state");
    assert_eq!(
        host.block_state_content(&stone_state),
        Some(("minecraft:stone".to_owned(), Default::default()))
    );
    assert_eq!(host.unknown_host_call_count(), 0);

    // Mod block whose getStateForPlacement returns a state of an unregistered block.
    let unregistered_placer = Value::Host(HostRef::new(
        HostKind::JarObject,
        "demo/UnregisteredPlacingBlock#0",
    ));
    fixture
        .jar
        .invoke_method(
            "demo/UnregisteredPlacingBlock",
            "<init>",
            "()V",
            Some(unregistered_placer.clone()),
            &mut host,
        )
        .unwrap();
    let unreg_state = fixture
        .jar
        .block_state_for_placement(
            unregistered_placer,
            unknown_placer_context("UP", false),
            &mut host,
        )
        .unwrap()
        .expect("unregistered block still returns a Java BlockState object");
    assert_eq!(host.block_state_content(&unreg_state), None);

    // Forged Strings and world-read block states with unknown properties never decode.
    assert_eq!(
        host.block_state_content(&Value::Ref(Some(
            "game/default-block-state/minecraft:stone".into()
        ))),
        None
    );
    assert_eq!(
        host.block_state_content(&Value::Host(HostRef::new(
            HostKind::GameRegistryObject,
            "game/block-state/minecraft:stone"
        ))),
        None
    );
}

/// `demo.LifetimeProbe.method` with `args`.
fn lifetime(
    fixture: &javac_fixture::Fixture,
    host: &mut StdHost,
    method: &str,
    descriptor: &str,
    args: &[Value],
) -> Result<Value, horizon_native_java::NativeError> {
    fixture
        .jar
        .invoke_method_with_args("demo/LifetimeProbe", method, descriptor, None, args, host)
}

/// `demo.LifetimeProbe.method()`'s `String` answer.
fn lifetime_text(fixture: &javac_fixture::Fixture, host: &mut StdHost, method: &str) -> String {
    match lifetime(fixture, host, method, "()Ljava/lang/String;", &[]) {
        Ok(Value::Ref(Some(text))) => text,
        other => panic!("{method}: {other:?}"),
    }
}

/// The block `demo.LifetimeProbe.block(which)` holds (0 keeping, 1 facing,
/// 2 throwing): made by the jar and kept in a static, so a collection keeps
/// it, as the game's registry does a registered block.
fn lifetime_block(fixture: &javac_fixture::Fixture, host: &mut StdHost, which: i32) -> Value {
    lifetime(
        fixture,
        host,
        "block",
        "(I)Ljava/lang/Object;",
        &[Value::Int(which)],
    )
    .unwrap()
}

/// A new `demo.Tag` holding `n`, which the caller does not retain.
fn make_tag(fixture: &javac_fixture::Fixture, host: &mut StdHost, n: i32) -> Value {
    lifetime(
        fixture,
        host,
        "makeTag",
        "(I)Ljava/lang/Object;",
        &[Value::Int(n)],
    )
    .unwrap()
}

/// The oracle's first `PLAYER` row: yaw, pitch, and what Java 25's
/// `getNearestLookingDirections()` answered for that look.
fn first_player_row() -> (f32, f32, &'static str) {
    let cols: Vec<&str> = ORACLE
        .lines()
        .find(|line| line.starts_with("PLAYER|"))
        .expect("the oracle has PLAYER rows")
        .split('|')
        .collect();
    (parse_f32_hex(cols[1]), parse_f32_hex(cols[2]), cols[7])
}

/// #635: a placement keeps nothing on the host once it ends. Every one of
/// 1,989 placements (one per `PLAYER` and `PLACE_DIRS` oracle row) leaves
/// both the context table and the `Direction[]` table empty, though the jar
/// keeps the context, the array and the hand item in statics each time. The
/// kept array is the jar's own: read after its context is gone, it is still
/// what Java 25 answered for that look (the row's last column).
#[test]
fn placements_leave_no_context_or_direction_array_behind() {
    let fixture = javac_fixture::Fixture::build("block-placement-lifetime", &sources());
    let mut host = fixture.jar.seed_host(StdHost::new());
    let keeping = lifetime_block(&fixture, &mut host, 0);
    let mut placed = 0usize;
    for (line_no, line) in ORACLE.lines().enumerate() {
        let cols: Vec<&str> = line.trim().split('|').collect();
        let (context, expected) = match cols[0] {
            // `replaceClicked = true`: Java's raw `Direction.orderedByNearest(player)`.
            "PLAYER" => (
                player_context(
                    parse_f32_hex(cols[1]),
                    parse_f32_hex(cols[2]),
                    "UP",
                    true,
                    true,
                    false,
                ),
                cols[7],
            ),
            "PLACE_DIRS" => (
                player_context(
                    parse_f32_hex(cols[1]),
                    parse_f32_hex(cols[2]),
                    cols[3],
                    cols[4] == "true",
                    true,
                    false,
                ),
                cols[5],
            ),
            _ => continue,
        };
        fixture
            .jar
            .block_state_for_placement(keeping.clone(), context, &mut host)
            .unwrap()
            .expect("KeepingBlock places its default state");
        placed += 1;
        assert_eq!(
            host.block_place_context_count(),
            0,
            "a context outlived the placement at line {}",
            line_no + 1
        );
        assert_eq!(
            host.placement_array_count(),
            0,
            "a direction array outlived the placement at line {}",
            line_no + 1
        );
        assert_eq!(
            lifetime_text(&fixture, &mut host, "keptDirs"),
            expected,
            "the kept array at line {}",
            line_no + 1
        );
    }
    assert_eq!(placed, 153 + 1836);
    assert_eq!(
        lifetime_text(&fixture, &mut host, "kept"),
        format!("true|{placed}")
    );
    assert_eq!(
        host.unknown_host_call_count(),
        0,
        "{:?}",
        host.unknown_host_calls()
    );
}

/// #635: a context the jar keeps past its placement roots nothing, and every
/// accessor called on it afterwards is counted, never answered from the old
/// snapshot (most of them read the player or the world as they are now).
///
/// One placement's hand item is a `demo.Tag` the jar keeps itself (through
/// `getItemInHand()`), the other's a `Tag` only its context held. After the
/// next collection the first still reads (`tag42`), through the jar's own
/// static, as does the `Direction[]` the jar kept (Java 25's answer for that
/// look); the second was collected with nothing left to root it, so reading
/// it fails by name, as reading any collected object does.
#[test]
fn a_kept_context_roots_nothing_and_counts_every_accessor() {
    let fixture = javac_fixture::Fixture::build("block-placement-kept", &sources());
    let mut host = fixture.jar.seed_host(StdHost::new());
    let keeping = lifetime_block(&fixture, &mut host, 0);
    let facing = lifetime_block(&fixture, &mut host, 1);
    let kept_item = make_tag(&fixture, &mut host, 42);
    let dropped_item = make_tag(&fixture, &mut host, 41);

    let (yaw, pitch, expected_dirs) = first_player_row();
    let mut context = player_context(yaw, pitch, "UP", true, true, false);
    context.item_in_hand = kept_item;
    fixture
        .jar
        .block_state_for_placement(keeping, context, &mut host)
        .unwrap()
        .expect("KeepingBlock places its default state");
    let mut context = player_context(90.0, 0.0, "UP", false, true, false);
    context.item_in_hand = dropped_item.clone();
    fixture
        .jar
        .block_state_for_placement(facing, context, &mut host)
        .unwrap()
        .expect("PlacementBlock places a facing state");
    assert_eq!(host.block_place_context_count(), 0);
    assert_eq!(host.placement_array_count(), 0);

    let fields_before = host.resident_heap().instance_fields();
    assert_eq!(lifetime_text(&fixture, &mut host, "garbage"), "g");
    assert!(
        host.resident_heap().instance_fields() < fields_before + 3000,
        "the garbage call did not collect"
    );
    let freed = lifetime(
        &fixture,
        &mut host,
        "readTag",
        "(Ljava/lang/Object;)Ljava/lang/String;",
        &[dropped_item],
    )
    .expect_err("only the released context held this Tag");
    assert!(
        format!("{freed:?}").contains("was collected while still reachable"),
        "a collected object read as {freed:?}"
    );
    assert_eq!(lifetime_text(&fixture, &mut host, "keptItem"), "tag42");
    assert_eq!(
        lifetime_text(&fixture, &mut host, "keptDirs"),
        expected_dirs
    );
    assert_eq!(lifetime_text(&fixture, &mut host, "kept"), "true|1");

    let accessors = [
        "getClickedPos()Lnet/minecraft/core/BlockPos;",
        "getClickedFace()Lnet/minecraft/core/Direction;",
        "getClickLocation()Lnet/minecraft/world/phys/Vec3;",
        "isInside()Z",
        "getItemInHand()Lnet/minecraft/world/item/ItemStack;",
        "getLevel()Lnet/minecraft/world/level/Level;",
        "canPlace()Z",
        "replacingClickedOnBlock()Z",
        "getPlayer()Lnet/minecraft/world/entity/player/Player;",
        "getHorizontalDirection()Lnet/minecraft/core/Direction;",
        "isSecondaryUseActive()Z",
        "getRotation()F",
        "getNearestLookingDirection()Lnet/minecraft/core/Direction;",
        "getNearestLookingVerticalDirection()Lnet/minecraft/core/Direction;",
        "getNearestLookingDirections()[Lnet/minecraft/core/Direction;",
    ];
    for (which, accessor) in accessors.iter().enumerate() {
        let before = host.unknown_host_call_count();
        let answer = lifetime(
            &fixture,
            &mut host,
            "touchKept",
            "(I)Ljava/lang/String;",
            &[Value::Int(which as i32)],
        )
        .unwrap();
        assert_eq!(answer, Value::Ref(Some("ok".into())), "{accessor}");
        assert_eq!(
            host.unknown_host_call_count(),
            before + 1,
            "{accessor} on a released context must be counted once"
        );
        let call = format!("net/minecraft/world/item/context/BlockPlaceContext#{accessor}");
        assert!(
            host.unknown_host_calls()
                .iter()
                .any(|(counted, _)| *counted == call),
            "expected {call} in {:?}",
            host.unknown_host_calls()
        );
    }
    assert_eq!(host.block_place_context_count(), 0);
    assert_eq!(host.placement_array_count(), 0);
}

/// #635: a placement whose override throws releases its context, and the
/// `Direction[]` it answered first, on the error path too.
#[test]
fn a_placement_that_throws_still_releases_its_context() {
    let fixture = javac_fixture::Fixture::build("block-placement-throws", &sources());
    let mut host = fixture.jar.seed_host(StdHost::new());
    let throwing = lifetime_block(&fixture, &mut host, 2);
    let error = fixture
        .jar
        .block_state_for_placement(
            throwing,
            player_context(0.0, 0.0, "UP", false, true, false),
            &mut host,
        )
        .expect_err("the override throws");
    assert!(
        format!("{error:?}").contains("IllegalStateException"),
        "{error:?}"
    );
    assert_eq!(host.block_place_context_count(), 0);
    assert_eq!(host.placement_array_count(), 0);
}
