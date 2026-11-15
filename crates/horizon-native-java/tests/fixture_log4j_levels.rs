//! Log4j 2.26.0 Level fixture compiled against the real shipped API jar.
#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use javac_fixture::Fixture;

const PROBE: &str = r#"
package demo;
import org.apache.logging.log4j.Level;
public final class Probe {
  public static String levels() {
    Level info=Level.INFO, error=Level.ERROR;
    String forged="l4lvl:INFO";
    String strict; try { Level.valueOf("missing"); strict="no"; } catch (IllegalArgumentException e) { strict="yes"; }
    Level custom=Level.forName("custom",350), repeat=Level.forName(" CUSTOM ",1);
    StringBuilder order=new StringBuilder(); for(Level level:Level.values()) order.append(level.name()).append(":").append(level.intLevel()).append(",");
    return (info==Level.valueOf(" info "))+"|"+info.name()+"|"+info.intLevel()+"|"+info.toString()
      +"|"+error.isMoreSpecificThan(info)+"|"+info.isLessSpecificThan(error)
      +"|"+error.isInRange(Level.ERROR,Level.DEBUG)+"|"+error.compareTo(info)
      +"|"+(Level.getLevel("missing")==null)+"|"+Level.toLevel("missing").toString()
      +"|"+Level.toLevel(null,Level.WARN).toString()+"|"+strict
      +"|"+custom.name()+"|"+custom.intLevel()+"|"+(custom==repeat)+"|"+(Level.valueOf("custom")==custom)
      +"|"+order+"|"+forged.getClass().getName()+"|"+forged.equals("l4lvl:INFO");
  }
}
"#;

#[test]
fn real_log4j_level_surface_matches_java25_oracle() {
    // Java 25.0.5 + Log4j API 2.26.0 output:
    // true|INFO|400|INFO|true|true|true|-1|true|DEBUG|WARN|yes|custom|350|true|true|TRACE:600,ALL:2147483647,custom:350,ERROR:200,INFO:400,FATAL:100,DEBUG:500,OFF:0,WARN:300,|java.lang.String|true
    let api_jar = std::env::var("LOG4J_API_JAR")
        .expect("LOG4J_API_JAR must point to Minecraft 26.3 log4j-api-2.26.0.jar");
    assert!(
        std::path::Path::new(&api_jar).is_file(),
        "missing real Log4j API jar: {api_jar}"
    );
    let fixture =
        Fixture::build_with_classpath("log4j-levels", &[("demo/Probe.java", PROBE)], &api_jar);
    assert_eq!(
        fixture.text("levels"),
        "true|INFO|400|INFO|true|true|true|-1|true|DEBUG|WARN|yes|custom|350|true|true|TRACE:600,ALL:2147483647,custom:350,ERROR:200,INFO:400,FATAL:100,DEBUG:500,OFF:0,WARN:300,|java.lang.String|true"
    );
}
