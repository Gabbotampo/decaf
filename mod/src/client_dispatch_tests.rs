//! Contract and end-to-end tests for the Sunrise-facing client adapters:
//! - `Hooks::build_creative_mode_tab_contents` -> Fabric `CreativeModeTabEvents` listeners
//! - `Hooks::item_tooltip` -> Java `Item.appendHoverText` + Sunrise rich `Text` boundary

use std::cell::RefCell;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use mod_api::__private::game::BackendRefOf;
use mod_api::text::{TextContent, TextStyle};
use mod_api::{
    Backend, BackendRef, BuildCreativeModeTabContentsEvent, ClientHost, ClientSetupEvent,
    CreativeTabChange, CreativeTabEdit, CreativeTabEditError, CreativeTabList,
    CreativeTabPermissions, CreativeTabPlacement, CreativeTabVisibility, Dist, HookId, Hooks, Host,
    HostSide, Identifier, ItemKey, ItemStack, ItemTooltipEvent, Listener, ModEntry, Tag, Text,
    TextFormError, TooltipDisplay,
};
use zip::write::SimpleFileOptions;

use crate::HorizonJava;

static FIXTURE_BUILDS: AtomicU64 = AtomicU64::new(0);

/// Build a Fabric jar from Java sources with `javac --release 21`, packing only `demo/` classes
/// plus `fabric.mod.json` so every `net/minecraft/*` and `net/fabricmc/*` call crosses the
/// native runtime boundary.
fn build_javac_jar(test: &str, mod_json: &str, sources: &[(&str, &str)]) -> Vec<u8> {
    struct Cleanup(PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let build = FIXTURE_BUILDS.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "horizon-java-mod-client-{test}-{}-{unique}-{build}",
        std::process::id()
    ));
    let _cleanup = Cleanup(root.clone());
    let classes = root.join("classes");

    let javac = std::env::var_os("JAVA_HOME")
        .map(|home| PathBuf::from(home).join("bin/javac"))
        .unwrap_or_else(|| PathBuf::from("javac"));
    let mut cmd = Command::new(javac);
    cmd.args(["--release", "21", "-d"]).arg(&classes);
    for (path, source) in sources {
        let file = root.join("src").join(path);
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(&file, source).unwrap();
        cmd.arg(file);
    }
    let output = cmd.output().expect("invoke javac");
    assert!(
        output.status.success(),
        "javac failed for {test}:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    fn collect_files(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries {
            let path = entry.unwrap().path();
            if path.is_dir() {
                collect_files(&path, out);
            } else {
                out.push(path);
            }
        }
    }

    let mut class_files = Vec::new();
    collect_files(&classes.join("demo"), &mut class_files);
    class_files.sort();

    let mut bytes = std::io::Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut bytes);
        let opts = SimpleFileOptions::default();
        zip.start_file("fabric.mod.json", opts).unwrap();
        zip.write_all(mod_json.as_bytes()).unwrap();
        for class in class_files {
            let rel = class
                .strip_prefix(&classes)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            zip.start_file(rel, opts).unwrap();
            zip.write_all(&fs::read(class).unwrap()).unwrap();
        }
        zip.finish().unwrap();
    }
    bytes.into_inner()
}

/// Forwarding mod entry that preserves `HorizonJava`'s exact `#[mod_api::sunrise_mod]` hook
/// subscriptions while allowing the test to inspect `HorizonJava`'s client state between calls.
struct SharedHorizonJava(Arc<HorizonJava>);

impl Hooks for SharedHorizonJava {
    fn subscribed(&self) -> &'static [Option<HookId>] {
        self.0.subscribed()
    }
    fn listeners(&self) -> &'static [Listener] {
        self.0.listeners()
    }
    fn client_setup(&self, host: &mut dyn ClientHost, event: &mut ClientSetupEvent) {
        self.0.client_setup(host, event);
    }
    fn build_creative_mode_tab_contents(
        &self,
        host: HostSide<'_>,
        event: &mut BuildCreativeModeTabContentsEvent<'_>,
    ) {
        self.0.build_creative_mode_tab_contents(host, event);
    }
    fn item_tooltip(&self, host: &dyn ClientHost, event: &mut ItemTooltipEvent<'_>) {
        self.0.item_tooltip(host, event);
    }
}

#[derive(Default)]
struct CreativeTabEditorState {
    active: Option<CreativeTabEdit>,
    parent: Vec<ItemStack>,
    search: Vec<ItemStack>,
}

/// Sunrise client host test double implementing the real creative-tab editor (`ItemsBackend`),
/// item registry keys (`ItemsHost` / `KeysBackend`), `TextHost::text_from_json` (matching
/// Sunrise's `native_text_json` component codec), and `ConfigHost::game_version`.
struct TestClientHost {
    fake: mod_api::testing::FakeHost,
    editor: RefCell<CreativeTabEditorState>,
}

impl TestClientHost {
    fn new(item_ids: &[&str]) -> Self {
        let mut fake = mod_api::testing::FakeHost::new();
        let parsed: Vec<Identifier> = item_ids
            .iter()
            .map(|id| Identifier::parse(id).unwrap_or_else(|e| panic!("bad id {id}: {e:?}")))
            .collect();
        fake.registries
            .resolve_items(parsed)
            .expect("resolve test items");
        Self {
            fake,
            editor: RefCell::new(CreativeTabEditorState::default()),
        }
    }

    fn stack(&self, id: &str) -> ItemStack {
        let key = mod_api::prelude::ItemsHost::item_key(self, &Identifier::parse(id).unwrap())
            .unwrap_or_else(|| panic!("unknown test item {id}"));
        ItemStack::of(key)
    }

    fn begin_tab(&self, id: u64, parent: &[&str], search: &[&str]) -> CreativeTabEdit {
        let edit = mod_api::__private::creative_tab_edit(id);
        let mut editor = self.editor.borrow_mut();
        editor.active = Some(edit);
        editor.parent = parent.iter().map(|s| self.stack(s)).collect();
        editor.search = search.iter().map(|s| self.stack(s)).collect();
        edit
    }

    fn tab_entries(&self, list: CreativeTabList) -> Vec<(String, u32)> {
        let editor = self.editor.borrow();
        let entries = match list {
            CreativeTabList::Parent => &editor.parent,
            CreativeTabList::Search => &editor.search,
        };
        entries
            .iter()
            .map(|stack| (stack.item().id(self), stack.count()))
            .collect()
    }
}

impl Host for TestClientHost {
    fn backend(&self) -> Option<BackendRef<'_>> {
        Some(BackendRef::of(self))
    }
}
mod_api::impl_host_areas!(TestClientHost, except(config, items, text));

impl ClientHost for TestClientHost {}
mod_api::impl_client_host_areas!(TestClientHost);

impl Backend for TestClientHost {}
mod_api::impl_backend_areas!(TestClientHost, except(items, keys));

impl mod_api::prelude::ConfigHost for TestClientHost {
    fn dist(&self) -> Dist {
        Dist::Client
    }
    fn game_version(&self) -> &'static str {
        "26.3"
    }
    fn on(&self, _mod_id: &str) -> bool {
        true
    }
}

impl mod_api::prelude::ItemsHost for TestClientHost {
    fn item_key(&self, id: &Identifier) -> Option<ItemKey> {
        mod_api::prelude::ItemsHost::item_key(&self.fake, id)
    }
}

impl mod_api::prelude::KeysBackend for TestClientHost {
    fn item_key_id(&self, key: ItemKey) -> String {
        mod_api::prelude::KeysBackend::item_key_id(&self.fake, key)
    }
}

impl mod_api::prelude::ItemsBackend for TestClientHost {
    fn creative_tab_entries(
        &self,
        edit: CreativeTabEdit,
        list: CreativeTabList,
    ) -> Result<Vec<ItemStack>, CreativeTabEditError> {
        let editor = self.editor.borrow();
        if editor.active != Some(edit) {
            return Err(CreativeTabEditError::Stale);
        }
        Ok(match list {
            CreativeTabList::Parent => editor.parent.clone(),
            CreativeTabList::Search => editor.search.clone(),
        })
    }

    fn creative_tab_change(
        &self,
        edit: CreativeTabEdit,
        change: CreativeTabChange,
    ) -> Result<(), CreativeTabEditError> {
        let mut editor = self.editor.borrow_mut();
        if editor.active != Some(edit) {
            return Err(CreativeTabEditError::Stale);
        }
        match change {
            CreativeTabChange::Insert {
                placement,
                stack,
                visibility,
            } => {
                if stack.count() != 1 {
                    return Err(CreativeTabEditError::Count);
                }
                let lists: &[CreativeTabList] = match visibility {
                    CreativeTabVisibility::Parent => &[CreativeTabList::Parent],
                    CreativeTabVisibility::Search => &[CreativeTabList::Search],
                    CreativeTabVisibility::Both => {
                        &[CreativeTabList::Parent, CreativeTabList::Search]
                    }
                };
                // Validate every target list before mutating, matching Sunrise's editor.
                let mut indices = Vec::with_capacity(lists.len());
                for &list in lists {
                    let entries = match list {
                        CreativeTabList::Parent => &editor.parent,
                        CreativeTabList::Search => &editor.search,
                    };
                    if entries
                        .iter()
                        .any(|existing| existing.is_stackable_with(&stack))
                    {
                        return Err(CreativeTabEditError::Duplicate(list));
                    }
                    let index = match &placement {
                        CreativeTabPlacement::First => 0,
                        CreativeTabPlacement::Last => entries.len(),
                        CreativeTabPlacement::Before(anchor) => entries
                            .iter()
                            .position(|existing| existing.is_stackable_with(anchor))
                            .ok_or(CreativeTabEditError::MissingAnchor(list))?,
                        CreativeTabPlacement::After(anchor) => entries
                            .iter()
                            .rposition(|existing| existing.is_stackable_with(anchor))
                            .map(|idx| idx + 1)
                            .ok_or(CreativeTabEditError::MissingAnchor(list))?,
                    };
                    indices.push((list, index));
                }
                for (list, index) in indices {
                    let entries = match list {
                        CreativeTabList::Parent => &mut editor.parent,
                        CreativeTabList::Search => &mut editor.search,
                    };
                    entries.insert(index, stack.clone());
                }
                Ok(())
            }
            CreativeTabChange::Remove { stack, visibility } => {
                if matches!(
                    visibility,
                    CreativeTabVisibility::Parent | CreativeTabVisibility::Both
                ) {
                    editor
                        .parent
                        .retain(|existing| !existing.is_stackable_with(&stack));
                }
                if matches!(
                    visibility,
                    CreativeTabVisibility::Search | CreativeTabVisibility::Both
                ) {
                    editor
                        .search
                        .retain(|existing| !existing.is_stackable_with(&stack));
                }
                Ok(())
            }
        }
    }
}

impl mod_api::prelude::TextHost for TestClientHost {
    fn text_from_json(&self, json: &str) -> Result<Option<Text>, TextFormError> {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(json) else {
            return Ok(None);
        };
        Ok(parse_json_component(&value, false))
    }
}

/// Parse vanilla `ComponentSerialization.CODEC` JSON into Sunrise [`Text`], matching Sunrise's
/// `native_text_json` rules (returns `None` on malformed or unsupported shapes).
fn parse_json_component(value: &serde_json::Value, allow_primitive: bool) -> Option<Text> {
    match value {
        serde_json::Value::String(s) => Some(Text::literal(s.clone())),
        serde_json::Value::Bool(b) if allow_primitive => Some(Text::literal(b.to_string())),
        serde_json::Value::Number(n) if allow_primitive => Some(Text::literal(n.to_string())),
        serde_json::Value::Array(items) => {
            let (first, rest) = items.split_first()?;
            let mut root = parse_json_component(first, false)?;
            for item in rest {
                root.children.push(parse_json_component(item, false)?);
            }
            Some(root)
        }
        serde_json::Value::Object(map) => {
            let content = if let Some(text) = map.get("text") {
                TextContent::Literal(text.as_str()?.to_owned())
            } else {
                let key = map.get("translate")?.as_str()?.to_owned();
                let fallback = match map.get("fallback") {
                    None => None,
                    Some(v) => Some(v.as_str()?.to_owned()),
                };
                let arguments = match map.get("with") {
                    None => Vec::new(),
                    Some(serde_json::Value::Array(args)) => args
                        .iter()
                        .map(|arg| parse_json_component(arg, true))
                        .collect::<Option<Vec<_>>>()?,
                    Some(_) => return None,
                };
                TextContent::Translatable {
                    key,
                    fallback,
                    arguments,
                }
            };
            let mut style = TextStyle::default();
            if let Some(color) = map.get("color") {
                style.color = Some(color.as_str()?.to_owned());
            }
            if let Some(bold) = map.get("bold") {
                style.bold = Some(bold.as_bool()?);
            }
            if let Some(italic) = map.get("italic") {
                style.italic = Some(italic.as_bool()?);
            }
            if let Some(underlined) = map.get("underlined") {
                style.underlined = Some(underlined.as_bool()?);
            }
            if let Some(strikethrough) = map.get("strikethrough") {
                style.strikethrough = Some(strikethrough.as_bool()?);
            }
            if let Some(obfuscated) = map.get("obfuscated") {
                style.obfuscated = Some(obfuscated.as_bool()?);
            }
            let mut text = Text::new(content).with_style(style);
            if let Some(extra) = map.get("extra") {
                let items = extra.as_array()?;
                if items.is_empty() {
                    return None;
                }
                for item in items {
                    text.children.push(parse_json_component(item, false)?);
                }
            }
            Some(text)
        }
        _ => None,
    }
}

/// Compile-only Minecraft 26.3 + Fabric API 5.0.21 stubs for the client dispatch fixture.
const CLIENT_DISPATCH_STUBS: &[(&str, &str)] = &[
    (
        "net/fabricmc/api/ModInitializer.java",
        "package net.fabricmc.api; public interface ModInitializer { void onInitialize(); }",
    ),
    (
        "net/minecraft/ChatFormatting.java",
        "package net.minecraft; public enum ChatFormatting { BLACK, DARK_BLUE, DARK_GREEN, DARK_AQUA, DARK_RED, DARK_PURPLE, GOLD, GRAY, DARK_GRAY, BLUE, GREEN, AQUA, RED, LIGHT_PURPLE, YELLOW, WHITE, OBFUSCATED, BOLD, STRIKETHROUGH, UNDERLINE, ITALIC, RESET; }",
    ),
    (
        "net/minecraft/resources/Identifier.java",
        "package net.minecraft.resources; public final class Identifier { public static Identifier fromNamespaceAndPath(String ns, String path) { throw new AssertionError(); } }",
    ),
    (
        "net/minecraft/resources/ResourceKey.java",
        "package net.minecraft.resources; public final class ResourceKey<T> { public static <T> ResourceKey<T> create(ResourceKey<?> reg, Identifier id) { throw new AssertionError(); } }",
    ),
    (
        "net/minecraft/core/Registry.java",
        "package net.minecraft.core; import net.minecraft.resources.Identifier; public interface Registry<T> { static <V, T extends V> T register(Registry<V> registry, Identifier id, T value) { throw new AssertionError(); } }",
    ),
    (
        "net/minecraft/core/DefaultedRegistry.java",
        "package net.minecraft.core; public interface DefaultedRegistry<T> extends Registry<T> {}",
    ),
    (
        "net/minecraft/core/registries/BuiltInRegistries.java",
        "package net.minecraft.core.registries; import net.minecraft.core.DefaultedRegistry; import net.minecraft.world.item.Item; public final class BuiltInRegistries { public static final DefaultedRegistry<Item> ITEM = null; }",
    ),
    (
        "net/minecraft/network/chat/Component.java",
        "package net.minecraft.network.chat; public interface Component { static MutableComponent translatable(String k) { throw new AssertionError(); } static MutableComponent translatable(String k, Object... args) { throw new AssertionError(); } static MutableComponent literal(String s) { throw new AssertionError(); } }",
    ),
    (
        "net/minecraft/network/chat/MutableComponent.java",
        "package net.minecraft.network.chat; import net.minecraft.ChatFormatting; public interface MutableComponent extends Component { MutableComponent withStyle(ChatFormatting... styles); }",
    ),
    (
        "net/minecraft/world/flag/FeatureFlag.java",
        "package net.minecraft.world.flag; public class FeatureFlag {}",
    ),
    (
        "net/minecraft/world/flag/FeatureFlagSet.java",
        "package net.minecraft.world.flag; public final class FeatureFlagSet {}",
    ),
    (
        "net/minecraft/world/flag/FeatureFlags.java",
        "package net.minecraft.world.flag; public final class FeatureFlags { public static final FeatureFlag VANILLA = null; public static final FeatureFlag TRADE_REBALANCE = null; }",
    ),
    (
        "net/minecraft/world/level/ItemLike.java",
        "package net.minecraft.world.level; import net.minecraft.world.item.Item; public interface ItemLike { Item asItem(); }",
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
        "net/minecraft/world/item/Item.java",
        r#"package net.minecraft.world.item;
import java.util.function.Consumer;
import net.minecraft.network.chat.Component;
import net.minecraft.world.flag.FeatureFlag;
import net.minecraft.world.item.component.TooltipDisplay;
import net.minecraft.world.level.ItemLike;
public class Item implements ItemLike {
    public static class Properties {
        public Properties() {}
        public Properties requiredFeatures(FeatureFlag... flags) { throw new AssertionError(); }
    }
    public interface TooltipContext {}
    public Item(Properties properties) {}
    @Override public Item asItem() { return this; }
    public void appendHoverText(ItemStack stack, TooltipContext ctx, TooltipDisplay display, Consumer<Component> out, TooltipFlag flag) {}
}"#,
    ),
    (
        "net/minecraft/world/item/Items.java",
        "package net.minecraft.world.item; public final class Items { public static final Item STONE = null; public static final Item DIRT = null; public static final Item GOLD_INGOT = null; }",
    ),
    (
        "net/minecraft/world/item/ItemStack.java",
        "package net.minecraft.world.item; import net.minecraft.world.level.ItemLike; public final class ItemStack { public ItemStack(ItemLike item) {} }",
    ),
    (
        "net/minecraft/world/item/CreativeModeTab.java",
        r#"package net.minecraft.world.item;
import net.minecraft.world.level.ItemLike;
public class CreativeModeTab {
    public enum TabVisibility {
        PARENT_AND_SEARCH_TABS,
        PARENT_TAB_ONLY,
        SEARCH_TAB_ONLY
    }
    public interface Output {
        void accept(ItemStack stack, TabVisibility visibility);
        default void accept(ItemStack stack) { accept(stack, TabVisibility.PARENT_AND_SEARCH_TABS); }
        default void accept(ItemLike item, TabVisibility visibility) { accept(new ItemStack(item), visibility); }
        default void accept(ItemLike item) { accept(new ItemStack(item), TabVisibility.PARENT_AND_SEARCH_TABS); }
    }
}"#,
    ),
    (
        "net/minecraft/world/item/CreativeModeTabs.java",
        "package net.minecraft.world.item; import net.minecraft.resources.ResourceKey; public final class CreativeModeTabs { public static final ResourceKey<CreativeModeTab> FUNCTIONAL_BLOCKS = null; public static final ResourceKey<CreativeModeTab> COMBAT = null; }",
    ),
    (
        "net/fabricmc/fabric/api/event/Event.java",
        "package net.fabricmc.fabric.api.event; public abstract class Event<T> { public abstract void register(T listener); }",
    ),
    (
        "net/fabricmc/fabric/api/creativetab/v1/FabricCreativeModeTabOutput.java",
        r#"package net.fabricmc.fabric.api.creativetab.v1;
import java.util.Collection;
import net.minecraft.world.item.CreativeModeTab;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.level.ItemLike;
public class FabricCreativeModeTabOutput implements CreativeModeTab.Output {
    @Override public void accept(ItemStack stack, CreativeModeTab.TabVisibility visibility) { throw new AssertionError(); }
    public void prepend(ItemLike item) { throw new AssertionError(); }
    public void prepend(ItemStack stack, CreativeModeTab.TabVisibility visibility) { throw new AssertionError(); }
    public void insertBefore(ItemLike anchor, Collection<ItemStack> stacks, CreativeModeTab.TabVisibility visibility) { throw new AssertionError(); }
    public void insertBefore(ItemLike anchor, ItemLike... items) { throw new AssertionError(); }
    public void insertAfter(ItemLike anchor, Collection<ItemStack> stacks, CreativeModeTab.TabVisibility visibility) { throw new AssertionError(); }
    public void insertAfter(ItemLike anchor, ItemLike... items) { throw new AssertionError(); }
}"#,
    ),
    (
        "net/fabricmc/fabric/api/creativetab/v1/CreativeModeTabEvents.java",
        r#"package net.fabricmc.fabric.api.creativetab.v1;
import net.fabricmc.fabric.api.event.Event;
import net.minecraft.resources.ResourceKey;
import net.minecraft.world.item.CreativeModeTab;
public final class CreativeModeTabEvents {
    public static final Event<ModifyOutputAll> MODIFY_OUTPUT_ALL = null;
    public interface ModifyOutput { void modifyOutput(FabricCreativeModeTabOutput output); }
    public interface ModifyOutputAll { void modifyOutput(CreativeModeTab tab, FabricCreativeModeTabOutput output); }
    public static Event<ModifyOutput> modifyOutputEvent(ResourceKey<CreativeModeTab> tabKey) { throw new AssertionError(); }
}"#,
    ),
];

const FIXTURE_MOD_SOURCE: &str = r#"package demo;
import java.util.List;
import java.util.function.Consumer;
import net.fabricmc.api.ModInitializer;
import net.fabricmc.fabric.api.creativetab.v1.CreativeModeTabEvents;
import net.minecraft.ChatFormatting;
import net.minecraft.core.Registry;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.network.chat.Component;
import net.minecraft.resources.Identifier;
import net.minecraft.world.flag.FeatureFlags;
import net.minecraft.world.item.CreativeModeTab;
import net.minecraft.world.item.CreativeModeTabs;
import net.minecraft.world.item.Item;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.item.Items;
import net.minecraft.world.item.TooltipFlag;
import net.minecraft.world.item.component.TooltipDisplay;

public final class FixtureMod implements ModInitializer {
    static final class StyledTooltipItem extends Item {
        StyledTooltipItem(Properties props) {
            super(props);
        }

        @Override
        public void appendHoverText(
            ItemStack stack,
            TooltipContext context,
            TooltipDisplay display,
            Consumer<Component> out,
            TooltipFlag flag
        ) {
            out.accept(
                Component.translatable("demo.tooltip.hint", "ok")
                    .withStyle(ChatFormatting.GRAY, ChatFormatting.ITALIC)
            );
            // Arbitrary Object arguments are accepted by TranslatableContents at construction
            // but refused by ComponentSerialization.CODEC -> must be counted, not silently dropped.
            out.accept(Component.translatable("demo.tooltip.unconvertible", new Object()));
        }
    }

    private static Item register(String path, Item item) {
        return Registry.register(
            BuiltInRegistries.ITEM,
            Identifier.fromNamespaceAndPath("demo", path),
            item
        );
    }

    @Override
    public void onInitialize() {
        Item beforeAnchor = register("before_anchor", new Item(new Item.Properties()));
        Item afterAnchor = register("after_anchor", new Item(new Item.Properties()));
        Item followsBefore = register("follows_before", new Item(new Item.Properties()));
        Item tradeItem = register(
            "trade_flag_item",
            new Item(new Item.Properties().requiredFeatures(FeatureFlags.TRADE_REBALANCE))
        );
        Item missingAnchor = register("missing_anchor_item", new Item(new Item.Properties()));
        Item prepended = register("prepended_item", new Item(new Item.Properties()));
        Item combatParent = register("combat_parent_only", new Item(new Item.Properties()));
        Item combatSearch = register("combat_search_only", new Item(new Item.Properties()));
        Item combatAll = register("combat_all_tabs", new Item(new Item.Properties()));
        register("tooltip_item", new StyledTooltipItem(new Item.Properties()));

        CreativeModeTabEvents.modifyOutputEvent(CreativeModeTabs.FUNCTIONAL_BLOCKS).register(entries -> {
            entries.insertBefore(
                Items.DIRT,
                List.of(new ItemStack(beforeAnchor)),
                CreativeModeTab.TabVisibility.PARENT_TAB_ONLY
            );
            entries.insertAfter(
                Items.STONE,
                List.of(new ItemStack(afterAnchor)),
                CreativeModeTab.TabVisibility.SEARCH_TAB_ONLY
            );
            // In Parent, `beforeAnchor` is present so both follow it; in Search, `beforeAnchor`
            // is absent so enabled stacks append to the end. `tradeItem` is filtered unless
            // TRADE_REBALANCE is enabled.
            entries.insertAfter(beforeAnchor, followsBefore, tradeItem);
            // Missing anchor appends to the end of both lists (Fabric 5.0.21 semantics).
            entries.insertBefore(Items.GOLD_INGOT, missingAnchor);
            entries.prepend(prepended);
        });

        CreativeModeTabEvents.modifyOutputEvent(CreativeModeTabs.COMBAT).register(entries -> {
            entries.accept(combatParent, CreativeModeTab.TabVisibility.PARENT_TAB_ONLY);
            entries.accept(combatSearch, CreativeModeTab.TabVisibility.SEARCH_TAB_ONLY);
            entries.accept(combatAll);
        });
    }
}
"#;

fn report_row(java: &HorizonJava) -> serde_json::Value {
    java.lock_client()
        .report_rows()
        .next()
        .expect("one client jar row")
}

fn unfired_sources(row: &serde_json::Value) -> Vec<(String, u64)> {
    row["client"]["unfired_registrations"]
        .as_array()
        .expect("unfired_registrations array")
        .iter()
        .map(|entry| {
            (
                entry["source"].as_str().unwrap().to_owned(),
                entry["count"].as_u64().unwrap(),
            )
        })
        .collect()
}

#[test]
fn creative_features_use_exact_vanilla_masks_and_refuse_unknowns() {
    assert_eq!(
        crate::runtime::creative_feature_mask(&["minecraft:vanilla".into()]),
        Some(1)
    );
    assert_eq!(
        crate::runtime::creative_feature_mask(&[
            "minecraft:vanilla".into(),
            "minecraft:trade_rebalance".into(),
        ]),
        Some(3)
    );
    assert_eq!(
        crate::runtime::creative_feature_mask(&["example:future".into()]),
        None
    );
}

#[test]
fn javac_fixture_creative_tab_anchors_visibility_flags_and_tooltip_bridge() {
    let mut sources = CLIENT_DISPATCH_STUBS.to_vec();
    sources.push(("demo/FixtureMod.java", FIXTURE_MOD_SOURCE));
    let jar_bytes = build_javac_jar(
        "client-dispatch",
        r#"{"schemaVersion":1,"id":"demo","version":"1.0","entrypoints":{"main":["demo.FixtureMod"]}}"#,
        &sources,
    );

    let java = Arc::new(HorizonJava::default());
    java.lock_client()
        .set_fixture_files(vec![("demo-client.jar".to_owned(), jar_bytes)]);

    let mods = vec![ModEntry::new(
        "horizon_java",
        Box::new(SharedHorizonJava(Arc::clone(&java))),
    )];
    mod_api::testing::with_mods(mods.into(), || {
        mod_api::testing::rebuild_parts(|| {
            let mut host = TestClientHost::new(&[
                "minecraft:stone",
                "minecraft:dirt",
                "minecraft:gold_ingot",
                "minecraft:iron_sword",
                "demo:before_anchor",
                "demo:after_anchor",
                "demo:follows_before",
                "demo:trade_flag_item",
                "demo:missing_anchor_item",
                "demo:prepended_item",
                "demo:combat_parent_only",
                "demo:combat_search_only",
                "demo:combat_all_tabs",
                "demo:tooltip_item",
            ]);

            // Load client jars through the real client_setup hook (sets game_version = "26.3").
            mod_api::dispatch_client_setup(&mut host);

            let functional_source = "net/fabricmc/fabric/api/creativetab/v1/CreativeModeTabEvents.modifyOutputEvent/minecraft:creative_mode_tab:minecraft:functional_blocks";
            let combat_source = "net/fabricmc/fabric/api/creativetab/v1/CreativeModeTabEvents.modifyOutputEvent/minecraft:creative_mode_tab:minecraft:combat";

            let initial_row = report_row(&java);
            assert_eq!(initial_row["verdict"], "ran-clean");
            assert_eq!(initial_row["unknown_host_call_count"], 0);
            assert_eq!(
                unfired_sources(&initial_row),
                vec![
                    (combat_source.to_owned(), 1),
                    (functional_source.to_owned(), 1),
                ]
            );

            // 1) Functional Blocks with vanilla-only flags: anchors, parent/search visibility,
            //    prepend, missing-anchor fallback, and filtered trade_flag_item — exact Java 25
            //    FabricCreativeModeTabOutput oracle order.
            let edit = host.begin_tab(
                1,
                &["minecraft:stone", "minecraft:dirt"],
                &["minecraft:stone", "minecraft:dirt"],
            );
            mod_api::dispatch_build_creative_mode_tab_contents(
                HostSide::Client(&mut host),
                "minecraft:functional_blocks",
                vec!["minecraft:vanilla".to_owned()],
                CreativeTabPermissions::Normal,
                edit,
            );
            assert_eq!(
                host.tab_entries(CreativeTabList::Parent),
                vec![
                    ("demo:prepended_item".to_owned(), 1),
                    ("minecraft:stone".to_owned(), 1),
                    ("demo:before_anchor".to_owned(), 1),
                    ("demo:follows_before".to_owned(), 1),
                    ("minecraft:dirt".to_owned(), 1),
                    ("demo:missing_anchor_item".to_owned(), 1),
                ]
            );
            assert_eq!(
                host.tab_entries(CreativeTabList::Search),
                vec![
                    ("demo:prepended_item".to_owned(), 1),
                    ("minecraft:stone".to_owned(), 1),
                    ("demo:after_anchor".to_owned(), 1),
                    ("minecraft:dirt".to_owned(), 1),
                    ("demo:follows_before".to_owned(), 1),
                    ("demo:missing_anchor_item".to_owned(), 1),
                ]
            );
            let after_functional = report_row(&java);
            assert_eq!(after_functional["unknown_host_call_count"], 0);
            assert_eq!(
                unfired_sources(&after_functional),
                vec![(combat_source.to_owned(), 1)]
            );

            // 2) Functional Blocks with TRADE_REBALANCE enabled: `demo:trade_flag_item` is now
            //    included immediately after `demo:follows_before` in both Parent and Search.
            let edit = host.begin_tab(
                2,
                &["minecraft:stone", "minecraft:dirt"],
                &["minecraft:stone", "minecraft:dirt"],
            );
            mod_api::dispatch_build_creative_mode_tab_contents(
                HostSide::Client(&mut host),
                "minecraft:functional_blocks",
                vec![
                    "minecraft:vanilla".to_owned(),
                    "minecraft:trade_rebalance".to_owned(),
                ],
                CreativeTabPermissions::Normal,
                edit,
            );
            assert_eq!(
                host.tab_entries(CreativeTabList::Parent),
                vec![
                    ("demo:prepended_item".to_owned(), 1),
                    ("minecraft:stone".to_owned(), 1),
                    ("demo:before_anchor".to_owned(), 1),
                    ("demo:follows_before".to_owned(), 1),
                    ("demo:trade_flag_item".to_owned(), 1),
                    ("minecraft:dirt".to_owned(), 1),
                    ("demo:missing_anchor_item".to_owned(), 1),
                ]
            );
            assert_eq!(
                host.tab_entries(CreativeTabList::Search),
                vec![
                    ("demo:prepended_item".to_owned(), 1),
                    ("minecraft:stone".to_owned(), 1),
                    ("demo:after_anchor".to_owned(), 1),
                    ("minecraft:dirt".to_owned(), 1),
                    ("demo:follows_before".to_owned(), 1),
                    ("demo:trade_flag_item".to_owned(), 1),
                    ("demo:missing_anchor_item".to_owned(), 1),
                ]
            );
            assert_eq!(report_row(&java)["unknown_host_call_count"], 0);

            // 3) Combat tab: parent-only vs search-only vs both visibility, and clears the
            //    remaining unfired creative-tab registration.
            let edit = host.begin_tab(3, &["minecraft:iron_sword"], &["minecraft:iron_sword"]);
            mod_api::dispatch_build_creative_mode_tab_contents(
                HostSide::Client(&mut host),
                "minecraft:combat",
                vec!["minecraft:vanilla".to_owned()],
                CreativeTabPermissions::Normal,
                edit,
            );
            assert_eq!(
                host.tab_entries(CreativeTabList::Parent),
                vec![
                    ("minecraft:iron_sword".to_owned(), 1),
                    ("demo:combat_parent_only".to_owned(), 1),
                    ("demo:combat_all_tabs".to_owned(), 1),
                ]
            );
            assert_eq!(
                host.tab_entries(CreativeTabList::Search),
                vec![
                    ("minecraft:iron_sword".to_owned(), 1),
                    ("demo:combat_search_only".to_owned(), 1),
                    ("demo:combat_all_tabs".to_owned(), 1),
                ]
            );
            let after_combat = report_row(&java);
            assert_eq!(after_combat["unknown_host_call_count"], 0);
            assert!(unfired_sources(&after_combat).is_empty());

            // 4) Unknown feature flag is refused and counted, leaving the tab untouched.
            let edit = host.begin_tab(4, &["minecraft:stone"], &["minecraft:stone"]);
            mod_api::dispatch_build_creative_mode_tab_contents(
                HostSide::Client(&mut host),
                "minecraft:functional_blocks",
                vec!["example:unknown".to_owned()],
                CreativeTabPermissions::Normal,
                edit,
            );
            assert_eq!(
                host.tab_entries(CreativeTabList::Parent),
                vec![("minecraft:stone".to_owned(), 1)]
            );
            assert_eq!(report_row(&java)["unknown_host_call_count"], 1);

            // 5) Item tooltip: styled translatable line is spliced immediately after line 0
            //    (item name) and before advanced lines; the unconvertible component is NOT
            //    emitted and IS counted (unknown_host_call_count goes 1 -> 2).
            let tooltip_stack = host.stack("demo:tooltip_item");
            let display = TooltipDisplay::for_stack(&tooltip_stack);
            let mut lines = vec![
                (Text::literal("Demo Tooltip Item"), 0xFFFF_FFFF),
                (Text::literal("demo:tooltip_item"), 0xFF55_5555),
            ];
            mod_api::dispatch_item_tooltip(
                &host,
                mod_api::__private::ItemTooltipCall {
                    player: 0,
                    player_entity_id: 1,
                    stack: &tooltip_stack,
                    display: &display,
                    advanced: true,
                    creative: false,
                    lines: &mut lines,
                },
            );
            let mut expected_style = TextStyle::default();
            expected_style.color = Some("gray".to_owned());
            expected_style.italic = Some(true);
            let expected_line =
                Text::translate("demo.tooltip.hint", vec![Text::literal("ok")])
                    .with_style(expected_style);
            assert_eq!(
                lines,
                vec![
                    (Text::literal("Demo Tooltip Item"), 0xFFFF_FFFF),
                    (expected_line.clone(), u32::MAX),
                    (Text::literal("demo:tooltip_item"), 0xFF55_5555),
                ]
            );
            assert_eq!(
                report_row(&java)["unknown_host_call_count"],
                2,
                "unconvertible tooltip component must be counted, not silently dropped"
            );

            // 6) `hide_tooltip` skips `appendHoverText` when `creative == false` (vanilla
            //    `ItemStack.getTooltipLines`), and runs it when `creative == true`.
            let mut hidden_stack = host.stack("demo:tooltip_item");
            hidden_stack
                .set_components(
                    [(
                        "minecraft:tooltip_display".into(),
                        Tag::compound(vec![("hide_tooltip", Tag::Byte(1))]),
                    )]
                    .into(),
                )
                .unwrap();
            let hidden_display = TooltipDisplay::for_stack(&hidden_stack);
            let mut hidden_lines = Vec::new();
            mod_api::dispatch_item_tooltip(
                &host,
                mod_api::__private::ItemTooltipCall {
                    player: 0,
                    player_entity_id: 1,
                    stack: &hidden_stack,
                    display: &hidden_display,
                    advanced: false,
                    creative: false,
                    lines: &mut hidden_lines,
                },
            );
            assert!(hidden_lines.is_empty());
            assert_eq!(report_row(&java)["unknown_host_call_count"], 2);

            let mut creative_lines = vec![(Text::literal("Demo Tooltip Item"), 0xFFFF_FFFF)];
            mod_api::dispatch_item_tooltip(
                &host,
                mod_api::__private::ItemTooltipCall {
                    player: 0,
                    player_entity_id: 1,
                    stack: &hidden_stack,
                    display: &hidden_display,
                    advanced: false,
                    creative: true,
                    lines: &mut creative_lines,
                },
            );
            assert_eq!(
                creative_lines,
                vec![
                    (Text::literal("Demo Tooltip Item"), 0xFFFF_FFFF),
                    (expected_line, u32::MAX),
                ]
            );
            assert_eq!(report_row(&java)["unknown_host_call_count"], 3);

            Ok::<_, ()>(())
        })
        .unwrap();
    });
}

#[test]
fn real_runes_jar_creative_tabs_and_altar_tooltip_through_sunrise_bridge() {
    let Some(path) = std::env::var_os("RUNES_JAR").map(PathBuf::from) else {
        eprintln!("RUNES_JAR is unset; real Runes test skipped");
        return;
    };
    if !path.exists() {
        eprintln!("RUNES_JAR does not exist ({path:?}); real Runes test skipped");
        return;
    }
    // 16 MiB stack so unoptimised debug builds do not hit `frame_bound`'s 2 MiB floor
    // while Runes initializes its nested `bundle-api` jar.
    std::thread::Builder::new()
        .name("real-runes-sunrise-bridge".into())
        .stack_size(16 << 20)
        .spawn(move || run_real_runes_bridge_test(&path))
        .unwrap()
        .join()
        .unwrap();
}

fn run_real_runes_bridge_test(path: &Path) {
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("runes.jar")
        .to_owned();
    let jar_bytes = fs::read(path).expect("read RUNES_JAR");

    let java = Arc::new(HorizonJava::default());
    java.lock_client()
        .set_fixture_files(vec![(file_name, jar_bytes)]);

    let mods = vec![ModEntry::new(
        "horizon_java",
        Box::new(SharedHorizonJava(Arc::clone(&java))),
    )];
    mod_api::testing::with_mods(mods.into(), || {
        mod_api::testing::rebuild_parts(|| {
            let mut host = TestClientHost::new(&[
                "minecraft:crafting_table",
                "minecraft:diamond_sword",
                "runes:crafting_altar",
                "runes:arcane_stone",
                "runes:fire_stone",
                "runes:frost_stone",
                "runes:healing_stone",
                "runes:lightning_stone",
                "runes:soul_stone",
                "runes:small_rune_pouch",
                "runes:medium_rune_pouch",
                "runes:large_rune_pouch",
            ]);

            mod_api::dispatch_client_setup(&mut host);
            assert_eq!(java.lock_client().resident_count(), 1);

            let functional_source = "net/fabricmc/fabric/api/creativetab/v1/CreativeModeTabEvents.modifyOutputEvent/minecraft:creative_mode_tab:minecraft:functional_blocks";
            let combat_source = "net/fabricmc/fabric/api/creativetab/v1/CreativeModeTabEvents.modifyOutputEvent/minecraft:creative_mode_tab:minecraft:combat";

            let before_dispatch = report_row(&java);
            let before_unknown = before_dispatch["unknown_host_call_count"]
                .as_u64()
                .expect("unknown_host_call_count");
            let before_unfired = unfired_sources(&before_dispatch);
            assert!(
                before_unfired.contains(&(functional_source.to_owned(), 1)),
                "expected functional_blocks creative listener before tab build: {before_unfired:?}"
            );
            assert!(
                before_unfired.contains(&(combat_source.to_owned(), 1)),
                "expected combat creative listener before tab build: {before_unfired:?}"
            );

            // Functional Blocks: Runes appends `runes:crafting_altar` after vanilla entries.
            let edit = host.begin_tab(
                10,
                &["minecraft:crafting_table"],
                &["minecraft:crafting_table"],
            );
            mod_api::dispatch_build_creative_mode_tab_contents(
                HostSide::Client(&mut host),
                "minecraft:functional_blocks",
                vec!["minecraft:vanilla".to_owned()],
                CreativeTabPermissions::Normal,
                edit,
            );
            let expected_functional = vec![
                ("minecraft:crafting_table".to_owned(), 1),
                ("runes:crafting_altar".to_owned(), 1),
            ];
            assert_eq!(host.tab_entries(CreativeTabList::Parent), expected_functional);
            assert_eq!(host.tab_entries(CreativeTabList::Search), expected_functional);

            // Combat: Runes appends the six rune stones, then bundled `bundle-api` appends the
            // three rune pouches.
            let edit = host.begin_tab(
                11,
                &["minecraft:diamond_sword"],
                &["minecraft:diamond_sword"],
            );
            mod_api::dispatch_build_creative_mode_tab_contents(
                HostSide::Client(&mut host),
                "minecraft:combat",
                vec!["minecraft:vanilla".to_owned()],
                CreativeTabPermissions::Normal,
                edit,
            );
            let expected_combat = vec![
                ("minecraft:diamond_sword".to_owned(), 1),
                ("runes:arcane_stone".to_owned(), 1),
                ("runes:fire_stone".to_owned(), 1),
                ("runes:frost_stone".to_owned(), 1),
                ("runes:healing_stone".to_owned(), 1),
                ("runes:lightning_stone".to_owned(), 1),
                ("runes:soul_stone".to_owned(), 1),
                ("runes:small_rune_pouch".to_owned(), 1),
                ("runes:medium_rune_pouch".to_owned(), 1),
                ("runes:large_rune_pouch".to_owned(), 1),
            ];
            assert_eq!(host.tab_entries(CreativeTabList::Parent), expected_combat);
            assert_eq!(host.tab_entries(CreativeTabList::Search), expected_combat);

            // Both creative-tab registrations must be cleared from `unfired_registrations`.
            let after_tabs = report_row(&java);
            let after_unfired = unfired_sources(&after_tabs);
            assert!(
                !after_unfired.iter().any(|(s, _)| s == functional_source),
                "functional_blocks listener remained unfired: {after_unfired:?}"
            );
            assert!(
                !after_unfired.iter().any(|(s, _)| s == combat_source),
                "combat listener remained unfired: {after_unfired:?}"
            );

            // Altar item tooltip: `RuneCraftingBlock$1.appendHoverText` appends italic gray
            // `block.runes.crafting_altar.hint` right after the item name.
            let altar = host.stack("runes:crafting_altar");
            let display = TooltipDisplay::for_stack(&altar);
            let mut lines = vec![(Text::literal("Rune Crafting Altar"), 0xFFFF_FFFF)];
            mod_api::dispatch_item_tooltip(
                &host,
                mod_api::__private::ItemTooltipCall {
                    player: 0,
                    player_entity_id: 1,
                    stack: &altar,
                    display: &display,
                    advanced: false,
                    creative: false,
                    lines: &mut lines,
                },
            );
            let mut expected_style = TextStyle::default();
            expected_style.color = Some("gray".to_owned());
            expected_style.italic = Some(true);
            let expected_hint =
                Text::translate("block.runes.crafting_altar.hint", Vec::new())
                    .with_style(expected_style);
            assert_eq!(
                lines,
                vec![
                    (Text::literal("Rune Crafting Altar"), 0xFFFF_FFFF),
                    (expected_hint, u32::MAX),
                ]
            );

            let after_all = report_row(&java);
            assert_eq!(
                after_all["unknown_host_call_count"].as_u64(),
                Some(before_unknown),
                "creative tab builds and altar tooltip must add zero unknown host calls"
            );

            Ok::<_, ()>(())
        })
        .unwrap();
    });
}

const EDGE_BRIDGE_SOURCE: &str = r#"package demo;
import net.fabricmc.api.ModInitializer;
import net.fabricmc.fabric.api.creativetab.v1.CreativeModeTabEvents;
import net.minecraft.core.Registry;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.resources.Identifier;
import net.minecraft.world.item.CreativeModeTabs;
import net.minecraft.world.item.Item;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.item.Items;

public final class EdgeBridgeMod implements ModInitializer {
    @Override
    public void onInitialize() {
        Item unmapped = Registry.register(
            BuiltInRegistries.ITEM,
            Identifier.fromNamespaceAndPath("demo", "not_in_sunrise"),
            new Item(new Item.Properties())
        );
        Item afterCustom = Registry.register(
            BuiltInRegistries.ITEM,
            Identifier.fromNamespaceAndPath("demo", "after_custom"),
            new Item(new Item.Properties())
        );
        Item allTabExtra = Registry.register(
            BuiltInRegistries.ITEM,
            Identifier.fromNamespaceAndPath("demo", "all_tab_extra"),
            new Item(new Item.Properties())
        );

        CreativeModeTabEvents.modifyOutputEvent(CreativeModeTabs.FUNCTIONAL_BLOCKS).register(entries -> {
            // Duplicate of seeded vanilla entry: vanilla ItemStackLinkedSet keeps first occurrence
            // without error.
            entries.accept(Items.STONE);
            // Unregistered in Sunrise's item table: bridge must count conversion refusal.
            entries.accept(unmapped);
            // Valid item placed after plain `Items.DIRT` (which sits after a custom-component stack).
            entries.insertAfter(Items.DIRT, afterCustom);
        });

        CreativeModeTabEvents.modifyOutputEvent(CreativeModeTabs.COMBAT).register(entries -> {
            // `ItemStack` anchor against a seeded stack with custom components must be counted
            // rather than falsely equating components.
            entries.insertAfter(new ItemStack(Items.STONE), afterCustom);
        });

        CreativeModeTabEvents.MODIFY_OUTPUT_ALL.register((tab, entries) -> {
            entries.accept(allTabExtra);
        });
    }
}
"#;

#[test]
fn bridge_counts_unmapped_stacks_and_opaque_anchors_and_deduplicates_like_vanilla() {
    let mut sources = CLIENT_DISPATCH_STUBS.to_vec();
    sources.push(("demo/EdgeBridgeMod.java", EDGE_BRIDGE_SOURCE));
    sources.push((
        "net/fabricmc/fabric/api/creativetab/v1/FabricCreativeModeTabOutput.java",
        r#"package net.fabricmc.fabric.api.creativetab.v1;
import java.util.Collection;
import net.minecraft.world.item.CreativeModeTab;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.level.ItemLike;
public class FabricCreativeModeTabOutput implements CreativeModeTab.Output {
    @Override public void accept(ItemStack stack, CreativeModeTab.TabVisibility visibility) { throw new AssertionError(); }
    public void prepend(ItemLike item) { throw new AssertionError(); }
    public void prepend(ItemStack stack, CreativeModeTab.TabVisibility visibility) { throw new AssertionError(); }
    public void insertBefore(ItemLike anchor, Collection<ItemStack> stacks, CreativeModeTab.TabVisibility visibility) { throw new AssertionError(); }
    public void insertBefore(ItemLike anchor, ItemLike... items) { throw new AssertionError(); }
    public void insertAfter(ItemLike anchor, Collection<ItemStack> stacks, CreativeModeTab.TabVisibility visibility) { throw new AssertionError(); }
    public void insertAfter(ItemLike anchor, ItemLike... items) { throw new AssertionError(); }
    public void insertAfter(ItemStack anchor, ItemLike... items) { throw new AssertionError(); }
}"#,
    ));
    // Replace the earlier stub entry for FabricCreativeModeTabOutput so javac sees only one.
    let stub_idx = sources
        .iter()
        .position(|(p, _)| {
            *p == "net/fabricmc/fabric/api/creativetab/v1/FabricCreativeModeTabOutput.java"
        })
        .unwrap();
    let replacement = sources.pop().unwrap();
    sources[stub_idx] = replacement;

    let jar_bytes = build_javac_jar(
        "client-dispatch-edges",
        r#"{"schemaVersion":1,"id":"demo","version":"1.0","entrypoints":{"main":["demo.EdgeBridgeMod"]}}"#,
        &sources,
    );

    let java = Arc::new(HorizonJava::default());
    java.lock_client()
        .set_fixture_files(vec![("demo-edges.jar".to_owned(), jar_bytes)]);

    let mods = vec![ModEntry::new(
        "horizon_java",
        Box::new(SharedHorizonJava(Arc::clone(&java))),
    )];
    mod_api::testing::with_mods(mods.into(), || {
        mod_api::testing::rebuild_parts(|| {
            // Deliberately omit `demo:not_in_sunrise` from Sunrise's item table.
            let mut host = TestClientHost::new(&[
                "minecraft:stone",
                "minecraft:dirt",
                "demo:after_custom",
                "demo:all_tab_extra",
            ]);
            mod_api::dispatch_client_setup(&mut host);
            assert_eq!(report_row(&java)["unknown_host_call_count"], 0);

            // Seed a damaged stone stack + plain dirt so `seed_creative_stacks` preserves custom
            // components on stone while plain dirt remains matchable.
            let mut damaged_stone = host.stack("minecraft:stone");
            damaged_stone.set_damage(3);
            let dirt = host.stack("minecraft:dirt");
            let edit = mod_api::__private::creative_tab_edit(20);
            {
                let mut editor = host.editor.borrow_mut();
                editor.active = Some(edit);
                editor.parent = vec![damaged_stone.clone(), dirt.clone()];
                editor.search = vec![damaged_stone.clone(), dirt.clone()];
            }
            mod_api::dispatch_build_creative_mode_tab_contents(
                HostSide::Client(&mut host),
                "minecraft:functional_blocks",
                vec!["minecraft:vanilla".to_owned()],
                CreativeTabPermissions::Normal,
                edit,
            );
            // `Items.STONE` duplicate adds plain stone beside damaged stone (different components!),
            // `demo:not_in_sunrise` is refused and counted once per list (2 total), `demo:after_custom`
            // lands after `minecraft:dirt`, and `MODIFY_OUTPUT_ALL` appends `demo:all_tab_extra`.
            assert_eq!(
                host.tab_entries(CreativeTabList::Parent),
                vec![
                    ("minecraft:stone".to_owned(), 1),
                    ("minecraft:dirt".to_owned(), 1),
                    ("demo:after_custom".to_owned(), 1),
                    ("minecraft:stone".to_owned(), 1),
                    ("demo:all_tab_extra".to_owned(), 1),
                ]
            );
            assert_eq!(host.editor.borrow().parent[0].damage_component(), Some(3));
            assert_eq!(host.editor.borrow().parent[3].damage_component(), None);
            assert_eq!(
                report_row(&java)["unknown_host_call_count"],
                2,
                "unmapped Sunrise item must be counted once for Parent and once for Search"
            );

            // Duplicate of an identical plain stack is ignored cleanly without counting.
            let edit = host.begin_tab(
                21,
                &["minecraft:stone", "minecraft:dirt"],
                &["minecraft:stone", "minecraft:dirt"],
            );
            mod_api::dispatch_build_creative_mode_tab_contents(
                HostSide::Client(&mut host),
                "minecraft:functional_blocks",
                vec!["minecraft:vanilla".to_owned()],
                CreativeTabPermissions::Normal,
                edit,
            );
            assert_eq!(
                host.tab_entries(CreativeTabList::Parent),
                vec![
                    ("minecraft:stone".to_owned(), 1),
                    ("minecraft:dirt".to_owned(), 1),
                    ("demo:after_custom".to_owned(), 1),
                    ("demo:all_tab_extra".to_owned(), 1),
                ]
            );
            // Only the 2 conversion counts for `demo:not_in_sunrise` (2 -> 4); zero duplicate errors.
            assert_eq!(report_row(&java)["unknown_host_call_count"], 4);

            // `ItemStack` anchor against damaged stone in Combat must count as unknown instead of
            // falsely matching components.
            let edit = mod_api::__private::creative_tab_edit(22);
            {
                let mut editor = host.editor.borrow_mut();
                editor.active = Some(edit);
                editor.parent = vec![damaged_stone.clone()];
                editor.search = vec![damaged_stone];
            }
            mod_api::dispatch_build_creative_mode_tab_contents(
                HostSide::Client(&mut host),
                "minecraft:combat",
                vec!["minecraft:vanilla".to_owned()],
                CreativeTabPermissions::Normal,
                edit,
            );
            assert!(
                report_row(&java)["unknown_host_call_count"]
                    .as_u64()
                    .unwrap()
                    > 4,
                "ItemStack anchor matching custom-component stack must be counted"
            );

            // Direct boundary check on `convert_tooltip_component`: non-component Value is refused.
            let runtime_host = crate::runtime::StdHost::new();
            let (text, refused) = crate::client::convert_tooltip_component(
                &host,
                &runtime_host,
                &crate::runtime::Value::Int(7),
            );
            assert_eq!(text, None);
            assert!(refused);

            Ok::<_, ()>(())
        })
        .unwrap();
    });
}
