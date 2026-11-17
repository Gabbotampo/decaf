#[test]
fn scanner_bigint_java25_capture_present() {
    let capture = include_str!("fixtures/scanner_bigint/java25.txt");
    assert!(!capture.is_empty());
}
