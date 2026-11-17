#[path = "support/javac_fixture.rs"] mod javac_fixture;
const ORACLE:&str=include_str!("fixtures/collections_reverse/java25.txt");
const PROBE:&str=include_str!("fixtures/collections_reverse/demo/Probe.java");
#[test] fn collections_reverse_and_stream_iterator(){let f=javac_fixture::Fixture::build("collections-reverse",&[("demo/Probe.java",PROBE)]);let mut h=f.jar.seed_host(horizon_native_java::StdHost::new());let v=f.jar.invoke_method_with_args("demo/Probe","rows","()Ljava/lang/String;",None,&[],&mut h).unwrap();let got=match v{horizon_native_java::Value::Ref(Some(s))=>s, x=>format!("{x:?}")};assert_eq!(got,ORACLE);assert!(h.unknown_host_calls().is_empty(),"{:?}",h.unknown_host_calls());}
