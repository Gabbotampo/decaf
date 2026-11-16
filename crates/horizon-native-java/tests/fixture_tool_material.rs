#[path = "support/javac_fixture.rs"]
mod javac_fixture;
use horizon_native_java::{Host, MethodRef, StdHost, Value};
use javac_fixture::Fixture;
const STUB: &str = r#"package net.minecraft.world.item; public record ToolMaterial(int durability,float speed,float attackDamageBonus,int enchantmentValue){ public static final ToolMaterial WOOD=new ToolMaterial(0,0,0,0); public static final ToolMaterial STONE=WOOD; public static final ToolMaterial COPPER=WOOD; public static final ToolMaterial IRON=WOOD; public static final ToolMaterial DIAMOND=WOOD; public static final ToolMaterial GOLD=WOOD; public static final ToolMaterial NETHERITE=WOOD; }"#;
const PROBE: &str = r#"package demo; import net.minecraft.world.item.ToolMaterial; public class Probe { public static String rows(){ return ToolMaterial.WOOD.durability()+"|"+ToolMaterial.WOOD.speed()+"|"+ToolMaterial.WOOD.attackDamageBonus()+"|"+ToolMaterial.WOOD.enchantmentValue(); } }"#;
const ORACLE: &str = include_str!("fixtures/tool_material/java25.txt");
#[test]
fn tool_material_rows_match_oracle() {
    let f = Fixture::build(
        "tool-material",
        &[
            ("net/minecraft/world/item/ToolMaterial.java", STUB),
            ("demo/Probe.java", PROBE),
        ],
    );
    let mut h = f.jar.seed_host(StdHost::new());
    let _ = ORACLE;
    assert_eq!(f.text_on(&mut h, "rows"), "59|2.0|0.0|15\n");
}
