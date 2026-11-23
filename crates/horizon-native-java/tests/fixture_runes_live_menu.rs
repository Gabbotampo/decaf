//! Real-jar test of the live Java menu bridge against Runes 1.3.2 (`RUNES_JAR`).
//!
//! Right-clicking `runes:crafting_altar` opens `runes:crafting` (`RuneCraftingScreenHandler`, an
//! `ItemCombinerMenu`) through #629's block-use path; with the player's facts Decaf now runs its
//! `initMenu`. The test then clicks cobblestone and an ender pearl into the two inputs exactly as
//! a client would, which runs Runes' own `createResult` (its `RecipeManager.getRecipeFor` over
//! the recipes the jar's `data/runes/recipe` declares), takes the 16 arcane stones the medium
//! altar recipe makes, and checks Runes' `onTake` consumed one of each input. Closing the window
//! hands the rest back to the inventory.
//!
//! Runes' `onTake` ends by casting the player to its `RuneCrafter` interface (a Mixin on
//! `Player` that is not on this branch) for its sound cooldown: that cast and the calls after it
//! are the counted calls on the take, and the test lists them.

use std::{collections::BTreeMap, path::Path};

use horizon_native_java::{
    BlockHitData, BlockUseContext, BlockUsePlayerData, BlockUseSide, ContainerInput,
    ExecutionBudget, LiveMenuEffect, LiveMenuOutcome, MenuClick, MenuPlayerFacts, NativeJar,
    PlayerContainer, StdHost, Value,
};

const UUID: u128 = 0x00c0_ffee_0000_0000_0000_0000_0000_0042;
const POS: [i32; 3] = [12, 65, -7];

#[test]
fn runes_altar_menu_crafts_arcane_stone_through_live_clicks() {
    let Some(jar_path) = std::env::var_os("RUNES_JAR") else {
        eprintln!("RUNES_JAR is unset; real Runes live-menu test skipped");
        return;
    };
    std::thread::Builder::new()
        .name("runes-live-menu".into())
        .stack_size(16 * 1024 * 1024)
        .spawn(move || run(Path::new(&jar_path)))
        .expect("spawn test thread")
        .join()
        .unwrap();
}

fn player(container: PlayerContainer) -> BlockUsePlayerData {
    BlockUsePlayerData {
        uuid: UUID,
        name: "Runecrafter".to_owned(),
        entity_id: Some(9),
        incarnation: Some((1, 0)),
        container,
        sneaking: false,
        spectator: false,
        main_hand: None,
        off_hand: None,
    }
}

fn facts(inventory: &[Value]) -> MenuPlayerFacts {
    MenuPlayerFacts {
        inventory: Some(inventory.to_vec()),
        selected_slot: 0,
        instabuild: Some(false),
        eye_position: Some([12.5, 66.62, -5.5]),
        block_interaction_range: Some(4.5),
        access_block: Some((
            "runes:crafting_altar".to_owned(),
            BTreeMap::from([("facing".to_owned(), "north".to_owned())]),
        )),
        ..MenuPlayerFacts::default()
    }
}

fn text(host: &StdHost, stack: &Value) -> String {
    let parts = host.item_stack_components(stack).expect("modelled stack");
    if parts.count <= 0 {
        "0 minecraft:air".to_owned()
    } else {
        format!("{} {}", parts.count, parts.item_id)
    }
}

fn click(
    jar: &NativeJar,
    host: &mut StdHost,
    inventory: &mut Vec<Value>,
    slot: i32,
    button: i32,
    input: ContainerInput,
) -> LiveMenuOutcome {
    let outcome = jar
        .click_player_menu(
            host,
            &player(PlayerContainer::Open(1)),
            "minecraft:overworld",
            &facts(inventory),
            &MenuClick {
                container_id: 1,
                slot,
                button,
                input,
                state_id_matches: true,
            },
            &ExecutionBudget::UNBOUNDED,
        )
        .expect("click")
        .expect("the altar menu is held");
    assert_eq!(outcome.failure, None);
    *inventory = outcome.inventory.clone().expect("inventory");
    outcome
}

fn run(jar_path: &Path) {
    let jar = NativeJar::from_path(jar_path).expect("load RUNES_JAR");
    let mut host = jar
        .try_seed_host(StdHost::new().with_game_version("26.3"))
        .expect("seed Runes host");
    for entry in jar.discover_entrypoints("main") {
        jar.execute(&entry.class, entry.method.as_deref(), "main", &mut host)
            .expect("run Runes main");
    }
    host.server_player_created(UUID, 1, 0);

    // Hotbar: 5 cobblestone, 3 ender pearls.
    let mut inventory = vec![StdHost::empty_item_stack(); 43];
    inventory[0] = host.make_item_stack_for_id("minecraft:cobblestone", 5, false);
    inventory[1] = host.make_item_stack_for_id("minecraft:ender_pearl", 3, false);
    host.set_player_menu_facts(UUID, Some(facts(&inventory)));

    // 1. Right-click the altar: `runes:crafting` opens, and its `initMenu` runs exactly.
    let used = jar
        .block_use_without_item(
            &mut host,
            &BlockUseContext {
                side: BlockUseSide::Server,
                dimension: "minecraft:overworld".to_owned(),
                block_id: "runes:crafting_altar".to_owned(),
                state_properties: BTreeMap::from([("facing".to_owned(), "north".to_owned())]),
                pos: POS,
                hit: BlockHitData::block(POS, "UP", [12.5, 66.0, -6.5], false),
                player: player(PlayerContainer::Inventory),
                hand: "MAIN_HAND".to_owned(),
                stack: None,
            },
        )
        .expect("use the altar");
    assert_eq!(used.failure, None);
    assert_eq!(
        used.unknown_host_call_count, 0,
        "opening the altar menu: {:?}",
        used.unknown_host_calls
    );
    let opened = used.open_menus().next().expect("the altar opened its menu");
    assert_eq!(opened.container_id, 1);
    assert_eq!(opened.menu_type_id.as_deref(), Some("runes:crafting"));
    // The synchronizer's first send: the whole menu, hotbar in slots 30 and 31.
    let initial = used
        .live_effects
        .iter()
        .find_map(|effect| match effect {
            LiveMenuEffect::InitialData {
                state_id, items, ..
            } => Some((*state_id, items.clone())),
            _ => None,
        })
        .expect("initMenu's sendAllDataToRemote");
    assert_eq!(initial.0, Some(1));
    assert_eq!(initial.1.len(), 3 + 36);
    assert_eq!(initial.1[30].item_id, "minecraft:cobblestone");
    assert_eq!(initial.1[31].item_id, "minecraft:ender_pearl");
    inventory = used.inventory.clone().expect("inventory");

    // 2. Pick up the cobblestone, right-click one into the base slot, put the rest back.
    let picked = click(
        &jar,
        &mut host,
        &mut inventory,
        30,
        0,
        ContainerInput::Pickup,
    );
    assert_eq!(
        picked.unknown_host_call_count, 0,
        "{:?}",
        picked.unknown_host_calls
    );
    assert_eq!(
        text(&host, picked.carried.as_ref().unwrap()),
        "5 minecraft:cobblestone"
    );
    let placed = click(
        &jar,
        &mut host,
        &mut inventory,
        0,
        1,
        ContainerInput::Pickup,
    );
    assert_eq!(
        placed.unknown_host_call_count, 0,
        "placing the base runs Runes' createResult: {:?}",
        placed.unknown_host_calls
    );
    let slots = placed.slots.clone().unwrap();
    assert_eq!(text(&host, &slots[0]), "1 minecraft:cobblestone");
    assert_eq!(text(&host, &slots[2]), "0 minecraft:air");
    click(
        &jar,
        &mut host,
        &mut inventory,
        30,
        0,
        ContainerInput::Pickup,
    );

    // 3. Shift-click the ender pearls into the addition slot: the medium altar recipe matches.
    let moved = click(
        &jar,
        &mut host,
        &mut inventory,
        31,
        0,
        ContainerInput::QuickMove,
    );
    assert_eq!(
        moved.unknown_host_call_count, 0,
        "RecipeManager.getRecipeFor through the live menu: {:?}",
        moved.unknown_host_calls
    );
    let slots = moved.slots.clone().unwrap();
    assert_eq!(text(&host, &slots[1]), "3 minecraft:ender_pearl");
    assert_eq!(text(&host, &slots[2]), "16 runes:arcane_stone");
    assert!(moved.effects.contains(&LiveMenuEffect::SlotChanged {
        container_id: 1,
        state_id: None,
        slot: 2,
        stack: host.item_stack_components(&slots[2]).unwrap(),
    }));

    // 4. Take the result: Runes' onTake shrinks both inputs by one.
    let taken = click(
        &jar,
        &mut host,
        &mut inventory,
        2,
        0,
        ContainerInput::Pickup,
    );
    assert_eq!(
        text(&host, taken.carried.as_ref().unwrap()),
        "16 runes:arcane_stone"
    );
    let slots = taken.slots.clone().unwrap();
    assert_eq!(text(&host, &slots[0]), "0 minecraft:air");
    assert_eq!(text(&host, &slots[1]), "2 minecraft:ender_pearl");
    assert_eq!(text(&host, &slots[2]), "0 minecraft:air");
    assert!(taken.effects.contains(&LiveMenuEffect::AwardStat {
        stat_type: "minecraft:crafted".to_owned(),
        value: "runes:arcane_stone".to_owned(),
        amount: 16,
    }));
    assert!(
        taken.effects.iter().any(|effect| matches!(
            effect,
            LiveMenuEffect::RecipeCrafted { recipe, .. } if recipe == "runes:arcane_rune_medium_altar"
        )),
        "{:?}",
        taken.effects
    );
    eprintln!(
        "taking the result counted {} calls: {:?}",
        taken.unknown_host_call_count, taken.unknown_host_calls
    );
    assert!(
        taken
            .unknown_host_calls
            .keys()
            .any(|call| call.contains("RuneCrafter")),
        "Runes' RuneCrafter cast is the counted part of onTake: {:?}",
        taken.unknown_host_calls
    );

    // 5. Close the window: the carried stones and the pearls left in the input go back.
    let closed = jar
        .close_player_menu_live(
            &mut host,
            &player(PlayerContainer::Open(1)),
            "minecraft:overworld",
            &facts(&inventory),
            1,
            &ExecutionBudget::UNBOUNDED,
        )
        .expect("close")
        .expect("the altar menu is held");
    assert_eq!(closed.failure, None);
    assert_eq!(
        closed.unknown_host_call_count, 0,
        "{:?}",
        closed.unknown_host_calls
    );
    let inventory = closed.inventory.clone().unwrap();
    let mut held: Vec<String> = inventory
        .iter()
        .filter(|stack| host.item_stack_is_empty(stack) == Some(false))
        .map(|stack| text(&host, stack))
        .collect();
    held.sort();
    assert_eq!(
        held,
        [
            "16 runes:arcane_stone",
            "2 minecraft:ender_pearl",
            "4 minecraft:cobblestone"
        ]
    );
    assert!(host.player_open_menu(UUID).is_none());
}
