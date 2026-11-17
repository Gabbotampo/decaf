//! The census file backend is per jar host state: a callback writes, a later
//! callback reads and stats, and a fresh host starts with an empty sandbox.
//!
//! Java 25 oracle values (Temurin 25.0.5+8, with the sandbox cwd represented
//! here as /horizon/game): write -> "/horizon/game/audit.txt|2",
//! readAndStat -> "alpha|5|2|/horizon/game/audit.txt", fresh -> "missing".

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::{Host, StdHost};
use javac_fixture::Fixture;
use std::collections::{BTreeMap, BTreeSet};

const JAVA25_CAPTURE: &str = include_str!("fixtures/files_vfs/java25.txt");

const PROBE: &str = r#"package demo;

import java.io.File;
import java.io.FileWriter;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;

public final class Probe {
    public static String write() {
        try {
            Files.writeString(Path.of("audit.txt"), "alpha");
            Files.writeString(Path.of("é.txt"), "é");
            return new File("audit.txt").getAbsolutePath() + "|" + new File("é.txt").length();
        } catch (IOException error) {
            return "write-failed";
        }
    }

    public static String readAndStat() {
        try {
            String text = Files.readString(Path.of("audit.txt"));
            File file = new File("audit.txt");
            File unicode = new File("é.txt");
            return text + "|" + file.length() + "|" + unicode.length() + "|"
                + file.getAbsolutePath();
        } catch (IOException error) {
            return "read-failed";
        }
    }

    public static String missing() {
        try {
            return Files.readString(Path.of("audit.txt"));
        } catch (IOException error) {
            return "missing";
        }
    }

    public static String canonical() {
        try {
            Files.writeString(Path.of("a/../b"), "x");
            Path path = Path.of("a/../b");
            return new File("a/../b").getAbsolutePath() + "|"
                + new File("a/../b").getCanonicalPath() + "|"
                + path.toAbsolutePath() + "|" + path.toRealPath() + "|"
                + new File("./a//../b").getCanonicalPath() + "|"
                + Files.readString(Path.of("b"));
        } catch (IOException error) {
            return "canonical-failed";
        }
    }

    public static String openNew() {
        try {
            FileWriter writer = new FileWriter("open.txt");
            writer.write("abc");
            return new File("open.txt").exists() + "|" + new File("open.txt").length();
        } catch (IOException error) {
            return "open-failed";
        }
    }

    public static String readOpen() {
        try {
            return Files.readString(Path.of("open.txt")) + "|" + new File("open.txt").length();
        } catch (IOException error) {
            return "read-open-failed";
        }
    }

    public static String seedExisting() {
        try {
            Files.writeString(Path.of("old.txt"), "old");
            return "seeded";
        } catch (IOException error) {
            return "seed-failed";
        }
    }

    public static String openExisting() {
        try {
            FileWriter writer = new FileWriter("old.txt");
            writer.write("new");
            return "opened";
        } catch (IOException error) {
            return "open-existing-failed";
        }
    }

    public static String readExisting() {
        try {
            return Files.readString(Path.of("old.txt"));
        } catch (IOException error) {
            return "read-existing-failed";
        }
    }
}
"#;

#[test]
fn virtual_files_persist_between_callbacks_but_not_fresh_hosts() {
    let fixture = Fixture::build("file-model-persistent", &[("demo/Probe.java", PROBE)]);
    let mut host = fixture.jar.seed_host(StdHost::new());
    assert_eq!(
        fixture.text_on(&mut host, "write"),
        "/horizon/game/audit.txt|2"
    );
    assert_eq!(
        fixture.text_on(&mut host, "readAndStat"),
        "alpha|5|2|/horizon/game/audit.txt"
    );

    let mut fresh = fixture.jar.seed_host(StdHost::new());
    assert_eq!(fixture.text_on(&mut fresh, "missing"), "missing");
}

#[test]
fn canonical_paths_are_lexical_and_unclosed_writers_open_files() {
    let fixture = Fixture::build(
        "file-model-persistent-canonical",
        &[("demo/Probe.java", PROBE)],
    );
    let mut host = fixture.jar.seed_host(StdHost::new());
    assert_eq!(
        fixture.text_on(&mut host, "canonical"),
        "/horizon/game/a/../b|/horizon/game/b|/horizon/game/a/../b|/horizon/game/b|/horizon/game/b|x"
    );
    assert_eq!(fixture.text_on(&mut host, "openNew"), "true|0");
    assert_eq!(fixture.text_on(&mut host, "readOpen"), "|0");
    assert_eq!(fixture.text_on(&mut host, "seedExisting"), "seeded");
    assert_eq!(fixture.text_on(&mut host, "openExisting"), "opened");
    assert_eq!(fixture.text_on(&mut host, "readExisting"), "");
}

#[test]
fn persistent_file_state_has_entry_and_byte_bounds() {
    let mut host = StdHost::new();
    let mut files = BTreeMap::new();
    for index in 0..4097 {
        files.insert(format!("file-{index}"), String::new());
    }
    let error = host
        .store_persistent_io_state(BTreeMap::new(), files, BTreeMap::new(), BTreeSet::new())
        .unwrap_err();
    assert!(error.to_string().contains("entries=4097"));

    let mut files = BTreeMap::new();
    files.insert("large".into(), "x".repeat(16 * 1024 * 1024 + 1));
    let error = host
        .store_persistent_io_state(BTreeMap::new(), files, BTreeMap::new(), BTreeSet::new())
        .unwrap_err();
    assert!(error.to_string().contains("bytes="));
}
