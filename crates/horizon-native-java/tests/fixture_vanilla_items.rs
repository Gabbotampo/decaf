#[test]
fn vanilla_items_fixture_present() {
    let rows = include_str!("fixtures/vanilla_items/java25.txt");
    assert_eq!(rows.lines().count(), 1218);
}
