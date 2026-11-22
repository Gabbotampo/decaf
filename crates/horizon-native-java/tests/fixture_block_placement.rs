//! `Block.getStateForPlacement` runs a jar override through a real virtual
//! dispatch with a host-owned `BlockPlaceContext` snapshot.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::{BlockPlaceContextData, HostKind, HostRef, StdHost, Value};

fn direction(name: &str) -> Value {
    Value::Ref(Some(format!("enum/net/minecraft/core/Direction/{name}")))
}

fn context(yaw: f32, face: &str, can_place: bool) -> BlockPlaceContextData {
    BlockPlaceContextData {
        clicked_pos: [4, 5, 6],
        clicked_face: direction(face),
        click_location: [4.5, 5.25, 6.75],
        inside: false,
        yaw,
        pitch: 0.0,
        secondary_use: false,
        replacing_clicked_on_block: false,
        can_place,
        player: Value::Host(HostRef::new(HostKind::GameEntity, "game/player/test")),
        item_in_hand: StdHost::empty_item_stack(),
        level: Value::Host(HostRef::new(HostKind::GameLevel, "game/level/overworld")),
    }
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
            "package net.minecraft.world.level.block.state; import net.minecraft.world.level.block.state.properties.Property; public class BlockState { public <T extends Comparable<T>> BlockState setValue(Property<T> p, T v) { return this; } public <T extends Comparable<T>> T getValue(Property<T> p) { return null; } }",
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
            "net/minecraft/world/item/context/BlockPlaceContext.java",
            "package net.minecraft.world.item.context; import net.minecraft.core.*; import net.minecraft.world.entity.player.Player; import net.minecraft.world.item.ItemStack; import net.minecraft.world.level.Level; public class BlockPlaceContext { public Direction getHorizontalDirection(){return null;} public Direction getClickedFace(){return null;} public BlockPos getClickedPos(){return null;} public boolean canPlace(){return true;} public boolean replacingClickedOnBlock(){return false;} public Direction getNearestLookingDirection(){return null;} public Direction getNearestLookingVerticalDirection(){return null;} public Direction[] getNearestLookingDirections(){return null;} public float getRotation(){return 0;} public boolean isSecondaryUseActive(){return false;} public Player getPlayer(){return null;} public ItemStack getItemInHand(){return null;} public Level getLevel(){return null;} public Object getClickLocation(){return null;} public boolean isInside(){return false;} public Object getHand(){return null;} }",
        ),
        (
            "net/minecraft/core/BlockPos.java",
            "package net.minecraft.core; public class BlockPos { public int getX(){return 0;} }",
        ),
        (
            "net/minecraft/world/level/block/Block.java",
            "package net.minecraft.world.level.block; import net.minecraft.world.level.block.state.*; import net.minecraft.world.level.block.state.properties.*; public class Block { protected BlockState defaultBlockState(){return new BlockState();} protected void createBlockStateDefinition(StateDefinition.Builder<Block,BlockState> b){} public BlockState getStateForPlacement(net.minecraft.world.item.context.BlockPlaceContext c){return null;} }",
        ),
        (
            "demo/PlacementBlock.java",
            "package demo; import net.minecraft.core.*; import net.minecraft.world.level.block.*; import net.minecraft.world.level.block.state.*; import net.minecraft.world.level.block.state.properties.*; import net.minecraft.world.item.context.*; public class PlacementBlock extends Block { public static final EnumProperty<Direction> FACING=BlockStateProperties.HORIZONTAL_FACING; @Override protected void createBlockStateDefinition(StateDefinition.Builder<Block,BlockState> b){b.add(FACING);} @Override public BlockState getStateForPlacement(BlockPlaceContext c) { c.getHand(); if (!c.canPlace()) return null; return defaultBlockState().setValue(FACING, c.getHorizontalDirection().getOpposite()); } }",
        ),
        (
            "demo/Probe.java",
            "package demo; import net.minecraft.core.Direction; import net.minecraft.world.item.context.BlockPlaceContext; public class Probe { public static String place(demo.PlacementBlock b, BlockPlaceContext c) { net.minecraft.world.level.block.state.BlockState s=b.getStateForPlacement(c); return s == null ? \"null\" : String.valueOf(s.getValue(demo.PlacementBlock.FACING)); } public static String basic(BlockPlaceContext c) { return (c.getPlayer()==null)+\":\"+c.getHorizontalDirection()+\":\"+c.getRotation()+\":\"+c.isSecondaryUseActive()+\":\"+c.getItemInHand().isEmpty(); } public static String context(BlockPlaceContext c) { Direction[] d=c.getNearestLookingDirections(); return basic(c)+\":\"+d[0]+\",\"+d[1]+\",\"+d[2]+\",\"+d[3]+\",\"+d[4]+\",\"+d[5]; } }",
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
            let context = host.new_block_place_context(context(yaw, face, true));
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
    let context = host.new_block_place_context(context(0.0, "UP", false));
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
fn placement_context_matches_mth_and_vanilla_null_player_branches() {
    let fixture = javac_fixture::Fixture::build("block-placement-context", &sources());
    let mut host = fixture.jar.seed_host(StdHost::new());
    let desc = "(Lnet/minecraft/world/item/context/BlockPlaceContext;)Ljava/lang/String;";
    let context_value = host.new_block_place_context(context(0.0, "NORTH", true));
    let value = fixture
        .jar
        .invoke_method_with_args(
            "demo/Probe",
            "context",
            desc,
            None,
            &[context_value],
            &mut host,
        )
        .unwrap();
    assert_eq!(
        value,
        Value::Ref(Some(
            "false:SOUTH:0.0:false:true:SOUTH,NORTH,DOWN,WEST,UP,EAST".into()
        ))
    );

    let yaw_135 = host.new_block_place_context(context(135.0, "UP", true));
    let value = fixture
        .jar
        .invoke_method_with_args("demo/Probe", "context", desc, None, &[yaw_135], &mut host)
        .unwrap();
    assert!(
        matches!(value, Value::Ref(Some(ref text)) if text.starts_with("false:NORTH:")),
        "{value:?}"
    );

    let mut no_player = context(90.0, "UP", true);
    no_player.player = Value::Ref(None);
    let context = host.new_block_place_context(no_player);
    let value = fixture
        .jar
        .invoke_method_with_args("demo/Probe", "basic", desc, None, &[context], &mut host)
        .unwrap();
    assert_eq!(value, Value::Ref(Some("true:NORTH:0.0:false:true".into())));
}
