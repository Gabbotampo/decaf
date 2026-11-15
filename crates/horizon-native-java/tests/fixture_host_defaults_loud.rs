//! A host that keeps no state of a kind is never answered for silently.
//!
//! [`NullHost`] implements no `Host` member, and `Constructs` only answers
//! constructors (a host that builds objects but records nothing about them).
//! Each probe below reaches one trait default through the runtime, as a jar
//! does. Before, each one answered: `null` for the static, `0` for the
//! call, `null` for an exception's message, an empty stream for a reader,
//! and a host marker's key inside a record's text. Now each refuses with
//! [`NativeError::UnknownHostCall`] naming what was not known, as a counting
//! host (`StdHost`) counts it.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::{Host, MethodRef, NativeError, NullHost, Value};
use javac_fixture::Fixture;

const PROBE: &str = r#"package demo;
public class Probe {
    public static String staticField() { return String.valueOf(net.minecraft.world.item.Items.DIAMOND); }
    public static int libraryCall() { return lib.Lib.answer(); }
    public static String message() { return new Oops("boom").getMessage(); }
    public static int read(java.io.InputStream in) throws java.io.IOException {
        return new java.io.InputStreamReader(in).read();
    }
    public static String record(Object value) { return new Box(value).toString(); }
    public static String buffered() throws java.io.IOException {
        java.io.InputStream in = new java.io.ByteArrayInputStream(new byte[] {104, 105});
        return new java.io.BufferedReader(new java.io.InputStreamReader(in)).readLine();
    }
    public static String nullReader() {
        try { new java.io.InputStreamReader((java.io.InputStream) null); return "none"; }
        catch (NullPointerException e) { return "npe " + e.getMessage(); }
    }
    public static String nullBuffered() {
        try { new java.io.BufferedReader(null); return "none"; }
        catch (NullPointerException e) { return "npe " + e.getMessage(); }
    }
    public static String initializerCause() {
        try { return String.valueOf(Bad.VALUE); }
        catch (ExceptionInInitializerError e) { return String.valueOf(e.getCause()); }
    }
    public static String raisedCause() {
        try { Object o = null; return String.valueOf(o.hashCode()); }
        catch (NullPointerException e) { return String.valueOf(e.getCause()); }
    }
}
"#;
const BAD: &str = "package demo;\npublic class Bad { public static final int VALUE = Integer.parseInt(\"x\"); }\n";
const OOPS: &str = "package demo;\npublic class Oops extends RuntimeException { public Oops(String m) { super(m); } }\n";
const BOX: &str = "package demo;\npublic record Box(Object value) {}\n";
const ITEMS: &str = "package net.minecraft.world.item;\npublic class Items { public static final Object DIAMOND = new Object(); }\n";
const LIB: &str = "package lib;\npublic class Lib { public static int answer() { return 42; } }\n";

fn fixture() -> Fixture {
    Fixture::build(
        "host-defaults-loud",
        &[
            ("demo/Probe.java", PROBE),
            ("demo/Oops.java", OOPS),
            ("demo/Box.java", BOX),
            ("demo/Bad.java", BAD),
            ("net/minecraft/world/item/Items.java", ITEMS),
            ("lib/Lib.java", LIB),
        ],
    )
}

/// A host that answers constructors and nothing else: it records no
/// message for the throwables it builds.
struct Constructs;

impl Host for Constructs {
    fn invoke(&mut self, r: &MethodRef, _: &[Value]) -> Result<Value, NativeError> {
        if r.name == "<init>" {
            return Ok(Value::Void);
        }
        Err(NativeError::UnknownHostCall(r.clone()))
    }
}

fn refusal<H: Host>(
    fixture: &Fixture,
    host: &mut H,
    method: &str,
    desc: &str,
    args: &[Value],
) -> String {
    match fixture
        .jar
        .invoke_method_with_args("demo/Probe", method, desc, None, args, host)
    {
        Err(NativeError::UnknownHostCall(r)) => format!("{}#{}{}", r.owner, r.name, r.descriptor),
        other => panic!("{method} answered without a count: {other:?}"),
    }
}

#[test]
fn a_host_that_models_nothing_refuses_what_it_was_asked() {
    let fixture = fixture();
    assert_eq!(
        refusal(
            &fixture,
            &mut NullHost,
            "staticField",
            "()Ljava/lang/String;",
            &[]
        ),
        "net/minecraft/world/item/Items#DIAMONDLjava/lang/Object;"
    );
    assert_eq!(
        refusal(&fixture, &mut NullHost, "libraryCall", "()I", &[]),
        "lib/Lib#answer()I"
    );
}

#[test]
fn a_message_no_host_recorded_is_not_null() {
    let fixture = fixture();
    assert_eq!(
        refusal(
            &fixture,
            &mut Constructs,
            "message",
            "()Ljava/lang/String;",
            &[]
        ),
        "demo/Oops#getMessage()Ljava/lang/String;"
    );
}

#[test]
fn a_reader_over_a_stream_no_host_holds_is_not_empty() {
    let fixture = fixture();
    let stream = Value::Ref(Some("resource-stream/config.json".into()));
    assert_eq!(
        refusal(
            &fixture,
            &mut Constructs,
            "read",
            "(Ljava/io/InputStream;)I",
            &[stream]
        ),
        "java/io/InputStreamReader#read()I"
    );
}

#[test]
fn a_record_never_prints_a_marker_no_model_prints() {
    let fixture = fixture();
    // A live callback: no model prints it, and its text (which carries the
    // run's tag) is never shown.
    let marker = horizon_native_java::callback_test_token("demo/Probe#0");
    assert_eq!(
        refusal(
            &fixture,
            &mut Constructs,
            "record",
            "(Ljava/lang/Object;)Ljava/lang/String;",
            &[marker]
        ),
        "java/lang/Object#toString()Ljava/lang/String;"
    );
}

/// What a counting host answers for the review's cases (#257's review): each
/// is Java 25's answer, or counted, never a silent other value. Java 25:
/// `hi`; `npe null` twice (`Reader(Object lock)`'s bare NPE); the
/// initializer error's cause is the `NumberFormatException`, which the host
/// never held, so it is counted; a raised NPE's cause is `null`, exactly.
#[test]
fn readers_and_raised_causes_answer_as_java_or_count() {
    let fixture = fixture();
    let call = |host: &mut horizon_native_java::StdHost, method: &str| -> String {
        match fixture.jar.invoke_method_with_args(
            "demo/Probe",
            method,
            "()Ljava/lang/String;",
            None,
            &[],
            host,
        ) {
            Ok(Value::Ref(text)) => text.unwrap_or_else(|| "null".into()),
            other => format!("{other:?}"),
        }
    };
    let counted = |host: &horizon_native_java::StdHost, needle: &str| {
        host.unknown_host_calls()
            .iter()
            .any(|(call, _)| call.contains(needle))
    };
    let mut host = fixture.jar.seed_host(horizon_native_java::StdHost::new());
    let line = call(&mut host, "buffered");
    assert!(
        line == "hi" || counted(&host, "java/io/"),
        "buffered: {line} {:?}",
        host.unknown_host_calls()
    );
    let mut host = fixture.jar.seed_host(horizon_native_java::StdHost::new());
    assert_eq!(call(&mut host, "nullReader"), "npe null");
    assert_eq!(call(&mut host, "nullBuffered"), "npe null");
    let mut host = fixture.jar.seed_host(horizon_native_java::StdHost::new());
    call(&mut host, "initializerCause");
    assert!(
        counted(&host, "getCause"),
        "{:?}",
        host.unknown_host_calls()
    );
    let mut host = fixture.jar.seed_host(horizon_native_java::StdHost::new());
    assert_eq!(call(&mut host, "raisedCause"), "null");
    assert!(
        !counted(&host, "getCause"),
        "{:?}",
        host.unknown_host_calls()
    );
}
