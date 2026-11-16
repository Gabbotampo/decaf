//! Captured Java 25 / real 26.3 server-game oracle rows for entity registration.
#[test]
fn captured_real_263_java25_rows_are_stable() {
    let captured = include_str!("fixtures/entity_registration/java25.txt");
    let rows: Vec<&str> = captured.lines().collect();
    assert_eq!(rows.len(), 14);
    // Keep the capture contract explicit: every line is an observed Java-25
    // stdout row, including rows whose native call is counted below.
    let expected = [
        "serial=true,false", "base=0", "sub=1", "base2=1", "sibling=0",
        "cat=MONSTER,monster,M,70,true,true,128,32,MONSTER",
        "cat=CREATURE,creature,C,10,true,true,128,32,CREATURE",
        "cat=AMBIENT,ambient,Am,15,true,true,128,32,AMBIENT",
        "cat=AXOLOTLS,axolotls,Axolotl,5,true,true,128,32,AXOLOTLS",
        "cat=UNDERGROUND_WATER_CREATURE,underground_water_creature,underwater,5,true,true,128,32,UNDERGROUND_WATER_CREATURE",
        "cat=WATER_CREATURE,water_creature, W,5,true,true,128,32,WATER_CREATURE",
        "cat=WATER_AMBIENT,water_ambient,Wa,20,true,true,128,32,WATER_AMBIENT",
        "cat=MISC,misc,MI,-1,true,true,128,32,MISC",
    ];
    for row in expected { assert!(rows.contains(&row), "missing captured row: {row}"); }
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
