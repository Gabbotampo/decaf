//! Real Runes 1.3.2 item tooltip execution through the general NativeJar API.

use horizon_native_java::{ExecutionBudget, HostKind, HostRef, NativeJar, StdHost, Value};
use std::{fs, path::PathBuf};

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

const TOOLTIP_STUBS: &[(&str, &str)] = &[
    (
        "net/minecraft/ChatFormatting.java",
        "package net.minecraft; public enum ChatFormatting { GRAY, ITALIC; }",
    ),
    (
        "net/minecraft/network/chat/Component.java",
        "package net.minecraft.network.chat; public interface Component { static MutableComponent translatable(String k, Object... a) { return null; } static MutableComponent literal(String s) { return null; } }",
    ),
    (
        "net/minecraft/network/chat/MutableComponent.java",
        "package net.minecraft.network.chat; import net.minecraft.ChatFormatting; public interface MutableComponent extends Component { MutableComponent withStyle(ChatFormatting... f); }",
    ),
    (
        "net/minecraft/world/item/ItemStack.java",
        "package net.minecraft.world.item; public class ItemStack {}",
    ),
    (
        "net/minecraft/world/item/component/TooltipDisplay.java",
        "package net.minecraft.world.item.component; public final class TooltipDisplay {}",
    ),
    (
        "net/minecraft/world/item/TooltipFlag.java",
        "package net.minecraft.world.item; public interface TooltipFlag { boolean isAdvanced(); boolean isCreative(); }",
    ),
    (
        "net/minecraft/world/level/block/Block.java",
        "package net.minecraft.world.level.block; public class Block {}",
    ),
    (
        "net/minecraft/world/item/Item.java",
        "package net.minecraft.world.item; import java.util.function.Consumer; import net.minecraft.network.chat.Component; import net.minecraft.world.item.component.TooltipDisplay; public class Item { public static class Properties {} public interface TooltipContext {} public Item(Properties p) {} public void appendHoverText(ItemStack s, TooltipContext c, TooltipDisplay d, Consumer<Component> o, TooltipFlag f) {} }",
    ),
    (
        "net/minecraft/world/item/BlockItem.java",
        "package net.minecraft.world.item; import java.util.function.Consumer; import net.minecraft.network.chat.Component; import net.minecraft.world.item.component.TooltipDisplay; import net.minecraft.world.level.block.Block; public class BlockItem extends Item { public BlockItem(Block b, Properties p) { super(p); } @Override public void appendHoverText(ItemStack s, TooltipContext c, TooltipDisplay d, Consumer<Component> o, TooltipFlag f) { super.appendHoverText(s,c,d,o,f); } }",
    ),
];

const TOOLTIP_SOURCE: &str = r#"package demo;
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
    @Override public void appendHoverText(ItemStack s, TooltipContext c, TooltipDisplay d, Consumer<Component> out, TooltipFlag f) { super.appendHoverText(s,c,d,out,f); out.accept(Component.translatable("demo.tip", "arg")); out.accept(Component.translatable("styled", "x")); }
  }
  static final class BlockOverride extends BlockItem {
    BlockOverride(net.minecraft.world.level.block.Block b, Properties p) { super(b, p); }
    @Override public void appendHoverText(ItemStack s, TooltipContext c, TooltipDisplay d, Consumer<Component> out, TooltipFlag f) { super.appendHoverText(s,c,d,out,f); out.accept(Component.translatable("demo.block", 7)); }
  }
}"#;

#[test]
fn runes_altar_item_appends_its_translatable_styled_line() {
    let Some(path) = std::env::var_os("RUNES_JAR").map(PathBuf::from) else {
        eprintln!("skipped: RUNES_JAR is unset");
        return;
    };
    if !path.exists() {
        eprintln!("skipped: RUNES_JAR does not exist: {path:?}");
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
    let mut sources = TOOLTIP_STUBS.to_vec();
    sources.push(("demo/TooltipItems.java", TOOLTIP_SOURCE));
    // Expected JSON comes from the equivalent real 26.3 server-class capture.
    let fixture = javac_fixture::Fixture::build_every_package("item-tooltip", &sources);
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
        r#"{"translate":"styled","with":["x"]}"#
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
