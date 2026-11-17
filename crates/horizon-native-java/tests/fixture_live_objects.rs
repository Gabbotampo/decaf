#[path = "support/javac_fixture.rs"]
mod javac_fixture;
use horizon_native_java::{HostRef, StdHost, Value};
use javac_fixture::Fixture;
const PROBE: &str = r#"package demo; public final class Probe { public static String rows(net.minecraft.server.level.ServerPlayer p, net.minecraft.world.inventory.AbstractContainerMenu m){ return (p instanceof net.minecraft.world.entity.player.Player)+"|"+(p instanceof net.minecraft.world.entity.LivingEntity)+"|"+(p instanceof net.minecraft.world.entity.Entity)+"|"+(p==p)+"|"+p.isRemoved()+"|"+p.hashCode()+"|"+p.getClass()+"|"+p.getX(); } public static boolean removed(net.minecraft.server.level.ServerPlayer p){ return p.isRemoved(); } public static void write(net.minecraft.server.level.ServerPlayer p){ p.setPos(1,2,3); }}"#;
#[test]
fn live_object_rows() {
    let f = Fixture::build("live-objects", &[("demo/Probe.java", PROBE)]);
    let mut h = StdHost::new();
    let key = "ServerPlayer:1";
    h.live_object_issued(key);
    let p = Value::Host(HostRef::live_game_object_for_test(key));
    let m = Value::Host(HostRef::live_game_object_for_test(
        "AbstractContainerMenu:1",
    ));
    let out=f.jar.invoke_method_with_args("demo/Probe","rows","(Lnet/minecraft/server/level/ServerPlayer;Lnet/minecraft/world/inventory/AbstractContainerMenu;)Ljava/lang/String;",None,&[p.clone(),m],&mut h).unwrap();
    let chain = include_str!("fixtures/live_objects/javap_facts.txt");
    let expected = format!(
        "{}|{}|{}|true|false",
        chain.contains("player.Player"),
        chain.contains("LivingEntity"),
        chain.contains("Entity")
    );
    assert_eq!(out, Value::Ref(Some(expected)));
    h.live_object_removed(key, "KILLED");
    let removed = f
        .jar
        .invoke_method_with_args(
            "demo/Probe",
            "removed",
            "(Lnet/minecraft/server/level/ServerPlayer;)Z",
            None,
            &[p],
            &mut h,
        )
        .unwrap();
    assert_eq!(removed, Value::Int(1));
    let forged = Value::Host(HostRef::live_game_object_for_test("ServerPlayer:forged"));
    let _ = f.jar.invoke_method_with_args(
        "demo/Probe",
        "removed",
        "(Lnet/minecraft/server/level/ServerPlayer;)Z",
        None,
        &[forged],
        &mut h,
    );
    let mut calls = h.unknown_host_calls();
    calls.sort();
    assert!(
        calls
            .iter()
            .any(|(k, n)| k == "java/lang/Object#hashCode()I" && *n == 1)
    );
    assert!(
        calls
            .iter()
            .any(|(k, n)| k == "java/lang/Object#getClass()Ljava/lang/Class;" && *n == 1)
    );
    assert!(
        calls
            .iter()
            .any(|(k, n)| k.ends_with("Entity#getX()D") && *n == 1)
    );
    assert!(
        calls
            .iter()
            .any(|(k, n)| k.ends_with("Entity#setPos(DDD)V") && *n == 1)
    );
    assert!(
        calls
            .iter()
            .any(|(k, n)| k.ends_with("Entity#isRemoved()Z") && *n == 1)
    );
}
