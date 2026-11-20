//! The #261 round-3 review (`fixtures/callbacks_rc261c/demo/Esc.java`;
//! `java25.txt` is its output on OpenJDK 25.0.5+8).
//!
//! * B1: no jar `String` becomes a callback, however many handles it is
//!   wrapped in and however its escapes nest: 48 systematic forges (an
//!   optional or a map entry, one to three deep, zero to three hidden
//!   escapes, run as a `Runnable` or sorted with as a `Comparator`) and the
//!   review's four strings. The forged target never runs.
//! * S1: a callback read back from an `Optional`, `Map.entry` or `entrySet`
//!   is the callback itself, captures and all (a capture the key's text cuts
//!   right after an escape included), and a composite runs whole.
//! * S2: what a lambda captures 100 links down a `thenComparing` chain, or
//!   81 optionals down, survives 40 collections.

#[path = "support/javac_fixture.rs"]
mod javac_fixture;

use horizon_native_java::{StdHost, Value};

const PROBE: &str = include_str!("fixtures/callbacks_rc261c/demo/Esc.java");
const ORACLE: &str = include_str!("fixtures/callbacks_rc261c/java25.txt");

fn java(case: &str) -> &'static str {
    ORACLE
        .lines()
        .find_map(|line| line.strip_prefix(case)?.strip_prefix('='))
        .unwrap_or_else(|| panic!("no Java answer for {case}"))
}

fn call(
    jar: &horizon_native_java::NativeJar,
    host: &mut StdHost,
    method: &str,
    arg: Option<i32>,
) -> Result<Value, horizon_native_java::NativeError> {
    let (desc, args) = match arg {
        Some(n) => ("(I)Ljava/lang/String;", vec![Value::Int(n)]),
        None => ("()Ljava/lang/String;", Vec::new()),
    };
    jar.invoke_method_with_args("demo/Esc", method, desc, None, &args, host)
}

fn shown(out: Result<Value, horizon_native_java::NativeError>) -> String {
    match out {
        Ok(Value::Ref(Some(text))) => text,
        other => format!("{other:?}"),
    }
}

/// No forge yields a callback: each answers as Java does, with a
/// `ClassCastException` at the `(List)` or `(Optional)` cast (`list/` text is
/// a `String` since #255), and the target never runs.
#[test]
fn no_jar_string_becomes_a_callback_at_any_escape_depth() {
    let fixture =
        javac_fixture::Fixture::build("callbacks-rc261c-forge", &[("demo/Esc.java", PROBE)]);
    let cases = (0..48)
        .map(|n| ("forge", n))
        .chain((0..4).map(|n| ("forgeLiteral", n)));
    let mut wrong = Vec::new();
    for (method, n) in cases {
        let name = format!("{method}{n}");
        assert_eq!(java(&name), "ClassCastException,ran=false", "{name}");
        let mut host = fixture.jar.seed_host(StdHost::new());
        let answer = shown(call(&fixture.jar, &mut host, method, Some(n)));
        let ran = shown(call(&fixture.jar, &mut host, "ranFlag", None));
        if ran != "ran=false" || answer != java(&name) {
            wrong.push(format!("{name}: {answer} / {ran}"));
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}

/// A callback read back from an optional, an entry or `entrySet` keeps what
/// it captured and is `==` to itself; composites over long captures and
/// nested composites run whole. Exact, nothing counted.
#[test]
fn a_callback_read_back_is_the_callback_itself() {
    // The 4093- to 5000-link captures nest interpreter frames as deep as the
    // test below: a debug build's frames are large, and the runtime's #260
    // guard throws `StackOverflowError` once the native stack left runs out.
    // On the default 2 MiB test thread this sat within bytes of that bound
    // (it failed at 2,000,000 bytes on main too), so it runs on the same
    // roomy thread as `captures_deeper_than_64_links_survive_collections`.
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(read_back_whole)
        .unwrap()
        .join()
        .unwrap();
}

fn read_back_whole() {
    let fixture =
        javac_fixture::Fixture::build("callbacks-rc261c-read", &[("demo/Esc.java", PROBE)]);
    for method in [
        "readBack4093",
        "readBack4094",
        "readBack5000",
        "readBackComposite",
        "readBackNested",
    ] {
        let mut host = fixture.jar.seed_host(StdHost::new());
        let answer = shown(call(&fixture.jar, &mut host, method, None));
        assert_eq!(answer, java(method), "{method}");
        assert_eq!(
            host.unknown_host_calls(),
            Vec::<(String, usize)>::new(),
            "{method}"
        );
    }
}

/// `gcHold` keeps captures 100 links down a `thenComparing` chain, 81
/// optionals down and 30 links down a negated predicate (running a deeper
/// one in a debug build meets #260's native-stack bound); 40 calls of garbage make the
/// host collect; `gcUse` reads every one back. The collector's walk used to
/// stop at 64 links.
#[test]
fn captures_deeper_than_64_links_survive_collections() {
    // A debug build's interpreter frames are large: run on a thread with room
    // for the 30-link predicate (the bound is the native stack left, #260).
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(collect_and_use)
        .unwrap()
        .join()
        .unwrap();
}

fn collect_and_use() {
    let fixture = javac_fixture::Fixture::build("callbacks-rc261c-gc", &[("demo/Esc.java", PROBE)]);
    let mut host = fixture.jar.seed_host(StdHost::new());
    assert_eq!(
        shown(call(&fixture.jar, &mut host, "gcHold", None)),
        java("gcHold")
    );
    let before = host.resident_heap().instance_fields();
    let mut collected = false;
    for _ in 0..40 {
        assert_eq!(
            shown(call(&fixture.jar, &mut host, "gcGarbage", None)),
            java("gcGarbage")
        );
        collected |= host.resident_heap().instance_fields() <= before;
    }
    assert!(collected, "the host never collected");
    assert_eq!(
        shown(call(&fixture.jar, &mut host, "gcUse", None)),
        java("gcUse")
    );
    assert_eq!(host.unknown_host_calls(), Vec::<(String, usize)>::new());
}
