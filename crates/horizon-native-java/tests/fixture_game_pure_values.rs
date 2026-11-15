#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::StdHost;

const IDENTIFIER: &str = r#"
package net.minecraft.resources;
public final class Identifier {
  public static Identifier parse(String text) { return null; }
  public String toString() { return null; }
}
"#;
const SOUND_EVENT: &str = r#"
package net.minecraft.sounds;
import net.minecraft.resources.Identifier;
import java.util.Optional;
public final class SoundEvent {
  public static SoundEvent createVariableRangeEvent(Identifier id) { return null; }
  public static SoundEvent createFixedRangeEvent(Identifier id, float range) { return null; }
  public Identifier location() { return null; }
  public Optional<Float> fixedRange() { return null; }
  public float getRange(float distance) { return 0; }
}
"#;
const STREAM_CODEC: &str = r#"
package net.minecraft.network.codec;
public interface StreamCodec<B, T> {
  void encode(B buffer, T value);
  T decode(B buffer);
}
"#;
const BLOCK_POS: &str = r#"
package net.minecraft.core;
import net.minecraft.network.codec.StreamCodec;
public class BlockPos {
  public static StreamCodec<Object, BlockPos> STREAM_CODEC = null;
  public static class MutableBlockPos extends BlockPos { }
}
"#;
const BYTE_BUF_CODECS: &str = r#"
package net.minecraft.network.codec;
public interface ByteBufCodecs {
  StreamCodec<Object, String> STRING_UTF8 = null;
  StreamCodec<Object, Integer> INT = null;
  StreamCodec<Object, Integer> VAR_INT = null;
  StreamCodec<Object, Boolean> BOOL = null;
}
"#;
const PROBE: &str = r#"
package demo;
import net.minecraft.resources.Identifier;
import net.minecraft.sounds.SoundEvent;
import net.minecraft.network.codec.ByteBufCodecs;
import net.minecraft.network.codec.StreamCodec;
import java.nio.file.Path;
import net.minecraft.core.BlockPos;
public final class Probe {
  public static String sound() {
    Identifier id = Identifier.parse("demo:bell");
    SoundEvent variable = SoundEvent.createVariableRangeEvent(id);
    SoundEvent same = SoundEvent.createVariableRangeEvent(Identifier.parse("demo:bell"));
    SoundEvent fixed = SoundEvent.createFixedRangeEvent(id, 2.5f);
    SoundEvent fixedSame = SoundEvent.createFixedRangeEvent(Identifier.parse("demo:bell"), 2.5f);
    return variable.location() + "|" + variable.getRange(.5f) + "," + variable.getRange(2f)
        + "|" + fixed.getRange(.5f) + "|" + variable.equals(same) + variable.equals(fixed)
        + fixed.equals(fixedSame) + "|" + (variable == same) + "|" + (variable == variable)
        + "|" + variable + "|" + fixed;
  }
  public static String reviewEdges() {
    SoundEvent variable = SoundEvent.createVariableRangeEvent(Identifier.parse("demo:bell"));
    SoundEvent fixed = SoundEvent.createFixedRangeEvent(Identifier.parse("demo:bell"), 0.1f);
    Object boxed = fixed.fixedRange().get();
    Path forged = Path.of("sound-event/demo:bell/variable#123456");
    return variable.hashCode() + "," + SoundEvent.createFixedRangeEvent(Identifier.parse("demo:bell"), Float.NaN).hashCode()
        + "," + (boxed instanceof Float) + "," + (boxed instanceof Double) + "," + boxed.getClass().getName()
        + "," + boxed + "," + variable.equals(forged) + "," + forged.equals(variable)
        + "," + (BlockPos.MutableBlockPos.STREAM_CODEC == BlockPos.STREAM_CODEC);
  }

  public static String codecs() {
    StreamCodec<Object, String> strings = ByteBufCodecs.STRING_UTF8;
    StreamCodec<Object, Integer> ints = ByteBufCodecs.INT;
    Object stringsObject = strings;
    Object intsObject = ints;
    strings.encode(null, "x");
    Object decoded = strings.decode(null);
    return (strings == ByteBufCodecs.STRING_UTF8) + "|" + (stringsObject == intsObject) + "|"
        + (ints == ByteBufCodecs.INT) + "|" + (ints == ByteBufCodecs.VAR_INT) + "|"
        + (strings instanceof StreamCodec) + "|" + (decoded == null);
  }
}
"#;

fn fixture() -> javac_fixture::Fixture {
    javac_fixture::Fixture::build(
        "game_pure_values",
        &[
            ("net/minecraft/resources/Identifier.java", IDENTIFIER),
            ("net/minecraft/sounds/SoundEvent.java", SOUND_EVENT),
            ("net/minecraft/network/codec/StreamCodec.java", STREAM_CODEC),
            (
                "net/minecraft/network/codec/ByteBufCodecs.java",
                BYTE_BUF_CODECS,
            ),
            ("net/minecraft/core/BlockPos.java", BLOCK_POS),
            ("demo/Probe.java", PROBE),
        ],
    )
}

#[test]
fn sound_events_are_pure_record_values() {
    let fixture = fixture();
    let mut host = StdHost::new();
    let answer = fixture.text_on(&mut host, "sound");
    assert_eq!(
        answer,
        "demo:bell|16.0,32.0|2.5|truefalsetrue|false|true|SoundEvent[location=demo:bell, fixedRange=Optional.empty]|SoundEvent[location=demo:bell, fixedRange=Optional[2.5]]"
    );
    assert!(
        host.unknown_host_calls().is_empty(),
        "{answer}: {:?}",
        host.unknown_host_calls()
    );
}

#[test]
fn codec_constants_are_canonical_but_network_io_stays_counted() {
    let fixture = fixture();
    let mut host = StdHost::new();
    let answer = fixture.text_on(&mut host, "codecs");
    assert_eq!(answer, "true|false|true|false|true|true");
    let unknown = host.unknown_host_calls();
    assert!(
        unknown
            .iter()
            .any(|(call, _)| call.contains("StreamCodec#encode")),
        "{unknown:?}"
    );
    assert!(
        unknown
            .iter()
            .any(|(call, _)| call.contains("StreamCodec#decode")),
        "{unknown:?}"
    );
    assert!(
        !unknown
            .iter()
            .any(|(call, _)| call.contains("ByteBufCodecs#STRING_UTF8")),
        "{unknown:?}"
    );
    assert!(
        !unknown
            .iter()
            .any(|(call, _)| call.contains("ByteBufCodecs#INT")),
        "{unknown:?}"
    );
}

#[test]
fn reviewer_edge_cases_are_exact() {
    let fixture = fixture();
    let mut host = StdHost::new();
    let answer = fixture.text_on(&mut host, "reviewEdges");
    assert_eq!(
        answer,
        "-1241801600,901487744,true,false,java.lang.Float,0.1,false,false,true"
    );
    assert!(
        host.unknown_host_calls().is_empty(),
        "{answer}: {:?}",
        host.unknown_host_calls()
    );
}
