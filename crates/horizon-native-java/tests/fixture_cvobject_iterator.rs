#[path = "support/javac_fixture.rs"] mod javac_fixture;
use horizon_native_java::StdHost;
use javac_fixture::Fixture;
const FIXTURE:&str=concat!(env!("CARGO_MANIFEST_DIR"),"/tests/fixtures/fabric_custom_value_iterator");
const MOD_JSON:&str=include_str!("fixtures/fabric_custom_value_iterator/fabric.mod.json");
#[test] fn cvobject_iterator_rows_match_java25(){ let sources:Vec<(&str,&str)>=Vec::new(); let fixture=Fixture::build_with_mod_json("cvobject-iterator",&sources,MOD_JSON); let mut host=fixture.jar.seed_host(StdHost::new()); let rows=fixture.text_on(&mut host,"rows"); let oracle=include_str!("fixtures/fabric_custom_value_iterator/java25.txt"); assert_eq!(rows,oracle); assert!(host.unknown_host_calls().is_empty(),"{:?}",host.unknown_host_calls()); }
