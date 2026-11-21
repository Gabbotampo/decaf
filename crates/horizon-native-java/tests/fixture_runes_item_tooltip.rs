//! Real Runes 1.3.2 item tooltip execution through the general NativeJar API.

use horizon_native_java::{ExecutionBudget, HostKind, HostRef, NativeJar, StdHost, Value};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

#[test]
fn runes_altar_item_appends_its_translatable_styled_line() {
    let path =
        Path::new("/home/solenopsisbot/horizon-corpus/deps-pool/runes-fabric-1.3.2+26.3.jar");
    if !path.exists() {
        eprintln!("skipped: missing Runes jar {path:?}");
        return;
    }
    let jar = NativeJar::from_bytes(&fs::read(path).unwrap()).unwrap();
    let item = Value::Host(HostRef::new(
        HostKind::JarObject,
        "net/runes/crafting/RuneCraftingBlock$1#0",
    ));
    let mut host = StdHost::new();
    jar.invoke_method_with_args(
        "net/runes/crafting/RuneCraftingBlock$1",
        "<init>",
        "(Lnet/minecraft/world/level/block/Block;Lnet/minecraft/world/item/Item$Properties;)V",
        Some(item.clone()),
        &[Value::Ref(None), Value::Ref(None)],
        &mut host,
    )
    .unwrap();
    let lines = jar
        .run_item_tooltip(
            &item,
            Value::Ref(Some("client/stack/tooltip".into())),
            Value::Ref(Some("client/tooltip-context".into())),
            Value::Ref(Some("client/tooltip-display".into())),
            Value::Ref(Some("client/tooltip-flag".into())),
            &mut host,
            &ExecutionBudget::UNBOUNDED,
        )
        .unwrap();
    assert_eq!(lines.len(), 1);
    let line = &lines[0];
    assert_eq!(
        host.component_json(line).unwrap(),
        r#"{"translate":"block.runes.crafting_altar.hint","color":"gray","italic":true}"#
    );
    assert!(
        host.unknown_host_calls().is_empty(),
        "{:?}",
        host.unknown_host_calls()
    );
}

#[test]
fn javac_item_and_block_item_overrides_run_through_nativejar() {
    let game = Path::new(
        "/home/solenopsisbot/coding/hj-scratch/mc263-server/META-INF/versions/26.3/server-26.3.jar",
    );
    if !game.exists() {
        eprintln!("skipped: missing 26.3 server jar");
        return;
    }
    let brigadier = "/home/solenopsisbot/coding/hj-scratch/mc263-server/META-INF/libraries/com/mojang/brigadier/1.3.11/brigadier-1.3.11.jar";
    let classpath = PathBuf::from(format!("{}:{}", game.display(), brigadier));
    let fixture = javac_fixture::Fixture::build_with_real_classpath(
        "item-tooltip",
        &[(
            "demo/TooltipItems.java",
            r#"
package demo;
import java.util.function.Consumer;
import net.minecraft.ChatFormatting;
import net.minecraft.network.chat.Component;
import net.minecraft.world.item.*;
import net.minecraft.world.item.component.TooltipDisplay;
public final class TooltipItems {
  public static Item item() { return new ItemOverride(new Item.Properties()); }
  public static Item block() { return new BlockOverride(null, new Item.Properties()); }
  static final class ItemOverride extends Item {
    ItemOverride(Properties p) { super(p); }
    @Override public void appendHoverText(ItemStack s, TooltipContext c, TooltipDisplay d,
        Consumer<Component> out, TooltipFlag f) {
      super.appendHoverText(s, c, d, out, f);
      out.accept(Component.translatable("demo.tip", "arg"));
      out.accept(Component.literal("styled").withStyle(ChatFormatting.GRAY, ChatFormatting.ITALIC));
    }
  }
  static final class BlockOverride extends BlockItem {
    BlockOverride(net.minecraft.world.level.block.Block b, Properties p) { super(b, p); }
    @Override public void appendHoverText(ItemStack s, TooltipContext c, TooltipDisplay d,
        Consumer<Component> out, TooltipFlag f) {
      super.appendHoverText(s, c, d, out, f);
      out.accept(Component.translatable("demo.block", 7));
    }
  }
}
"#,
        )],
        &classpath,
        r#"{"schemaVersion":1,"id":"fixture"}"#,
    );
    let mut host = StdHost::new();
    let run = |name: &str, host: &mut StdHost| {
        let item = fixture
            .jar
            .invoke_method(
                "demo/TooltipItems",
                name,
                "()Lnet/minecraft/world/item/Item;",
                None,
                host,
            )
            .unwrap();
        fixture
            .jar
            .run_item_tooltip(
                &item,
                Value::Ref(Some("client/stack/tooltip".into())),
                Value::Ref(Some("client/tooltip-context".into())),
                Value::Ref(Some("client/tooltip-display".into())),
                Value::Ref(Some("client/tooltip-flag".into())),
                host,
                &ExecutionBudget::UNBOUNDED,
            )
            .unwrap()
    };
    let item_lines = run("item", &mut host);
    assert_eq!(item_lines.len(), 2);
    assert_eq!(
        host.component_json(&item_lines[0]).unwrap(),
        r#"{"translate":"demo.tip","with":["arg"]}"#
    );
    assert_eq!(
        host.component_json(&item_lines[1]).unwrap(),
        r#"{"text":"styled","color":"gray","italic":true}"#
    );
    let block_lines = run("block", &mut host);
    assert_eq!(block_lines.len(), 1);
    assert_eq!(
        host.component_json(&block_lines[0]).unwrap(),
        r#"{"translate":"demo.block","with":[7]}"#
    );
    assert!(
        host.unknown_host_calls().is_empty(),
        "{:?}",
        host.unknown_host_calls()
    );
}
