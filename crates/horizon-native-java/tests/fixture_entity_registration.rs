//! Captured 26.3 server-game oracle rows for SynchedEntityData's class-tree IDs.
#[test]
fn captured_real_263_rows_are_stable() {
    let captured = include_str!("fixtures/entity_registration/java27.txt");
    assert_eq!(captured, "base=0\nsub=1\nbase2=1\n");
}
