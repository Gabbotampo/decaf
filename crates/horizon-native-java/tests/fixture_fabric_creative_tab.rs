#[path = "support/javac_fixture.rs"]
mod javac_fixture;
use horizon_native_java::StdHost;
use javac_fixture::Fixture;
const FABRIC: &str = r#"package net.fabricmc.fabric.api.creativetab.v1; public final class FabricCreativeModeTab { public static net.minecraft.world.item.CreativeModeTab.Builder builder(){ throw new AssertionError(); }}"#;
const PROBE: &str = include_str!("fixtures/fabric_creative_tab/demo/Probe.java");
#[test]
fn fabric_creative_tab_fixture() {
    let f = Fixture::build(
        "fabric-creative-tab",
        &[
            (
                "net/fabricmc/fabric/api/creativetab/v1/FabricCreativeModeTab.java",
                FABRIC,
            ),
            ("demo/Probe.java", PROBE),
        ],
    );
    let mut h = f.jar.seed_host(StdHost::new());
    assert_eq!(
        f.text_on(&mut h, "rows"),
        "b1!=b2|t1!=t2|built-twice|name|hash"
    );
}
