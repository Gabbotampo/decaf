#[path = "support/javac_fixture.rs"]
mod javac_fixture;
use horizon_native_java::{StdHost, Value};

#[test]
fn sublist_writes_match_java25() {
    let f = javac_fixture::Fixture::build(
        "sublist-writes",
        &[(
            "demo/Probe.java",
            include_str!("fixtures/sublist_writes/demo/Probe.java"),
        )],
    );
    for (op, expected) in include_str!("fixtures/sublist_writes/java25.txt")
        .lines()
        .filter_map(|line| {
            line.split_once('=')
                .map(|(op, expected)| (op.parse::<i32>().unwrap(), expected))
        })
    {
        let mut host = f.jar.seed_host(StdHost::new());
        let got = f.jar.invoke_method_with_args(
            "demo/Probe",
            "probe",
            "(I)Ljava/lang/String;",
            None,
            &[Value::Int(op)],
            &mut host,
        );
        assert!(matches!(got, Ok(Value::Ref(Some(ref x))) if x == expected));
        assert_eq!(
            host.unknown_host_call_count(),
            0,
            "op {op}: {:?}",
            host.unknown_host_calls()
        );
    }
    for op in [4, 5, 7, 8] {
        let mut counted = f.jar.seed_host(StdHost::new());
        let got = f.jar.invoke_method_with_args(
            "demo/Probe",
            "review",
            "(I)Ljava/lang/String;",
            None,
            &[Value::Int(op)],
            &mut counted,
        );
        assert!(matches!(
            got,
            Ok(Value::Ref(Some(ref result))) if result == "null|null|null|null"
        ));
        let expected_calls = match op {
            4 => vec![
                (
                    "java/lang/String#valueOf(Ljava/lang/Object;)Ljava/lang/String;".into(),
                    2,
                ),
                ("java/util/List#get(I)Ljava/lang/Object;".into(), 2),
                (
                    "java/util/List#replaceAll(Ljava/util/function/UnaryOperator;)V".into(),
                    1,
                ),
            ],
            5 => vec![
                (
                    "java/lang/String#valueOf(Ljava/lang/Object;)Ljava/lang/String;".into(),
                    2,
                ),
                ("java/util/List#get(I)Ljava/lang/Object;".into(), 2),
                ("java/util/List#sort(Ljava/util/Comparator;)V".into(), 1),
            ],
            7 => vec![
                (
                    "java/lang/String#valueOf(Ljava/lang/Object;)Ljava/lang/String;".into(),
                    2,
                ),
                ("java/util/Collections#sort(Ljava/util/List;)V".into(), 1),
                ("java/util/List#get(I)Ljava/lang/Object;".into(), 2),
            ],
            8 => vec![
                (
                    "java/lang/String#valueOf(Ljava/lang/Object;)Ljava/lang/String;".into(),
                    2,
                ),
                ("java/util/Collections#reverse(Ljava/util/List;)V".into(), 1),
                ("java/util/List#get(I)Ljava/lang/Object;".into(), 2),
            ],
            _ => unreachable!(),
        };
        assert_eq!(counted.unknown_host_calls(), expected_calls, "op {op}");
    }
}

const IMMUTABLE_SUBLIST_ORACLE: &str = "null=NullPointerException,NullPointerException|emptyAdd=UnsupportedOperationException|singleSort=OK|single=UnsupportedOperationException,UnsupportedOperationException,UnsupportedOperationException|singleEmpty=UnsupportedOperationException,OK|copies=UnsupportedOperationException,UnsupportedOperationException";
const IMMUTABLE_SUBLIST_PROBE: &str = r#"package demo;
import java.util.*;
public class Probe {
 static String x(Runnable r){try{r.run();return "OK";}catch(Throwable t){return t.getClass().getSimpleName();}}
 public static String run(){
  List<String> a=List.of("a","b","c","d"); List<String> s=a.subList(1,2).subList(0,1);
  List<String> e=Collections.<String>emptyList().subList(0,0);
  List<String> q=Collections.singletonList("a").subList(0,1);
  List<String> qe=Collections.singletonList("a").subList(0,0);
  List<String> n=Collections.nCopies(2,"a").subList(0,2);
  return "null="+x(()->s.indexOf(null))+","+x(()->s.contains(null))+"|emptyAdd="+x(()->e.add("z"))+"|singleSort="+x(()->Collections.singletonList("a").sort(null))+"|single="+x(()->q.set(0,"z"))+","+x(()->q.add("z"))+","+x(()->q.clear())+"|singleEmpty="+x(()->qe.add("z"))+","+x(()->qe.clear())+"|copies="+x(()->n.set(0,"z"))+","+x(()->n.add("z"));
 }
}"#;

#[test]
fn immutable_sub_lists_match_java_25() {
    let fixture = javac_fixture::Fixture::build(
        "sublist-immutable-kinds",
        &[("demo/Probe.java", IMMUTABLE_SUBLIST_PROBE)],
    );
    let mut host = fixture.jar.seed_host(StdHost::new());
    let got = fixture
        .jar
        .invoke_method_with_args(
            "demo/Probe",
            "run",
            "()Ljava/lang/String;",
            None,
            &[],
            &mut host,
        )
        .unwrap();
    assert!(matches!(got, Value::Ref(Some(ref text)) if text == IMMUTABLE_SUBLIST_ORACLE));
    assert_eq!(host.unknown_host_calls(), Vec::<(String, usize)>::new());
}
