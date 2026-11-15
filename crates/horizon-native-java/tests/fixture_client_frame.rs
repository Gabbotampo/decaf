//! C4 frame reads against the real 26.3 client jar.
#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::{
    StdHost,
    client_frame::{ClientHitResult, ClientOptions},
    client_surface::{ClientPlayer, ClientSnapshot, KeyPress},
};
use javac_fixture::Fixture;
use std::sync::Arc;

const PROBE: &str = r#"
package demo;
import net.minecraft.client.Minecraft;
import net.minecraft.world.phys.Vec3;
import net.minecraft.world.phys.HitResult;
import net.minecraft.core.BlockPos;
public final class Probe {
  public static String run() {
    Minecraft mc = Minecraft.getInstance();
    Vec3 a = mc.player.position();
    Vec3 b = a.add(-5.25, 7.5, -3.5);
    Vec3 n = a.normalize();
    HitResult h = mc.hitResult;
    BlockPos p = mc.player.blockPosition();
    return a.add(b)+"|"+a.subtract(1.5,2.5,-3.5)+"|"+a.scale(-2.0)
      +"|"+a.length()+"|"+a.lengthSqr()+"|"+n+"|"+a.distanceTo(b)
      +"|"+a.cross(b)+"|"+a.dot(b)+"|"+p
      +"|"+h.getType()+"|"+h.getLocation()
      +"|"+mc.options.particles().get()+"|"+mc.options.sensitivity().get()
      +"|"+mc.getWindow().getGuiScale();
  }
}
"#;

#[test]
fn real_client_frame_reads_match_java25_oracle() {
    let Ok(client_jar) = std::env::var("MINECRAFT_CLIENT_JAR") else {
        eprintln!("skipped: MINECRAFT_CLIENT_JAR not set");
        return;
    };
    let fixture =
        Fixture::build_with_classpath("client-frame", &[("demo/Probe.java", PROBE)], &client_jar);
    let mut host = fixture.jar.seed_host(StdHost::new());
    let snapshot = Arc::new(ClientSnapshot {
        player: Some(ClientPlayer {
            pos: [1.25, -2.5, 3.75],
            health: 20.0,
            max_health: 20.0,
            using_item: Some(false),
            sleeping: Some(false),
            ..ClientPlayer::default()
        }),
        options: ClientOptions {
            particles: Some("ALL".into()),
            sensitivity: Some(0.5),
        },
        hit_result: Some(ClientHitResult::Miss {
            location: [1.25, -2.5, 3.75],
        }),
        gui_scale: 2.0,
        ..ClientSnapshot::default()
    });
    host.begin_client_frame(snapshot, &[KeyPress::default()]);
    let printed = fixture.text_on(&mut host, "run");
    host.end_client_frame();
    assert_eq!(
        printed,
        "(-2.75, 2.5, 4.0)|(-0.25, -5.0, 7.25)|(-2.5, 5.0, -7.5)|4.677071733467427|21.875|(0.2672612419124244, -0.5345224838248488, 0.8017837257372731)|9.80114789195633|(-19.375, -15.3125, -3.75)|-16.5625|BlockPos{x=1, y=-3, z=3}|MISS|(1.25, -2.5, 3.75)|ALL|0.5|2"
    );
    assert_eq!(host.unknown_host_calls(), Vec::<(String, usize)>::new());
}
