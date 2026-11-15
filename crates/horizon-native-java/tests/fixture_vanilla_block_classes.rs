//! A vanilla block constant answers `instanceof` for its real class when the
//! game seeds the host with the prepared block class chains: a jar's
//! `Blocks.FIRE instanceof FireBlock` is true, and false for another class.

use horizon_native_java::{NativeJar, StdHost, Value};
use std::{collections::BTreeMap, fs, io::Write, path::Path, process::Command};
use zip::write::SimpleFileOptions;

fn javac() -> std::path::PathBuf {
    std::env::var_os("JAVA_HOME")
        .map(|home| std::path::PathBuf::from(home).join("bin/javac"))
        .unwrap_or_else(|| "javac".into())
}

fn files_under(root: &Path) -> Vec<std::path::PathBuf> {
    fs::read_dir(root)
        .unwrap()
        .flat_map(|entry| {
            let path = entry.unwrap().path();
            if path.is_dir() {
                files_under(&path)
            } else {
                vec![path]
            }
        })
        .collect()
}

fn build_jar(
    name: &str,
    sources: BTreeMap<&str, &str>,
    resources: &[(&str, &str)],
    excluded: &[&str],
) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!(
        "horizon-native-java-vanilla-block-classes-{name}-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&root);
    let src = root.join("src");
    let classes = root.join("classes");
    fs::create_dir_all(&classes).unwrap();
    let mut paths = Vec::new();
    for (relative, body) in sources {
        let path = src.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&path, body).unwrap();
        paths.push(path);
    }
    let status = Command::new(javac())
        .arg("--release")
        .arg("25")
        .arg("-d")
        .arg(&classes)
        .args(paths)
        .status()
        .unwrap();
    assert!(status.success(), "javac failed for {name}");

    let jar_path = root.join(format!("{name}.jar"));
    let mut zip = zip::ZipWriter::new(fs::File::create(&jar_path).unwrap());
    for (path, body) in resources {
        zip.start_file(*path, SimpleFileOptions::default()).unwrap();
        zip.write_all(body.as_bytes()).unwrap();
    }
    for class in files_under(&classes) {
        let entry = class
            .strip_prefix(&classes)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        if excluded.contains(&entry.as_str()) {
            continue;
        }
        zip.start_file(entry, SimpleFileOptions::default()).unwrap();
        zip.write_all(&fs::read(class).unwrap()).unwrap();
    }
    zip.finish().unwrap();
    jar_path
}

#[test]
fn vanilla_block_constants_answer_instanceof_from_seeded_classes() {
    let mut sources = BTreeMap::new();
    sources.insert(
        "demo/Probe.java",
        "package demo; public final class Probe { public static void check(String label, boolean value) {} }",
    );
    sources.insert(
        "net/minecraft/world/level/block/Block.java",
        "package net.minecraft.world.level.block; public class Block extends net.minecraft.world.level.block.state.BlockBehaviour implements net.minecraft.world.level.ItemLike {}",
    );
    sources.insert(
        "net/minecraft/world/level/block/BaseFireBlock.java",
        "package net.minecraft.world.level.block; public abstract class BaseFireBlock extends Block {}",
    );
    sources.insert(
        "net/minecraft/world/level/block/FireBlock.java",
        "package net.minecraft.world.level.block; public class FireBlock extends BaseFireBlock {}",
    );
    sources.insert(
        "net/minecraft/world/level/block/ChestBlock.java",
        "package net.minecraft.world.level.block; public class ChestBlock extends Block {}",
    );
    sources.insert(
        "net/minecraft/world/level/block/BucketPickup.java",
        "package net.minecraft.world.level.block; public interface BucketPickup {}",
    );
    sources.insert(
        "net/minecraft/world/level/block/LiquidBlockContainer.java",
        "package net.minecraft.world.level.block; public interface LiquidBlockContainer {}",
    );
    sources.insert(
        "net/minecraft/world/level/block/ChangeOverTimeBlock.java",
        "package net.minecraft.world.level.block; public interface ChangeOverTimeBlock {}",
    );
    sources.insert(
        "net/minecraft/world/level/block/SimpleWaterloggedBlock.java",
        "package net.minecraft.world.level.block; public interface SimpleWaterloggedBlock extends BucketPickup, LiquidBlockContainer {}",
    );
    sources.insert(
        "net/minecraft/world/level/block/LiquidBlock.java",
        "package net.minecraft.world.level.block; public class LiquidBlock extends Block implements BucketPickup, LiquidBlockContainer {}",
    );
    sources.insert(
        "net/minecraft/world/level/block/OakFenceBlock.java",
        "package net.minecraft.world.level.block; public class OakFenceBlock extends Block implements SimpleWaterloggedBlock {}",
    );
    sources.insert(
        "demo/jar/FireBlock.java",
        "package demo.jar; public interface FireBlock {}",
    );
    sources.insert(
        "demo/jar/BucketPickup.java",
        "package demo.jar; public interface BucketPickup {}",
    );
    sources.insert(
        "net/minecraft/world/level/block/Blocks.java",
        "package net.minecraft.world.level.block; public final class Blocks { public static final Block FIRE = null; public static final Block WATER = null; public static final Block OAK_FENCE = null; public static final Block CHEST = null; }",
    );
    sources.insert(
        "net/minecraft/world/level/block/state/BlockBehaviour.java",
        "package net.minecraft.world.level.block.state; public class BlockBehaviour {}",
    );
    sources.insert(
        "net/minecraft/world/level/ItemLike.java",
        "package net.minecraft.world.level; public interface ItemLike {}",
    );
    sources.insert(
        "demo/Main.java",
        r#"package demo;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.level.block.BucketPickup;
import net.minecraft.world.level.block.ChangeOverTimeBlock;
import net.minecraft.world.level.block.ChestBlock;
import net.minecraft.world.level.block.FireBlock;
import net.minecraft.world.level.block.LiquidBlock;
import net.minecraft.world.level.block.LiquidBlockContainer;
import net.minecraft.world.level.block.OakFenceBlock;
public final class Main {
    public static void main() {
        Probe.check("fire", Blocks.FIRE instanceof FireBlock);
        Probe.check("not-chest", !(Blocks.FIRE instanceof ChestBlock));
        Probe.check("block-behaviour", Blocks.FIRE instanceof net.minecraft.world.level.block.state.BlockBehaviour);
        Probe.check("item-like", Blocks.FIRE instanceof net.minecraft.world.level.ItemLike);
        Probe.check("water", Blocks.WATER instanceof LiquidBlock);
        Probe.check("bucket-pickup", Blocks.WATER instanceof BucketPickup);
        Probe.check("liquid-container", Blocks.WATER instanceof LiquidBlockContainer);
        Probe.check("fence-bucket", Blocks.OAK_FENCE instanceof BucketPickup);
        Probe.check("fence-liquid", Blocks.OAK_FENCE instanceof LiquidBlockContainer);
        Probe.check("fire-no-change", !(Blocks.FIRE instanceof ChangeOverTimeBlock));
        Probe.check("fence-no-change", !(Blocks.OAK_FENCE instanceof ChangeOverTimeBlock));
        Probe.check("jar-fire-name", !(Blocks.FIRE instanceof demo.jar.FireBlock));
        Probe.check("jar-bucket-name", !(Blocks.WATER instanceof demo.jar.BucketPickup));
        Probe.check("same-block", Blocks.FIRE == Blocks.FIRE);
        Probe.check("different-block", Blocks.FIRE != Blocks.CHEST);
    }
}
"#,
    );
    let jar_path = build_jar(
        "vanilla-block-classes",
        sources,
        &[],
        &[
            "demo/Probe.class",
            "net/minecraft/world/level/block/Block.class",
            "net/minecraft/world/level/block/BaseFireBlock.class",
            "net/minecraft/world/level/block/FireBlock.class",
            "net/minecraft/world/level/block/BucketPickup.class",
            "net/minecraft/world/level/block/LiquidBlockContainer.class",
            "net/minecraft/world/level/block/ChangeOverTimeBlock.class",
            "net/minecraft/world/level/block/SimpleWaterloggedBlock.class",
            "net/minecraft/world/level/block/LiquidBlock.class",
            "net/minecraft/world/level/block/OakFenceBlock.class",
            "net/minecraft/world/level/block/state/BlockBehaviour.class",
            "net/minecraft/world/level/ItemLike.class",
            "net/minecraft/world/level/block/ChestBlock.class",
            "net/minecraft/world/level/block/Blocks.class",
            "demo/jar/FireBlock.class",
            "demo/jar/BucketPickup.class",
        ],
    );
    let jar = NativeJar::from_path(&jar_path).unwrap();
    let table = jar_path
        .parent()
        .unwrap()
        .join("vanilla-block-classes.json");
    fs::write(
        &table,
        r#"{"blocks":{
            "minecraft:fire":["net/minecraft/world/level/block/FireBlock","net/minecraft/world/level/block/BaseFireBlock","net/minecraft/world/level/block/Block","net/minecraft/world/level/block/state/BlockBehaviour","net/minecraft/world/level/ItemLike","java/lang/Object"],
            "minecraft:water":["net/minecraft/world/level/block/LiquidBlock","net/minecraft/world/level/block/BucketPickup","net/minecraft/world/level/block/LiquidBlockContainer","net/minecraft/world/level/block/Block","net/minecraft/world/level/block/state/BlockBehaviour","net/minecraft/world/level/ItemLike","java/lang/Object"],
            "minecraft:oak_fence":["net/minecraft/world/level/block/OakFenceBlock","net/minecraft/world/level/block/SimpleWaterloggedBlock","net/minecraft/world/level/block/BucketPickup","net/minecraft/world/level/block/LiquidBlockContainer","net/minecraft/world/level/block/Block","net/minecraft/world/level/block/state/BlockBehaviour","net/minecraft/world/level/ItemLike","java/lang/Object"]
        }}"#,
    )
    .unwrap();
    let mut host = jar.seed_host(StdHost::new().with_game_block_classes_file(&table).unwrap());
    jar.execute("demo.Main", Some("main"), "main", &mut host)
        .unwrap();
    let checks: BTreeMap<String, bool> = host
        .invocations()
        .iter()
        .filter(|call| call.method.owner == "demo/Probe" && call.method.name == "check")
        .filter_map(|call| match call.args.as_slice() {
            [Value::Ref(Some(label)), Value::Int(flag)] => Some((label.clone(), *flag != 0)),
            _ => None,
        })
        .collect();
    assert_eq!(
        checks,
        BTreeMap::from([
            ("fire".to_owned(), true),
            ("not-chest".to_owned(), true),
            ("block-behaviour".to_owned(), true),
            ("item-like".to_owned(), true),
            ("same-block".to_owned(), true),
            ("water".to_owned(), true),
            ("bucket-pickup".to_owned(), true),
            ("liquid-container".to_owned(), true),
            ("fence-bucket".to_owned(), true),
            ("fence-liquid".to_owned(), true),
            ("fire-no-change".to_owned(), true),
            ("fence-no-change".to_owned(), true),
            ("jar-fire-name".to_owned(), true),
            ("jar-bucket-name".to_owned(), true),
            ("different-block".to_owned(), true),
        ])
    );
}
