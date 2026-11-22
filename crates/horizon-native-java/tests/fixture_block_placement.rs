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
