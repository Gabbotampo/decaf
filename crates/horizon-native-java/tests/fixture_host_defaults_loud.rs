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
}
"#;
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
    let marker = Value::Ref(Some("@indy:demo/Probe#0".into()));
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
