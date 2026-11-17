#[path = "support/javac_fixture.rs"]
mod javac_fixture;
use horizon_native_java::StdHost;
use javac_fixture::Fixture;
const FABRIC: &str = r#"package net.fabricmc.fabric.api.creativetab.v1; public final class FabricCreativeModeTab { public static net.minecraft.world.item.CreativeModeTab.Builder builder(){ throw new AssertionError(); }}"#;
const COMPONENT: &str = r#"package net.minecraft.network.chat; public interface Component { static Component literal(String s){ throw new AssertionError(); } String getString(); }"#;
const TAB: &str = r#"package net.minecraft.world.item; import net.minecraft.network.chat.Component; public class CreativeModeTab { public static class Builder { public Builder title(Component c){throw new AssertionError();} public CreativeModeTab build(){throw new AssertionError();} } public Component getDisplayName(){throw new AssertionError();} }"#;
const JAVA25_CAPTURE: &str = include_str!("fixtures/fabric_creative_tab/java25.txt");
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
            ("net/minecraft/network/chat/Component.java", COMPONENT),
            ("net/minecraft/world/item/CreativeModeTab.java", TAB),
            ("demo/Probe.java", PROBE),
        ],
    );
    let mut h = f.jar.seed_host(StdHost::new());
    assert_eq!(f.text_on(&mut h, "rows"), "true|true|one|one|two");
    assert!(
        h.unknown_host_calls().is_empty(),
        "{:?}",
        h.unknown_host_calls()
    );
}
