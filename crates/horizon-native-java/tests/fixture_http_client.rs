//! Java 25's `java.net.http` builders: fluent identity, defaults, validation,
//! request state, and a network call that remains counted at the host boundary.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::Value;

const PROBE: &str = include_str!("fixtures/http_client/demo/Probe.java");
const ORACLE: &str = include_str!("fixtures/http_client/java25.txt");

#[test]
fn http_client_and_request_builder_match_java_25() {
    let fixture = javac_fixture::Fixture::build("http-client", &[("demo/Probe.java", PROBE)]);
    let mut host = fixture.jar.seed_host(horizon_native_java::StdHost::new());
    let answer = fixture.jar.invoke_method_with_args(
        "demo/Probe",
        "run",
        "()Ljava/lang/String;",
        None,
        &[],
        &mut host,
    );
    let text = match answer {
        Ok(Value::Ref(Some(text))) => text,
        other => panic!("native answer: {other:?}"),
    };
    assert_eq!(text, ORACLE.trim());
    assert!(host.unknown_host_calls().iter().any(|(call, _)| {
        call == "java/net/http/HttpClient#send(Ljava/net/http/HttpRequest;Ljava/net/http/HttpResponse$BodyHandler;)Ljava/net/http/HttpResponse;"
    }), "send must stay counted: {:?}", host.unknown_host_calls());
}
