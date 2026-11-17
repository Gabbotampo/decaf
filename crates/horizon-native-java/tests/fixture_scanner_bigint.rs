#[test]
fn scanner_bigint_java25_capture_is_not_yet_modelled() {
    let capture = include_str!("fixtures/scanner_bigint/oracle/java25.txt");
    assert!(!capture.is_empty());
    panic!("Scanner(File) model pending");
}
