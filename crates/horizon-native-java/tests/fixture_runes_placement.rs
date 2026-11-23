//! Live Runes altar placement through the general block-context bridge.

use horizon_native_java::{
    BlockPlaceContextData, Entrypoint, HostKind, HostRef, NativeJar, Placer, StdHost, Value,
};
use std::path::Path;

fn direction(name: &str) -> Value {
    Value::Ref(Some(format!("enum/net/minecraft/core/Direction/{name}")))
}

fn player_context(yaw: f32) -> BlockPlaceContextData {
    BlockPlaceContextData {
        clicked_pos: [0, 64, 0],
        clicked_face: direction("UP"),
        click_location: [0.5, 65.0, 0.5],
        inside: false,
        replacing_clicked_on_block: false,
        can_place: true,
        placer: Placer::Player {
            player: Value::Host(HostRef::new(
                HostKind::GameEntity,
                "game/player/placement-test",
            )),
            yaw,
            pitch: 0.0,
            secondary_use: false,
        },
        item_in_hand: StdHost::empty_item_stack(),
        level: Value::Host(HostRef::new(
            HostKind::GameLevel,
            "game/level/minecraft:overworld",
        )),
    }
}

fn unknown_placer_context() -> BlockPlaceContextData {
    BlockPlaceContextData {
        clicked_pos: [0, 64, 0],
        clicked_face: direction("UP"),
        click_location: [0.5, 65.0, 0.5],
        inside: false,
        replacing_clicked_on_block: false,
        can_place: true,
        placer: Placer::Unknown,
        item_in_hand: StdHost::empty_item_stack(),
        level: Value::Host(HostRef::new(
            HostKind::GameLevel,
            "game/level/minecraft:overworld",
        )),
    }
}

#[test]
fn real_runes_altar_placement_uses_opposite_horizontal_look() {
    let Some(path) = std::env::var_os("RUNES_JAR") else {
        eprintln!("RUNES_JAR is unset; real Runes placement test skipped");
        return;
    };
    let jar = NativeJar::from_path(Path::new(&path)).expect("load real Runes jar");
    let mut host = StdHost::new();
    let entries = jar.discover_entrypoints("main");
    assert!(!entries.is_empty(), "Runes has no main entrypoint");
    for Entrypoint { class, method, .. } in entries {
        jar.execute(&class, method.as_deref(), "main", &mut host)
            .expect("Runes main entrypoint");
    }
    assert_eq!(
        host.unknown_host_call_count(),
        0,
        "Runes main entrypoint unknowns: {:?}",
        host.unknown_host_calls()
    );
    let block = host
        .registrations()
        .iter()
        .find(|entry| entry.registry == "minecraft:block" && entry.id == "runes:crafting_altar")
        .map(|entry| entry.value.clone())
        .expect("Runes altar registration");

    for (yaw, expected) in [
        (0.0, "NORTH"),
        (90.0, "EAST"),
        (180.0, "SOUTH"),
        (-90.0, "WEST"),
    ] {
        let expected = expected.to_ascii_lowercase();
        let state = jar
            .block_state_for_placement(block.clone(), player_context(yaw), &mut host)
            .expect("placement override")
            .expect("altar is placeable");
        let Value::Ref(Some(marker)) = state else {
            panic!("unexpected state {state:?}")
        };
        let (block_id, properties) = host
            .block_state_content(&Value::Ref(Some(marker.clone())))
            .expect("altar state content");
        assert_eq!(block_id, "runes:crafting_altar");
        assert_eq!(
            properties.get("facing").map(String::as_str),
            Some(expected.as_str())
        );
        assert!(
            marker.contains(&format!("facing={expected}")),
            "{marker} for yaw {yaw}"
        );
    }
    assert_eq!(host.unknown_host_call_count(), 0);
    // Each placement's context snapshot ended with its call (#635).
    assert_eq!(host.block_place_context_count(), 0);
    assert_eq!(host.placement_array_count(), 0);

    // Under Placer::Unknown (the honest state until the SDK supplies placer identity),
    // Runes' getHorizontalDirection() call is counted rather than guessed.
    let _ = jar.block_state_for_placement(block, unknown_placer_context(), &mut host);
    assert!(
        host.unknown_host_call_count() >= 1,
        "expected unknown placer call to be counted"
    );
    assert!(
        host.unknown_host_calls().iter().any(|(call, _)| call
            == "net/minecraft/world/item/context/BlockPlaceContext#getHorizontalDirection()Lnet/minecraft/core/Direction;"),
        "expected getHorizontalDirection unknown, got {:?}",
        host.unknown_host_calls()
    );
    assert_eq!(host.block_place_context_count(), 0);
}
