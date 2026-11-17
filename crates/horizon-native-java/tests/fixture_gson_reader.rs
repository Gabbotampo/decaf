#[path = "support/javac_fixture.rs"]
mod javac_fixture;
use horizon_native_java::{StdHost, Value};
const PROBE: &str = include_str!("fixtures/gson_reader/demo/Probe.java");
const ORACLE: &str = include_str!("fixtures/gson_reader/java25.txt");
const COUNTED: &str = include_str!("fixtures/gson_reader/java25_counted.txt");
const RESOURCE: &[u8] = include_bytes!("fixtures/gson_reader/resource-config.json");
#[test]
fn gson_reader_matches_java25() {
 let fixture=javac_fixture::Fixture::build_with_lib("gson-reader", &[("demo/Probe.java",PROBE)], &[("resource-config.json",RESOURCE)]);
 let mut host=fixture.jar.seed_host(StdHost::new());
 let answer=fixture.jar.invoke_method_with_args("demo/Probe","rows","()Ljava/lang/String;",None,&[],&mut host).unwrap();
 let text=match answer { Value::Ref(Some(s))=>s, other=>panic!("{other:?}") };
 assert_eq!(text,ORACLE);
 assert!(host.unknown_host_calls().is_empty(), "{:?}",host.unknown_host_calls());
 assert!(COUNTED.is_empty());
}
