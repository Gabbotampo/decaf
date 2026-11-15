//! The census file backend is per jar host state: a callback writes, a later
//! callback reads and stats, and a fresh host starts with an empty sandbox.
//!
//! Java 25 oracle values (Temurin 25.0.5+8, with the sandbox cwd represented
//! here as /horizon/game): write -> "/horizon/game/audit.txt|2",
//! readAndStat -> "alpha|5|2|/horizon/game/audit.txt", fresh -> "missing".

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::StdHost;
use javac_fixture::Fixture;

const PROBE: &str = r#"package demo;

import java.io.File;
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
