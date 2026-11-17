#[test]
fn vanilla_items_host_rows_match_capture() {
    let capture = include_str!("fixtures/vanilla_items/java25.txt");
    assert!(
        capture
            .lines()
            .any(|r| r == "DRY_SHORT_GRASS|minecraft:short_dry_grass")
    );
    assert!(capture.lines().any(|r| r == "STONE|minecraft:stone"));
    assert!(capture.lines().all(|r| r.contains('|')));
}
