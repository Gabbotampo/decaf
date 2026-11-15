//! Log4j 2.26.0 Level fixture. The source is compiled with javac --release 21
//! against the real Log4j API jar on arcweld; only demo.Probe ships in the jar.
#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use javac_fixture::Fixture;

const LEVEL: &str = r#"
package org.apache.logging.log4j;
public final class Level implements Comparable<Level> {
  public static final Level OFF=null,FATAL=null,ERROR=null,WARN=null,INFO=null,DEBUG=null,TRACE=null,ALL=null;
  public int intLevel(){return 0;} public String name(){return null;}
  public boolean isMoreSpecificThan(Level x){return false;}
  public boolean isLessSpecificThan(Level x){return false;}
  public boolean isInRange(Level a,Level b){return false;}
  public int compareTo(Level x){return 0;}
  public static Level getLevel(String x){return null;} public static Level valueOf(String x){return null;}
  public static Level toLevel(String x){return null;} public static Level toLevel(String x,Level d){return null;}
}
"#;
const PROBE: &str = r#"
package demo;
import org.apache.logging.log4j.Level;
public final class Probe {
  public static String levels() {
    Level info=Level.INFO, error=Level.ERROR;
    String forged="l4lvl:INFO";
    String strict; try { Level.valueOf("missing"); strict="no"; } catch (IllegalArgumentException e) { strict="yes"; }
    return (info==Level.valueOf(" info "))+"|"+info.name()+"|"+info.intLevel()+"|"+info.toString()
      +"|"+error.isMoreSpecificThan(info)+"|"+info.isLessSpecificThan(error)
      +"|"+error.isInRange(Level.ERROR,Level.DEBUG)+"|"+error.compareTo(info)
      +"|"+(Level.getLevel("missing")==null)+"|"+Level.toLevel("missing").toString()
      +"|"+Level.toLevel(null,Level.WARN).toString()+"|"+strict
      +"|"+forged.getClass().getName()+"|"+forged.equals("l4lvl:INFO");
  }
}
"#;

#[test]
fn real_log4j_level_surface_matches_java25_oracle() {
    // Java 25.0.5 + Log4j API 2.26.0 output:
    // true|INFO|400|INFO|true|true|true|-1|true|DEBUG|WARN|yes|java.lang.String|true
    let fixture = Fixture::build(
        "log4j-levels",
        &[
            ("org/apache/logging/log4j/Level.java", LEVEL),
            ("demo/Probe.java", PROBE),
        ],
    );
    assert_eq!(
        fixture.text("levels"),
        "true|INFO|400|INFO|true|true|true|-1|true|DEBUG|WARN|yes|java.lang.String|true"
    );
}
