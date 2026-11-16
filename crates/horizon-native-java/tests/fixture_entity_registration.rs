//! Captured Java 25 / real 26.3 server-game oracle rows for entity registration.
#[test]
fn captured_real_263_java25_rows_are_stable() {
    let captured = include_str!("fixtures/entity_registration/java25.txt");
    let rows: Vec<&str> = captured.lines().collect();
    assert_eq!(rows.len(), 14);
    assert!(rows.contains(&"serial=true,false"));
    assert!(rows.contains(&"base=0"));
    assert!(rows.contains(&"sub=1"));
    assert!(rows.contains(&"base2=1"));
    assert!(rows.contains(&"sibling=0"));
    assert!(
        rows.iter()
            .any(|r| r.starts_with("overflow=Data value id is too big with 255!"))
    );
    assert!(rows.contains(&"cat=CREATURE,creature,C,10,true,true,128,32,CREATURE"));
    assert!(rows.contains(&"cat=MISC,misc,MI,-1,true,true,128,32,MISC"));
}
