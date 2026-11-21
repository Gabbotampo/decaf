//! Item-owned `appendHoverText` tooltip execution through the general `NativeJar` API.
//!
//! Covers:
//! - Real Runes 1.3.2 altar `BlockItem` tooltip (`RUNES_JAR` env var, opt-in for CI).
//! - Vanilla `ComponentSerialization.CODEC` + `JsonOps.INSTANCE` comparison against real JDK 25 +
//!   MC 26.3 server captures (`fixtures/runes_item_tooltip/java25.txt`).
//! - Opaque `Consumer<Component>` sink semantics (exact `instanceof`, counted `Object` methods).
//! - Disallowed translation arguments (`null`, `Character`, arbitrary objects) counted, never
//!   stringified.
//! - Override lookup across linked jars, ignoring `static`/`private` methods, counting
//!   unresolvable superclasses.
//! - Frame-backed `TooltipContext.tickRate()`, removal of invented `level()`/`player()`, dedicated
//!   handles for `TooltipContext`/`TooltipDisplay`/`TooltipFlag` with counted `Object` methods,
//!   and honouring supplied `TooltipFlag` (`isAdvanced`, `isCreative`).

use horizon_native_java::{
    ExecutionBudget, HostKind, HostRef, NativeJar, StdHost, Value,
    client_surface::{
        ClientLevel, ClientPlayer, ClientSnapshot, tooltip_context, tooltip_context_empty,
        tooltip_display, tooltip_flag, tooltip_stack,
    },
};
use std::{collections::BTreeMap, fs, path::PathBuf, sync::Arc};

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

const JAVA25_CAPTURES: &str = include_str!("fixtures/runes_item_tooltip/java25.txt");

const TOOLTIP_STUBS: &[(&str, &str)] = &[
    (
        "net/minecraft/ChatFormatting.java",
        "package net.minecraft; public enum ChatFormatting { BLACK, DARK_BLUE, DARK_GREEN, DARK_AQUA, DARK_RED, DARK_PURPLE, GOLD, GRAY, DARK_GRAY, BLUE, GREEN, AQUA, RED, LIGHT_PURPLE, YELLOW, WHITE, OBFUSCATED, BOLD, STRIKETHROUGH, UNDERLINE, ITALIC, RESET; }",
    ),
    (
        "net/minecraft/core/HolderLookup.java",
        "package net.minecraft.core; public interface HolderLookup { interface Provider {} }",
    ),
    (
        "net/minecraft/world/level/Level.java",
        "package net.minecraft.world.level; public abstract class Level {}",
    ),
    (
        "net/minecraft/world/level/saveddata/maps/MapId.java",
        "package net.minecraft.world.level.saveddata.maps; public final class MapId {}",
    ),
    (
        "net/minecraft/world/level/saveddata/maps/MapItemSavedData.java",
        "package net.minecraft.world.level.saveddata.maps; public class MapItemSavedData {}",
    ),
    (
        "net/minecraft/network/chat/Component.java",
        "package net.minecraft.network.chat; public interface Component { static MutableComponent translatable(String k) { return null; } static MutableComponent translatable(String k, Object... a) { return null; } static MutableComponent translatableEscape(String k, Object... a) { return null; } static MutableComponent translatableWithFallback(String k, String fb, Object... a) { return null; } static MutableComponent literal(String s) { return null; } String getString(); }",
    ),
    (
        "net/minecraft/network/chat/MutableComponent.java",
        "package net.minecraft.network.chat; import net.minecraft.ChatFormatting; public interface MutableComponent extends Component { MutableComponent withStyle(ChatFormatting f); MutableComponent withStyle(ChatFormatting... f); MutableComponent withColor(int rgb); MutableComponent append(Component c); MutableComponent append(String s); MutableComponent copy(); MutableComponent plainCopy(); }",
    ),
    (
        "net/minecraft/world/item/ItemStack.java",
        "package net.minecraft.world.item; public class ItemStack {}",
    ),
    (
        "net/minecraft/world/item/component/TooltipDisplay.java",
        "package net.minecraft.world.item.component; public final class TooltipDisplay { public static final TooltipDisplay DEFAULT = null; public boolean hideTooltip() { return false; } }",
    ),
    (
        "net/minecraft/world/item/TooltipFlag.java",
        "package net.minecraft.world.item; public interface TooltipFlag { Default NORMAL = new Default(false, false); Default ADVANCED = new Default(true, false); boolean isAdvanced(); boolean isCreative(); record Default(boolean advanced, boolean creative) implements TooltipFlag { public boolean isAdvanced() { return advanced; } public boolean isCreative() { return creative; } public Default asCreative() { return new Default(advanced, true); } } }",
    ),
    (
        "net/minecraft/world/level/block/Block.java",
        "package net.minecraft.world.level.block; public class Block {}",
    ),
    (
        "net/minecraft/world/item/Item.java",
        "package net.minecraft.world.item; import java.util.function.Consumer; import net.minecraft.core.HolderLookup; import net.minecraft.network.chat.Component; import net.minecraft.world.item.component.TooltipDisplay; import net.minecraft.world.level.Level; import net.minecraft.world.level.saveddata.maps.MapId; import net.minecraft.world.level.saveddata.maps.MapItemSavedData; public class Item { public static class Properties {} public interface TooltipContext { TooltipContext EMPTY = null; static TooltipContext of(Level level) { return null; } float tickRate(); HolderLookup.Provider registries(); MapItemSavedData mapData(MapId id); boolean isPeaceful(); Object level(); Object player(); } public Item(Properties p) {} public void appendHoverText(ItemStack s, TooltipContext c, TooltipDisplay d, Consumer<Component> o, TooltipFlag f) {} }",
    ),
    (
        "net/minecraft/world/item/BlockItem.java",
        "package net.minecraft.world.item; import net.minecraft.world.level.block.Block; public class BlockItem extends Item { public BlockItem(Block b, Properties p) { super(p); } }",
    ),
    (
        "ext/UnlinkedBaseItem.java",
        "package ext; import net.minecraft.world.item.Item; public class UnlinkedBaseItem extends Item { public UnlinkedBaseItem(Properties p) { super(p); } }",
    ),
];

const LIB_BASE_SOURCE: &str = r#"package lib;
import java.util.function.Consumer;
import net.minecraft.ChatFormatting;
import net.minecraft.network.chat.Component;
import net.minecraft.world.item.*;
import net.minecraft.world.item.component.TooltipDisplay;
public class BaseTooltipItem extends Item {
  public BaseTooltipItem(Properties p) { super(p); }
  @Override
  public void appendHoverText(ItemStack s, TooltipContext c, TooltipDisplay d, Consumer<Component> out, TooltipFlag f) {
    super.appendHoverText(s, c, d, out, f);
    out.accept(Component.translatable("lib.base.line").withStyle(ChatFormatting.GRAY));
  }
}"#;

const DEMO_TOOLTIP_SOURCE: &str = r#"package demo;
import ext.UnlinkedBaseItem;
import java.util.ArrayList;
import java.util.List;
import java.util.function.Consumer;
import lib.BaseTooltipItem;
import net.minecraft.ChatFormatting;
import net.minecraft.network.chat.Component;
import net.minecraft.network.chat.MutableComponent;
import net.minecraft.world.item.*;
import net.minecraft.world.item.component.TooltipDisplay;
import net.minecraft.world.level.Level;

public final class TooltipProbe {
  static final class CustomLabel {
    @Override
    public String toString() {
      return "custom-str";
    }
  }

  static final class NullToString {
    @Override
    public String toString() {
      return null;
    }
  }

  static final class PlainObj {}

  record Pair(int x, String y) {}

  public static Item codecCases() { return new CodecCasesItem(new Item.Properties()); }
  public static Item blockItem() { return new BlockOverride(null, new Item.Properties()); }
  public static Item plainItem() { return new PlainItem(new Item.Properties()); }
  public static Item plainBlockItem() { return new PlainBlockItem(null, new Item.Properties()); }
  public static Item linkedSubItem() { return new LinkedSubItem(new Item.Properties()); }
  public static Item unlinkedSubItem() { return new UnlinkedSubItem(new Item.Properties()); }
  public static Item consumerChecks() { return new ConsumerChecksItem(new Item.Properties()); }
  public static Item consumerObjectMethod(int which) { return new ConsumerObjectMethodItem(new Item.Properties(), which); }
  public static Item handleChecks() { return new HandleChecksItem(new Item.Properties()); }
  public static Item handleObjectMethod(int target, int which) { return new HandleObjectMethodItem(new Item.Properties(), target, which); }
  public static Item flagReader() { return new FlagReaderItem(new Item.Properties()); }
  public static Item flagDefaultOracle() { return new FlagDefaultOracleItem(new Item.Properties()); }
  public static Item emptyContextOracle() { return new EmptyContextOracleItem(new Item.Properties()); }
  public static Item emptyContextObjectMethod(int which) { return new EmptyContextObjectMethodItem(new Item.Properties(), which); }
  public static Item tickRateReader() { return new TickRateReaderItem(new Item.Properties()); }
  public static Item contextMethodCaller(int which) { return new ContextMethodCallerItem(new Item.Properties(), which); }
  public static Item badArgItem(int which) { return new BadArgItem(new Item.Properties(), which); }
  public static Item badArgGetString(int which) { return new BadArgGetStringItem(new Item.Properties(), which); }
  public static Item escapeEdgeItem(int which) { return new EscapeEdgeItem(new Item.Properties(), which); }

  static final class CodecCasesItem extends Item {
    CodecCasesItem(Properties p) { super(p); }
    @Override
    public void appendHoverText(ItemStack s, TooltipContext c, TooltipDisplay d, Consumer<Component> out, TooltipFlag f) {
      super.appendHoverText(s, c, d, out, f);
      out.accept(Component.literal("hello"));
      out.accept(Component.literal(""));
      out.accept(Component.literal("styled").withStyle(ChatFormatting.GRAY, ChatFormatting.ITALIC));
      out.accept(Component.literal("hex").withColor(0xABCDEF));
      out.accept(Component.translatable("block.runes.crafting_altar.hint"));
      out.accept(Component.translatable("block.runes.crafting_altar.hint").withStyle(ChatFormatting.GRAY, ChatFormatting.ITALIC));
      out.accept(Component.translatable("demo.tip", "arg"));
      out.accept(Component.translatable("demo.block", 7));
      out.accept(Component.translatable("demo.prim", true, 42L, 1.5, 2.5f));
      out.accept(Component.translatable("demo.lit", Component.literal("foo")));
      out.accept(Component.translatable("demo.styled", Component.literal("foo").withStyle(ChatFormatting.RED)));
      out.accept(Component.translatable("demo.nested", Component.translatable("inner", Component.literal("bar"))));
      out.accept(Component.translatableWithFallback("demo.fb", "fb", "a", Component.literal("b")).withStyle(ChatFormatting.ITALIC, ChatFormatting.GRAY).append(Component.literal("e")));
      out.accept(Component.literal("a").withStyle(ChatFormatting.GRAY).append(Component.literal("b")));
      out.accept(Component.literal("a").append(Component.literal("b")).withStyle(ChatFormatting.GRAY));
      out.accept(Component.literal("x").withStyle(ChatFormatting.ITALIC, ChatFormatting.GRAY, ChatFormatting.BOLD));
      out.accept(Component.literal("x").withStyle(ChatFormatting.OBFUSCATED, ChatFormatting.STRIKETHROUGH, ChatFormatting.UNDERLINE, ChatFormatting.ITALIC, ChatFormatting.BOLD, ChatFormatting.RED));
      out.accept(Component.literal("x").withStyle(ChatFormatting.RED, ChatFormatting.ITALIC, ChatFormatting.RESET, ChatFormatting.BOLD));
      out.accept(Component.literal("x").withStyle(ChatFormatting.RED, ChatFormatting.ITALIC).withStyle(ChatFormatting.RESET).withStyle(ChatFormatting.BOLD));
      out.accept(Component.literal("a").append(Component.literal("b").append(Component.literal("c"))).append(Component.literal("d").withStyle(ChatFormatting.GOLD)));
      out.accept(Component.translatable("demo.noargs", new Object[0]));
      out.accept(Component.translatableEscape("demo.esc", new Object[] { null }));
      out.accept(Component.translatableEscape("demo.esc", Character.valueOf('Z')));
      out.accept(Component.translatableEscape("demo.esc", new CustomLabel()));
      out.accept(Component.translatableEscape("demo.esc", new Pair(3, "hi")));
      ArrayList<String> list = new ArrayList<>();
      list.add("a");
      list.add("b");
      out.accept(Component.translatableEscape("demo.esc", list));
      out.accept(Component.translatableEscape("demo.esc", ChatFormatting.RED));
      out.accept(Component.translatableEscape("demo.esc", Component.literal("lit").withStyle(ChatFormatting.GOLD)));
      out.accept(Component.translatableEscape("demo.esc", TooltipFlag.NORMAL));
    }
  }

  static final class BlockOverride extends BlockItem {
    BlockOverride(net.minecraft.world.level.block.Block b, Properties p) { super(b, p); }
    @Override
    public void appendHoverText(ItemStack s, TooltipContext c, TooltipDisplay d, Consumer<Component> out, TooltipFlag f) {
      super.appendHoverText(s, c, d, out, f);
      out.accept(Component.translatable("demo.block", 7));
    }
  }

  static final class PlainItem extends Item {
    PlainItem(Properties p) { super(p); }
  }

  static final class PlainBlockItem extends BlockItem {
    PlainBlockItem(net.minecraft.world.level.block.Block b, Properties p) { super(b, p); }
  }

  static final class LinkedSubItem extends BaseTooltipItem {
    LinkedSubItem(Properties p) { super(p); }
  }

  static final class UnlinkedSubItem extends UnlinkedBaseItem {
    UnlinkedSubItem(Properties p) { super(p); }
  }

  static final class ConsumerChecksItem extends Item {
    ConsumerChecksItem(Properties p) { super(p); }
    @Override
    public void appendHoverText(ItemStack s, TooltipContext c, TooltipDisplay d, Consumer<Component> out, TooltipFlag f) {
      Object o = out;
      String verdict = ""
          + (o instanceof Consumer) + ","
          + (o instanceof Object) + ","
          + (o instanceof List) + ","
          + (o instanceof ArrayList) + ","
          + (o instanceof String) + ","
          + (o instanceof Runnable);
      out.accept(Component.literal(verdict));
    }
  }

  static final class ConsumerObjectMethodItem extends Item {
    final int which;
    ConsumerObjectMethodItem(Properties p, int which) { super(p); this.which = which; }
    @Override
    public void appendHoverText(ItemStack s, TooltipContext c, TooltipDisplay d, Consumer<Component> out, TooltipFlag f) {
      Object o = out;
      switch (which) {
        case 0 -> o.toString();
        case 1 -> o.hashCode();
        case 2 -> o.equals(o);
        case 3 -> o.equals(new ArrayList<>());
        case 4 -> o.getClass();
      }
    }
  }

  static final class HandleChecksItem extends Item {
    HandleChecksItem(Properties p) { super(p); }
    @Override
    public void appendHoverText(ItemStack s, TooltipContext c, TooltipDisplay d, Consumer<Component> out, TooltipFlag f) {
      Object co = c;
      Object doObj = d;
      Object fo = f;
      String verdict = ""
          + (co instanceof TooltipContext) + "," + (co instanceof String) + ","
          + (doObj instanceof TooltipDisplay) + "," + (doObj instanceof String) + ","
          + (fo instanceof TooltipFlag) + "," + (fo instanceof String);
      out.accept(Component.literal(verdict));
    }
  }

  static final class HandleObjectMethodItem extends Item {
    final int target;
    final int which;
    HandleObjectMethodItem(Properties p, int target, int which) { super(p); this.target = target; this.which = which; }
    @Override
    public void appendHoverText(ItemStack s, TooltipContext c, TooltipDisplay d, Consumer<Component> out, TooltipFlag f) {
      Object o = switch (target) {
        case 0 -> c;
        case 1 -> d;
        default -> f;
      };
      switch (which) {
        case 0 -> o.toString();
        case 1 -> o.hashCode();
        case 2 -> o.equals(o);
        case 3 -> o.getClass();
      }
    }
  }

  static final class FlagReaderItem extends Item {
    FlagReaderItem(Properties p) { super(p); }
    @Override
    public void appendHoverText(ItemStack s, TooltipContext c, TooltipDisplay d, Consumer<Component> out, TooltipFlag f) {
      TooltipFlag.Default def = (TooltipFlag.Default) f;
      out.accept(Component.literal("adv=" + def.advanced() + ",cre=" + def.creative() + ",iAdv=" + f.isAdvanced() + ",iCre=" + f.isCreative()));
    }
  }

  static final class FlagDefaultOracleItem extends Item {
    FlagDefaultOracleItem(Properties p) { super(p); }
    @Override
    public void appendHoverText(ItemStack s, TooltipContext c, TooltipDisplay d, Consumer<Component> out, TooltipFlag f) {
      TooltipFlag.Default normal = TooltipFlag.NORMAL;
      TooltipFlag.Default advanced = TooltipFlag.ADVANCED;
      TooltipFlag.Default normalCreative = normal.asCreative();
      TooltipFlag.Default advancedCreative = advanced.asCreative();
      out.accept(Component.literal(normal + "|" + advanced + "|" + normalCreative + "|" + advancedCreative));
      out.accept(Component.literal(normal.hashCode() + "," + advanced.hashCode() + "," + normalCreative.hashCode() + "," + advancedCreative.hashCode()));
      out.accept(Component.literal(normal.advanced() + "," + normal.creative() + "," + normal.isAdvanced() + "," + normal.isCreative() + "|" + advancedCreative.advanced() + "," + advancedCreative.creative() + "," + advancedCreative.isAdvanced() + "," + advancedCreative.isCreative()));
      out.accept(Component.literal((normal == TooltipFlag.NORMAL) + "," + normal.equals(TooltipFlag.NORMAL) + "," + normal.equals(advanced) + "," + normalCreative.equals(normal.asCreative()) + "," + (normalCreative == normal.asCreative()) + "," + normal.equals("not-a-flag") + "," + normal.equals(null)));
      Object flagObj = normalCreative;
      out.accept(Component.literal(flagObj.getClass().getName() + "," + (flagObj instanceof TooltipFlag.Default) + "," + (flagObj instanceof TooltipFlag) + "," + (flagObj instanceof Record) + "," + (flagObj instanceof String)));
    }
  }

  static final class EmptyContextOracleItem extends Item {
    EmptyContextOracleItem(Properties p) { super(p); }
    @Override
    public void appendHoverText(ItemStack s, TooltipContext c, TooltipDisplay d, Consumer<Component> out, TooltipFlag f) {
      TooltipContext empty = TooltipContext.EMPTY;
      out.accept(Component.literal(
          empty.tickRate() + ","
              + (empty.registries() == null) + ","
              + empty.isPeaceful() + ","
              + (empty.mapData(null) == null) + ","
              + (TooltipContext.of((Level) null) == empty) + ","
              + empty.equals(TooltipContext.EMPTY) + ","
              + empty.equals(null)));
    }
  }

  static final class EmptyContextObjectMethodItem extends Item {
    final int which;
    EmptyContextObjectMethodItem(Properties p, int which) { super(p); this.which = which; }
    @Override
    public void appendHoverText(ItemStack s, TooltipContext c, TooltipDisplay d, Consumer<Component> out, TooltipFlag f) {
      Object o = TooltipContext.EMPTY;
      switch (which) {
        case 0 -> o.toString();
        case 1 -> o.hashCode();
        case 2 -> o.getClass();
      }
    }
  }

  static final class TickRateReaderItem extends Item {
    TickRateReaderItem(Properties p) { super(p); }
    @Override
    public void appendHoverText(ItemStack s, TooltipContext c, TooltipDisplay d, Consumer<Component> out, TooltipFlag f) {
      out.accept(Component.literal("rate=" + c.tickRate()));
    }
  }

  static final class ContextMethodCallerItem extends Item {
    final int which;
    ContextMethodCallerItem(Properties p, int which) { super(p); this.which = which; }
    @Override
    public void appendHoverText(ItemStack s, TooltipContext c, TooltipDisplay d, Consumer<Component> out, TooltipFlag f) {
      switch (which) {
        case 0 -> c.level();
        case 1 -> c.player();
        case 2 -> c.registries();
        case 3 -> c.isPeaceful();
        case 4 -> d.hideTooltip();
      }
    }
  }

  static final class BadArgItem extends Item {
    final int which;
    BadArgItem(Properties p, int which) { super(p); this.which = which; }
    @Override
    public void appendHoverText(ItemStack s, TooltipContext c, TooltipDisplay d, Consumer<Component> out, TooltipFlag f) {
      MutableComponent line = switch (which) {
        case 0 -> Component.translatable("k", new Object[] { null });
        case 1 -> Component.translatable("k", Character.valueOf('c'));
        case 2 -> Component.translatable("k", new Object());
        case 3 -> Component.translatable("k", new ArrayList<>());
        default -> Component.translatable("k", ChatFormatting.RED);
      };
      // Construction preserves all arguments; styling, copying, and self-equality succeed without counting.
      MutableComponent styled = line.copy().withStyle(ChatFormatting.GRAY);
      MutableComponent plain = styled.plainCopy();
      if (!styled.equals(styled) || plain.equals("not-a-component")) {
        throw new AssertionError("unexpected equality");
      }
      out.accept(styled);
    }
  }

  static final class BadArgGetStringItem extends Item {
    final int which;
    BadArgGetStringItem(Properties p, int which) { super(p); this.which = which; }
    @Override
    public void appendHoverText(ItemStack s, TooltipContext c, TooltipDisplay d, Consumer<Component> out, TooltipFlag f) {
      Component line = Component.translatable("k", new Object());
      switch (which) {
        case 0 -> line.getString();
        case 1 -> line.toString();
        case 2 -> line.hashCode();
      }
    }
  }

  static final class EscapeEdgeItem extends Item {
    final int which;
    EscapeEdgeItem(Properties p, int which) { super(p); this.which = which; }
    @Override
    public void appendHoverText(ItemStack s, TooltipContext c, TooltipDisplay d, Consumer<Component> out, TooltipFlag f) {
      switch (which) {
        case 0 -> {
          Object[] arr = new Object[] { new CustomLabel(), 7 };
          Component line = Component.translatableEscape("demo.arr", arr);
          if (!(arr[0] instanceof String) || !arr[0].equals("custom-str")) {
            throw new AssertionError("expected in-place escape: " + arr[0]);
          }
          out.accept(line);
        }
        case 1 -> out.accept(Component.translatableEscape("demo.nullstr", new NullToString()));
        case 2 -> out.accept(Component.translatableEscape("demo.obj", new Object()));
        case 3 -> out.accept(Component.translatableEscape("demo.plain", new PlainObj()));
        default -> out.accept(Component.translatableEscape("demo.ctx", c));
      }
    }
  }
}"#;

fn build_probe_fixture() -> javac_fixture::Fixture {
    let mut sources = TOOLTIP_STUBS.to_vec();
    sources.push(("lib/BaseTooltipItem.java", LIB_BASE_SOURCE));
    sources.push(("demo/TooltipProbe.java", DEMO_TOOLTIP_SOURCE));
    javac_fixture::Fixture::build_with_lib("item-tooltip-probe", &sources, &[])
}

fn parse_java25_captures() -> Vec<(String, String)> {
    JAVA25_CAPTURES
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let (name, json) = line
                .split_once('=')
                .unwrap_or_else(|| panic!("malformed capture line: {line}"));
            (name.to_owned(), json.to_owned())
        })
        .collect()
}

fn make_item(jar: &NativeJar, host: &mut StdHost, method: &str) -> Value {
    jar.invoke_method(
        "demo/TooltipProbe",
        method,
        "()Lnet/minecraft/world/item/Item;",
        None,
        host,
    )
    .unwrap()
}

#[test]
fn runes_altar_item_appends_its_translatable_styled_line() {
    let Some(path) = std::env::var_os("RUNES_JAR").map(PathBuf::from) else {
        eprintln!("RUNES_JAR is unset; real Runes test skipped");
        return;
    };
    if !path.exists() {
        eprintln!("RUNES_JAR does not exist ({path:?}); real Runes test skipped");
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
            tooltip_stack(),
            tooltip_context(),
            tooltip_display(),
            tooltip_flag(false, false),
            &mut host,
            &ExecutionBudget::UNBOUNDED,
        )
        .unwrap();
    assert_eq!(lines.len(), 1);
    let captures: BTreeMap<String, String> = parse_java25_captures().into_iter().collect();
    assert_eq!(
        host.component_json(&lines[0]).as_deref(),
        Some(captures["translatable_styled"].as_str())
    );
    assert!(
        host.unknown_host_calls().is_empty(),
        "{:?}",
        host.unknown_host_calls()
    );
}

#[test]
fn component_json_matches_real_java25_codec_captures() {
    let fixture = build_probe_fixture();
    let all_captures = parse_java25_captures();
    let component_captures: Vec<(String, String)> = all_captures
        .iter()
        .filter(|(name, _)| !name.starts_with("flag_default_") && !name.starts_with("ctx_empty"))
        .cloned()
        .collect();
    let mut host = StdHost::new();
    let item = make_item(&fixture.jar, &mut host, "codecCases");
    let lines = fixture
        .jar
        .run_item_tooltip(
            &item,
            tooltip_stack(),
            tooltip_context(),
            tooltip_display(),
            tooltip_flag(false, false),
            &mut host,
            &ExecutionBudget::UNBOUNDED,
        )
        .unwrap();
    assert_eq!(
        lines.len(),
        component_captures.len(),
        "codec case count must match java25.txt"
    );
    for ((name, expected), line) in component_captures.iter().zip(&lines) {
        let actual = host
            .component_json(line)
            .unwrap_or_else(|| panic!("{name}: component_json returned None"));
        assert_eq!(actual, *expected, "mismatch for capture case `{name}`");
    }
    let block_item = make_item(&fixture.jar, &mut host, "blockItem");
    let block_lines = fixture
        .jar
        .run_item_tooltip(
            &block_item,
            tooltip_stack(),
            tooltip_context(),
            tooltip_display(),
            tooltip_flag(false, false),
            &mut host,
            &ExecutionBudget::UNBOUNDED,
        )
        .unwrap();
    assert_eq!(block_lines.len(), 1);
    let capture_map: BTreeMap<_, _> = all_captures.into_iter().collect();
    assert_eq!(
        host.component_json(&block_lines[0]).as_deref(),
        Some(capture_map["translatable_int_arg"].as_str())
    );
    assert!(
        host.unknown_host_calls().is_empty(),
        "{:?}",
        host.unknown_host_calls()
    );
}

#[test]
fn unsupported_translation_arguments_are_preserved_at_construction_and_refused_on_serialization() {
    let fixture = build_probe_fixture();
    for (which, label) in [
        (0, "null"),
        (1, "Character"),
        (2, "Object"),
        (3, "ArrayList"),
        (4, "ChatFormatting"),
    ] {
        let mut host = StdHost::new();
        let item = fixture
            .jar
            .invoke_method_with_args(
                "demo/TooltipProbe",
                "badArgItem",
                "(I)Lnet/minecraft/world/item/Item;",
                None,
                &[Value::Int(which)],
                &mut host,
            )
            .unwrap();
        let lines = fixture
            .jar
            .run_item_tooltip(
                &item,
                tooltip_stack(),
                tooltip_context(),
                tooltip_display(),
                tooltip_flag(false, false),
                &mut host,
                &ExecutionBudget::UNBOUNDED,
            )
            .unwrap();
        // TranslatableContents accepts any Object[] at construction: keeping,
        // styling, copying, and passing the component must not count.
        assert!(
            host.unknown_host_calls().is_empty(),
            "{label}: construction/manipulation must not count, got {:?}",
            host.unknown_host_calls()
        );
        assert_eq!(lines.len(), 1, "{label}");
        assert_eq!(
            host.component_json(&lines[0]),
            None,
            "{label}: component_json must refuse disallowed translation arguments"
        );
    }

    // Reading/serializing a component with a disallowed argument counts.
    for (which, label) in [(0, "getString"), (1, "toString"), (2, "hashCode")] {
        let mut host = StdHost::new();
        let item = fixture
            .jar
            .invoke_method_with_args(
                "demo/TooltipProbe",
                "badArgGetString",
                "(I)Lnet/minecraft/world/item/Item;",
                None,
                &[Value::Int(which)],
                &mut host,
            )
            .unwrap();
        fixture
            .jar
            .run_item_tooltip(
                &item,
                tooltip_stack(),
                tooltip_context(),
                tooltip_display(),
                tooltip_flag(false, false),
                &mut host,
                &ExecutionBudget::UNBOUNDED,
            )
            .unwrap();
        assert!(
            !host.unknown_host_calls().is_empty(),
            "badArg.{label} must be counted"
        );
    }

    // translatableEscape edge cases: in-place array mutation, custom toString() returning null,
    // and unknown Object.toString() counted + refused on component_json.
    {
        let mut host = StdHost::new();
        let item = fixture
            .jar
            .invoke_method_with_args(
                "demo/TooltipProbe",
                "escapeEdgeItem",
                "(I)Lnet/minecraft/world/item/Item;",
                None,
                &[Value::Int(0)],
                &mut host,
            )
            .unwrap();
        let lines = fixture
            .jar
            .run_item_tooltip(
                &item,
                tooltip_stack(),
                tooltip_context(),
                tooltip_display(),
                tooltip_flag(false, false),
                &mut host,
                &ExecutionBudget::UNBOUNDED,
            )
            .unwrap();
        assert_eq!(
            host.component_json(&lines[0]).as_deref(),
            Some(r#"{"translate":"demo.arr","with":["custom-str",7]}"#)
        );
        assert!(
            host.unknown_host_calls().is_empty(),
            "{:?}",
            host.unknown_host_calls()
        );
    }
    // Custom toString() returning null: String.valueOf returns null (not counted), codec refuses.
    {
        let mut host = StdHost::new();
        let item = fixture
            .jar
            .invoke_method_with_args(
                "demo/TooltipProbe",
                "escapeEdgeItem",
                "(I)Lnet/minecraft/world/item/Item;",
                None,
                &[Value::Int(1)],
                &mut host,
            )
            .unwrap();
        let lines = fixture
            .jar
            .run_item_tooltip(
                &item,
                tooltip_stack(),
                tooltip_context(),
                tooltip_display(),
                tooltip_flag(false, false),
                &mut host,
                &ExecutionBudget::UNBOUNDED,
            )
            .unwrap();
        assert!(
            host.unknown_host_calls().is_empty(),
            "null toString must not count at construction: {:?}",
            host.unknown_host_calls()
        );
        assert_eq!(host.component_json(&lines[0]), None);
    }
    // Objects whose toString() is unknown (Object, plain jar class, TooltipContext): counted
    // during translatableEscape and refused at component_json.
    for (which, label) in [(2, "Object"), (3, "PlainObj"), (4, "TooltipContext")] {
        let mut host = StdHost::new();
        let item = fixture
            .jar
            .invoke_method_with_args(
                "demo/TooltipProbe",
                "escapeEdgeItem",
                "(I)Lnet/minecraft/world/item/Item;",
                None,
                &[Value::Int(which)],
                &mut host,
            )
            .unwrap();
        let lines = fixture
            .jar
            .run_item_tooltip(
                &item,
                tooltip_stack(),
                tooltip_context(),
                tooltip_display(),
                tooltip_flag(false, false),
                &mut host,
                &ExecutionBudget::UNBOUNDED,
            )
            .unwrap();
        assert!(
            !host.unknown_host_calls().is_empty(),
            "translatableEscape({label}) must count unknown toString"
        );
        assert_eq!(
            host.component_json(&lines[0]),
            None,
            "translatableEscape({label}) must not serialize a placeholder string"
        );
    }
}

#[test]
fn tooltip_consumer_is_an_opaque_handle_with_exact_instanceof_and_counted_object_methods() {
    let fixture = build_probe_fixture();
    let mut host = StdHost::new();
    let item = make_item(&fixture.jar, &mut host, "consumerChecks");
    let lines = fixture
        .jar
        .run_item_tooltip(
            &item,
            tooltip_stack(),
            tooltip_context(),
            tooltip_display(),
            tooltip_flag(false, false),
            &mut host,
            &ExecutionBudget::UNBOUNDED,
        )
        .unwrap();
    assert_eq!(lines.len(), 1);
    assert_eq!(
        host.component_json(&lines[0]).as_deref(),
        Some(r#""true,true,false,false,false,false""#)
    );
    assert!(
        host.unknown_host_calls().is_empty(),
        "{:?}",
        host.unknown_host_calls()
    );

    for (which, label) in [
        (0, "toString"),
        (1, "hashCode"),
        (2, "equals(self)"),
        (3, "equals(ArrayList)"),
        (4, "getClass"),
    ] {
        let mut host = StdHost::new();
        let item = fixture
            .jar
            .invoke_method_with_args(
                "demo/TooltipProbe",
                "consumerObjectMethod",
                "(I)Lnet/minecraft/world/item/Item;",
                None,
                &[Value::Int(which)],
                &mut host,
            )
            .unwrap();
        fixture
            .jar
            .run_item_tooltip(
                &item,
                tooltip_stack(),
                tooltip_context(),
                tooltip_display(),
                tooltip_flag(false, false),
                &mut host,
                &ExecutionBudget::UNBOUNDED,
            )
            .unwrap();
        assert!(
            !host.unknown_host_calls().is_empty(),
            "Consumer.{label} must be counted"
        );
    }
}

#[test]
fn override_lookup_resolves_linked_superclasses_checks_flags_and_counts_unresolvable() {
    let fixture = build_probe_fixture();
    let lib = fixture.lib.as_ref().expect("lib jar");

    // 1. Plain Item / BlockItem subclasses with no override: zero lines, zero counted calls.
    for method in ["plainItem", "plainBlockItem"] {
        let mut host = StdHost::new();
        let item = make_item(&fixture.jar, &mut host, method);
        let lines = fixture
            .jar
            .run_item_tooltip(
                &item,
                tooltip_stack(),
                tooltip_context(),
                tooltip_display(),
                tooltip_flag(false, false),
                &mut host,
                &ExecutionBudget::UNBOUNDED,
            )
            .unwrap();
        assert!(lines.is_empty(), "{method}");
        assert!(
            host.unknown_host_calls().is_empty(),
            "{method}: {:?}",
            host.unknown_host_calls()
        );
    }

    // 2. Override inherited from a linked sibling jar (`lib.BaseTooltipItem`).
    {
        let mut host = StdHost::new();
        host.link_jar("probe-lib", lib).unwrap();
        let item = make_item(&fixture.jar, &mut host, "linkedSubItem");
        let lines = fixture
            .jar
            .run_item_tooltip(
                &item,
                tooltip_stack(),
                tooltip_context(),
                tooltip_display(),
                tooltip_flag(false, false),
                &mut host,
                &ExecutionBudget::UNBOUNDED,
            )
            .unwrap();
        assert_eq!(lines.len(), 1);
        assert_eq!(
            host.component_json(&lines[0]).as_deref(),
            Some(r#"{"translate":"lib.base.line","color":"gray"}"#)
        );
        assert!(
            host.unknown_host_calls().is_empty(),
            "{:?}",
            host.unknown_host_calls()
        );
    }

    // 3. Private and static `appendHoverText` methods with the exact 5-arg descriptor
    //    do NOT override: skipped when walking up to vanilla BlockItem/Item or to a
    //    linked superclass that carries a real public virtual override.
    {
        // Compile against an `Item`/`BaseTooltipItem` stub without `appendHoverText` so
        // javac emits genuine bytecode with `ACC_PRIVATE` / `ACC_STATIC` on that descriptor.
        let mut non_override_stubs: Vec<(&str, &str)> = TOOLTIP_STUBS
            .iter()
            .copied()
            .filter(|(path, _)| *path != "net/minecraft/world/item/Item.java")
            .collect();
        non_override_stubs.push((
            "net/minecraft/world/item/Item.java",
            "package net.minecraft.world.item; public class Item { public static class Properties {} public interface TooltipContext {} public Item(Properties p) {} }",
        ));
        non_override_stubs.push((
            "lib/BaseTooltipItem.java",
            "package lib; import net.minecraft.world.item.Item; public class BaseTooltipItem extends Item { public BaseTooltipItem(Properties p) { super(p); } }",
        ));
        non_override_stubs.push((
            "demo/NonOverrides.java",
            r#"package demo;
import java.util.function.Consumer;
import lib.BaseTooltipItem;
import net.minecraft.network.chat.Component;
import net.minecraft.world.item.*;
import net.minecraft.world.item.component.TooltipDisplay;
public final class NonOverrides {
  public static Item privateOverBlock() { return new PrivateOverBlock(null, new Item.Properties()); }
  public static Item staticOverItem() { return new StaticOverItem(new Item.Properties()); }
  public static Item privateOverBase() { return new PrivateOverBase(new Item.Properties()); }
  public static Item staticOverBase() { return new StaticOverBase(new Item.Properties()); }
  static final class PrivateOverBlock extends BlockItem {
    PrivateOverBlock(net.minecraft.world.level.block.Block b, Properties p) { super(b, p); }
    private void appendHoverText(ItemStack s, TooltipContext c, TooltipDisplay d, Consumer<Component> out, TooltipFlag f) {
      out.accept(Component.literal("private-must-not-run"));
    }
  }
  static final class StaticOverItem extends Item {
    StaticOverItem(Properties p) { super(p); }
    public static void appendHoverText(ItemStack s, TooltipContext c, TooltipDisplay d, Consumer<Component> out, TooltipFlag f) {
      out.accept(Component.literal("static-must-not-run"));
    }
  }
  static final class PrivateOverBase extends BaseTooltipItem {
    PrivateOverBase(Properties p) { super(p); }
    private void appendHoverText(ItemStack s, TooltipContext c, TooltipDisplay d, Consumer<Component> out, TooltipFlag f) {
      out.accept(Component.literal("private-must-not-run"));
    }
  }
  static final class StaticOverBase extends BaseTooltipItem {
    StaticOverBase(Properties p) { super(p); }
    public static void appendHoverText(ItemStack s, TooltipContext c, TooltipDisplay d, Consumer<Component> out, TooltipFlag f) {
      out.accept(Component.literal("static-must-not-run"));
    }
  }
}"#,
        ));
        let non_overrides =
            javac_fixture::Fixture::build("item-tooltip-non-overrides", &non_override_stubs);
        for method in ["privateOverBlock", "staticOverItem"] {
            let mut host = StdHost::new();
            let item = non_overrides
                .jar
                .invoke_method(
                    "demo/NonOverrides",
                    method,
                    "()Lnet/minecraft/world/item/Item;",
                    None,
                    &mut host,
                )
                .unwrap();
            let lines = non_overrides
                .jar
                .run_item_tooltip(
                    &item,
                    tooltip_stack(),
                    tooltip_context(),
                    tooltip_display(),
                    tooltip_flag(false, false),
                    &mut host,
                    &ExecutionBudget::UNBOUNDED,
                )
                .unwrap();
            assert!(lines.is_empty(), "{method} must not run non-override");
            assert!(
                host.unknown_host_calls().is_empty(),
                "{method}: {:?}",
                host.unknown_host_calls()
            );
        }
        // When extending the real linked `lib.BaseTooltipItem`, private/static methods on the
        // subclass are skipped and the linked superclass's public virtual override runs.
        for method in ["privateOverBase", "staticOverBase"] {
            let mut host = StdHost::new();
            host.link_jar("probe-lib", lib).unwrap();
            let item = non_overrides
                .jar
                .invoke_method(
                    "demo/NonOverrides",
                    method,
                    "()Lnet/minecraft/world/item/Item;",
                    None,
                    &mut host,
                )
                .unwrap();
            let lines = non_overrides
                .jar
                .run_item_tooltip(
                    &item,
                    tooltip_stack(),
                    tooltip_context(),
                    tooltip_display(),
                    tooltip_flag(false, false),
                    &mut host,
                    &ExecutionBudget::UNBOUNDED,
                )
                .unwrap();
            assert_eq!(lines.len(), 1, "{method}");
            assert_eq!(
                host.component_json(&lines[0]).as_deref(),
                Some(r#"{"translate":"lib.base.line","color":"gray"}"#),
                "{method}"
            );
            assert!(
                host.unknown_host_calls().is_empty(),
                "{method}: {:?}",
                host.unknown_host_calls()
            );
        }
    }

    // 4. Unresolvable external superclass (both unlinked `lib.BaseTooltipItem` and `ext.UnlinkedBaseItem`)
    //    must be COUNTED, never silently treated as "no override".
    for method in ["linkedSubItem", "unlinkedSubItem"] {
        let mut host = StdHost::new();
        let item = make_item(&fixture.jar, &mut host, method);
        let lines = fixture
            .jar
            .run_item_tooltip(
                &item,
                tooltip_stack(),
                tooltip_context(),
                tooltip_display(),
                tooltip_flag(false, false),
                &mut host,
                &ExecutionBudget::UNBOUNDED,
            )
            .unwrap();
        assert!(lines.is_empty(), "{method}");
        assert!(
            !host.unknown_host_calls().is_empty(),
            "{method}: unresolvable superclass must be counted"
        );
    }
}

#[test]
fn tooltip_handles_hide_string_semantics_honour_flag_and_frame_tick_rate() {
    let fixture = build_probe_fixture();

    // Dedicated handles: proper instanceof and not String.
    {
        let mut host = StdHost::new();
        let item = make_item(&fixture.jar, &mut host, "handleChecks");
        let lines = fixture
            .jar
            .run_item_tooltip(
                &item,
                tooltip_stack(),
                tooltip_context(),
                tooltip_display(),
                tooltip_flag(false, false),
                &mut host,
                &ExecutionBudget::UNBOUNDED,
            )
            .unwrap();
        assert_eq!(lines.len(), 1);
        assert_eq!(
            host.component_json(&lines[0]).as_deref(),
            Some(r#""true,false,true,false,true,false""#)
        );
        assert!(
            host.unknown_host_calls().is_empty(),
            "{:?}",
            host.unknown_host_calls()
        );
    }

    // Object methods (toString/hashCode/equals/getClass) are counted on opaque
    // TooltipContext, TooltipDisplay, and frame-backed opaque TooltipFlag handles.
    for (target, target_name, opaque_flag) in [
        (0, "TooltipContext", tooltip_flag(false, false)),
        (1, "TooltipDisplay", tooltip_flag(false, false)),
        (
            2,
            "OpaqueTooltipFlag",
            Value::Host(HostRef::new(HostKind::TooltipFlag, "frame")),
        ),
    ] {
        for (which, method_name) in [
            (0, "toString"),
            (1, "hashCode"),
            (2, "equals"),
            (3, "getClass"),
        ] {
            let mut host = StdHost::new();
            let item = fixture
                .jar
                .invoke_method_with_args(
                    "demo/TooltipProbe",
                    "handleObjectMethod",
                    "(II)Lnet/minecraft/world/item/Item;",
                    None,
                    &[Value::Int(target), Value::Int(which)],
                    &mut host,
                )
                .unwrap();
            fixture
                .jar
                .run_item_tooltip(
                    &item,
                    tooltip_stack(),
                    tooltip_context(),
                    tooltip_display(),
                    opaque_flag.clone(),
                    &mut host,
                    &ExecutionBudget::UNBOUNDED,
                )
                .unwrap();
            assert!(
                !host.unknown_host_calls().is_empty(),
                "{target_name}.{method_name} must be counted"
            );
        }
    }

    // TooltipFlag$Default record model matches the real JDK 25 + MC 26.3 oracle captures.
    let captures: BTreeMap<String, String> = parse_java25_captures().into_iter().collect();
    {
        let mut host = StdHost::new();
        let item = make_item(&fixture.jar, &mut host, "flagDefaultOracle");
        let lines = fixture
            .jar
            .run_item_tooltip(
                &item,
                tooltip_stack(),
                tooltip_context(),
                tooltip_display(),
                tooltip_flag(false, false),
                &mut host,
                &ExecutionBudget::UNBOUNDED,
            )
            .unwrap();
        let keys = [
            "flag_default_strings",
            "flag_default_hashes",
            "flag_default_accessors",
            "flag_default_equality",
            "flag_default_types",
        ];
        assert_eq!(lines.len(), keys.len());
        for (key, line) in keys.iter().zip(&lines) {
            let expected = serde_json::Value::String(captures[*key].clone()).to_string();
            assert_eq!(
                host.component_json(line).as_deref(),
                Some(expected.as_str()),
                "mismatch for oracle case `{key}`"
            );
        }
        assert!(
            host.unknown_host_calls().is_empty(),
            "{:?}",
            host.unknown_host_calls()
        );
    }

    // Item.TooltipContext.EMPTY matches the real JDK 25 + MC 26.3 oracle capture even
    // inside a client frame with a different tick rate, and when passed directly as context.
    {
        let mut host = StdHost::new();
        let item = make_item(&fixture.jar, &mut host, "emptyContextOracle");
        host.begin_client_frame(
            Arc::new(ClientSnapshot {
                tick_rate: Some(12.5),
                ..ClientSnapshot::default()
            }),
            &[],
        );
        let lines = fixture
            .jar
            .run_item_tooltip(
                &item,
                tooltip_stack(),
                tooltip_context(),
                tooltip_display(),
                tooltip_flag(false, false),
                &mut host,
                &ExecutionBudget::UNBOUNDED,
            )
            .unwrap();
        host.end_client_frame();
        assert_eq!(lines.len(), 1);
        let expected = serde_json::Value::String(captures["ctx_empty"].clone()).to_string();
        assert_eq!(
            host.component_json(&lines[0]).as_deref(),
            Some(expected.as_str())
        );
        assert!(
            host.unknown_host_calls().is_empty(),
            "{:?}",
            host.unknown_host_calls()
        );
    }
    // Passing tooltip_context_empty() directly to run_item_tooltip answers 20.0f even outside a frame.
    {
        let mut host = StdHost::new();
        let item = make_item(&fixture.jar, &mut host, "tickRateReader");
        let lines = fixture
            .jar
            .run_item_tooltip(
                &item,
                tooltip_stack(),
                tooltip_context_empty(),
                tooltip_display(),
                tooltip_flag(false, false),
                &mut host,
                &ExecutionBudget::UNBOUNDED,
            )
            .unwrap();
        assert_eq!(lines.len(), 1);
        assert_eq!(
            host.component_json(&lines[0]).as_deref(),
            Some(r#""rate=20.0""#)
        );
        assert!(
            host.unknown_host_calls().is_empty(),
            "{:?}",
            host.unknown_host_calls()
        );
    }
    // EMPTY anonymous class identity methods (toString/hashCode/getClass) stay counted.
    for (which, method_name) in [(0, "toString"), (1, "hashCode"), (2, "getClass")] {
        let mut host = StdHost::new();
        let item = fixture
            .jar
            .invoke_method_with_args(
                "demo/TooltipProbe",
                "emptyContextObjectMethod",
                "(I)Lnet/minecraft/world/item/Item;",
                None,
                &[Value::Int(which)],
                &mut host,
            )
            .unwrap();
        fixture
            .jar
            .run_item_tooltip(
                &item,
                tooltip_stack(),
                tooltip_context(),
                tooltip_display(),
                tooltip_flag(false, false),
                &mut host,
                &ExecutionBudget::UNBOUNDED,
            )
            .unwrap();
        assert!(
            !host.unknown_host_calls().is_empty(),
            "TooltipContext.EMPTY.{method_name} must be counted"
        );
    }

    // Supplied TooltipFlag$Default is honoured for all (advanced, creative) combinations,
    // both outside a frame and when overriding a client frame's default flags.
    for (adv, cre) in [(false, false), (true, false), (false, true), (true, true)] {
        let mut host = StdHost::new();
        let item = make_item(&fixture.jar, &mut host, "flagReader");
        host.begin_client_frame(
            Arc::new(ClientSnapshot {
                advanced_tooltips: !adv,
                creative_tooltips: !cre,
                ..ClientSnapshot::default()
            }),
            &[],
        );
        let lines = fixture
            .jar
            .run_item_tooltip(
                &item,
                tooltip_stack(),
                tooltip_context(),
                tooltip_display(),
                tooltip_flag(adv, cre),
                &mut host,
                &ExecutionBudget::UNBOUNDED,
            )
            .unwrap();
        host.end_client_frame();
        assert_eq!(lines.len(), 1);
        assert_eq!(
            host.component_json(&lines[0]).as_deref(),
            Some(format!(r#""adv={adv},cre={cre},iAdv={adv},iCre={cre}""#).as_str())
        );
        assert!(
            host.unknown_host_calls().is_empty(),
            "({adv},{cre}): {:?}",
            host.unknown_host_calls()
        );
    }

    // TooltipContext.tickRate(): returns frame's tick_rate when present, counts otherwise.
    {
        let mut host = StdHost::new();
        let item = make_item(&fixture.jar, &mut host, "tickRateReader");
        let frame = Arc::new(ClientSnapshot {
            tick_rate: Some(12.5),
            ..ClientSnapshot::default()
        });
        host.begin_client_frame(frame, &[]);
        let lines = fixture
            .jar
            .run_item_tooltip(
                &item,
                tooltip_stack(),
                tooltip_context(),
                tooltip_display(),
                tooltip_flag(false, false),
                &mut host,
                &ExecutionBudget::UNBOUNDED,
            )
            .unwrap();
        host.end_client_frame();
        assert_eq!(lines.len(), 1);
        assert_eq!(
            host.component_json(&lines[0]).as_deref(),
            Some(r#""rate=12.5""#)
        );
        assert!(
            host.unknown_host_calls().is_empty(),
            "{:?}",
            host.unknown_host_calls()
        );
    }
    // Frame without tick_rate -> counted.
    {
        let mut host = StdHost::new();
        let item = make_item(&fixture.jar, &mut host, "tickRateReader");
        host.begin_client_frame(Arc::new(ClientSnapshot::default()), &[]);
        fixture
            .jar
            .run_item_tooltip(
                &item,
                tooltip_stack(),
                tooltip_context(),
                tooltip_display(),
                tooltip_flag(false, false),
                &mut host,
                &ExecutionBudget::UNBOUNDED,
            )
            .unwrap();
        host.end_client_frame();
        assert!(
            !host.unknown_host_calls().is_empty(),
            "tickRate() with no frame tick_rate must be counted"
        );
    }
    // Outside a frame -> counted.
    {
        let mut host = StdHost::new();
        let item = make_item(&fixture.jar, &mut host, "tickRateReader");
        fixture
            .jar
            .run_item_tooltip(
                &item,
                tooltip_stack(),
                tooltip_context(),
                tooltip_display(),
                tooltip_flag(false, false),
                &mut host,
                &ExecutionBudget::UNBOUNDED,
            )
            .unwrap();
        assert!(
            !host.unknown_host_calls().is_empty(),
            "tickRate() outside a client frame must be counted"
        );
    }

    // Invented TooltipContext.level()/player(), plus registries()/isPeaceful()/TooltipDisplay.hideTooltip(),
    // are all counted even when a frame with level and player is active.
    for (which, label) in [
        (0, "TooltipContext.level"),
        (1, "TooltipContext.player"),
        (2, "TooltipContext.registries"),
        (3, "TooltipContext.isPeaceful"),
        (4, "TooltipDisplay.hideTooltip"),
    ] {
        let mut host = StdHost::new();
        let item = fixture
            .jar
            .invoke_method_with_args(
                "demo/TooltipProbe",
                "contextMethodCaller",
                "(I)Lnet/minecraft/world/item/Item;",
                None,
                &[Value::Int(which)],
                &mut host,
            )
            .unwrap();
        let frame = Arc::new(ClientSnapshot {
            level: Some(ClientLevel::default()),
            player: Some(ClientPlayer::default()),
            tick_rate: Some(20.0),
            ..ClientSnapshot::default()
        });
        host.begin_client_frame(frame, &[]);
        fixture
            .jar
            .run_item_tooltip(
                &item,
                tooltip_stack(),
                tooltip_context(),
                tooltip_display(),
                tooltip_flag(false, false),
                &mut host,
                &ExecutionBudget::UNBOUNDED,
            )
            .unwrap();
        host.end_client_frame();
        assert!(
            !host.unknown_host_calls().is_empty(),
            "{label} must be counted"
        );
    }
}
