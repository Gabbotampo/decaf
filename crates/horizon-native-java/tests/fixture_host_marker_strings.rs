#[path = "support/javac_fixture.rs"] mod javac_fixture;
const PROBE: &str = include_str!("fixtures/host_marker_strings/demo/Probe.java");
const ORACLE: &str = include_str!("fixtures/host_marker_strings/java25.txt");
fn shown(v: Result<horizon_native_java::Value, horizon_native_java::NativeError>) -> String { match v { Ok(horizon_native_java::Value::Ref(Some(s))) => s, x => format!("{x:?}"), } }
#[test]
fn marker_spelling_is_string_and_callback_collections_count_to_string() {
 let f=javac_fixture::Fixture::build("host-marker-strings", &[("demo/Probe.java", PROBE)]);
 let mut h=horizon_native_java::StdHost::new();
 let s=shown(f.jar.invoke_method_with_args("demo/Probe","strings","()Ljava/lang/String;",None,&[],&mut h));
 assert_eq!(s, ORACLE.lines().take(6).collect::<Vec<_>>().join("\n"));
 assert!(h.unknown_host_calls().is_empty());
 let mut h=horizon_native_java::StdHost::new();
 let _=shown(f.jar.invoke_method_with_args("demo/Probe","collections","()Ljava/lang/String;",None,&[],&mut h));
 assert_eq!(h.unknown_host_calls(), vec![("java/lang/Object#toString()Ljava/lang/String;".into(), 4)]);
}
