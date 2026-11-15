//! Bounded Java 25 models for `java.net.http` builders and immutable values.
//!
//! Builder state lives in the runtime's persistent static-state map, keyed by the
//! typed handle. Fluent setters return the same handle, while `build()` snapshots
//! the state into a new client/request handle. Network operations intentionally
//! return `None` so the normal host boundary counts them.

use crate::host_ref::{HostKind, HostRef, Methods, Nested, instance_of_class};
use crate::{MethodRef, NativeError, StdHost, Value};

const HTTP_CLIENT: &str = "java/net/http/HttpClient";
const HTTP_CLIENT_BUILDER: &str = "java/net/http/HttpClient$Builder";
const HTTP_REQUEST: &str = "java/net/http/HttpRequest";
const HTTP_REQUEST_BUILDER: &str = "java/net/http/HttpRequest$Builder";
const STATE_PREFIX: &str = "\u{0}horizon-java/http-client/";

fn identity_equals(
    handle: &HostRef,
    other: &Value,
    _: &mut dyn Nested,
) -> Result<Option<bool>, NativeError> {
    Ok(match other {
        Value::Host(other) if other.kind() == handle.kind() => handle.same_object(other),
        _ => Some(false),
    })
}

const OBJECT_METHODS: Methods = Methods {
    to_string: |_, _| Ok(None),
    hash_code: |_, _| Ok(None),
    equals: identity_equals,
    instance_of: instance_of_class,
    crosses_jars: |_, _| false,
    compare_to: None,
};

pub(crate) fn client_types(_: &str) -> &'static [&'static str] {
    &[HTTP_CLIENT]
}
pub(crate) fn client_builder_types(_: &str) -> &'static [&'static str] {
    &[HTTP_CLIENT_BUILDER]
}
pub(crate) fn request_types(_: &str) -> &'static [&'static str] {
    &[HTTP_REQUEST]
}
pub(crate) fn request_builder_types(_: &str) -> &'static [&'static str] {
    &[HTTP_REQUEST_BUILDER]
}

pub(crate) const CLIENT_METHODS: Methods = OBJECT_METHODS;
pub(crate) const CLIENT_BUILDER_METHODS: Methods = OBJECT_METHODS;
pub(crate) const REQUEST_METHODS: Methods = OBJECT_METHODS;
pub(crate) const REQUEST_BUILDER_METHODS: Methods = OBJECT_METHODS;

fn enum_value(owner: &str, name: &str) -> Value {
    Value::Ref(Some(format!("enum/{owner}/{name}")))
}

pub(crate) fn static_field(owner: &str, name: &str, descriptor: &str) -> Option<Value> {
    match (owner, name, descriptor) {
        ("java/net/http/HttpClient$Version", "HTTP_1_1", "Ljava/net/http/HttpClient$Version;")
        | ("java/net/http/HttpClient$Version", "HTTP_2", "Ljava/net/http/HttpClient$Version;")
        | ("java/net/http/HttpClient$Redirect", "NEVER", "Ljava/net/http/HttpClient$Redirect;")
        | ("java/net/http/HttpClient$Redirect", "NORMAL", "Ljava/net/http/HttpClient$Redirect;")
        | ("java/net/http/HttpClient$Redirect", "ALWAYS", "Ljava/net/http/HttpClient$Redirect;") => {
            Some(enum_value(owner, name))
        }
        _ => None,
    }
}

fn state_key(kind: &str, key: &str) -> String {
    format!("{STATE_PREFIX}{kind}/{key}")
}

fn state(runtime: &StdHost, kind: &str, key: &str) -> Option<serde_json::Value> {
    let key = state_key(kind, key);
    let Value::Ref(Some(text)) = runtime.persistent_static_fields.get(&key)? else {
        return None;
    };
    serde_json::from_str(text).ok()
}

fn save_state(runtime: &mut StdHost, kind: &str, key: &str, value: serde_json::Value) {
    runtime
        .persistent_static_fields
        .insert(state_key(kind, key), Value::Ref(Some(value.to_string())));
}

fn allocation(kind: HostKind, class: &str) -> Value {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let serial = NEXT.fetch_add(1, Ordering::Relaxed);
    Value::Host(HostRef::new(kind, format!("{class}#http-{serial}")))
}

fn null_pointer() -> NativeError {
    NativeError::UncaughtException {
        class: "java/lang/NullPointerException".into(),
        message: None,
        frame: None,
    }
}

fn illegal_argument(message: &str) -> NativeError {
    NativeError::UncaughtException {
        class: "java/lang/IllegalArgumentException".into(),
        message: Some(message.into()),
        frame: None,
    }
}

fn invalid_duration(value: &Value) -> Result<i128, NativeError> {
    let Some(nanos) = crate::host_jdk_value::duration_nanos(value) else {
        return Err(null_pointer());
    };
    if nanos < 0 {
        return Err(illegal_argument("Invalid duration"));
    }
    Ok(nanos)
}

fn string_arg(value: Option<&Value>) -> Option<&str> {
    match value {
        Some(Value::Ref(Some(value))) => Some(value),
        _ => None,
    }
}

fn valid_header_name(name: &str) -> bool {
    !name.is_empty()
        && name.bytes().all(|byte| {
            byte.is_ascii()
                && byte > 0x20
                && byte < 0x7f
                && !b"()<>@,;:\\[\\]={}\t\"".contains(&byte)
        })
}

fn valid_header_value(value: &str) -> bool {
    !value.bytes().any(|byte| byte == b'\r' || byte == b'\n')
}

fn method_name(name: &str) -> Result<(), NativeError> {
    if name.is_empty()
        || !name.bytes().all(|byte| {
            byte.is_ascii()
                && byte > 0x20
                && byte < 0x7f
                && !b"()<>@,;:\\[\\]={}\t\"".contains(&byte)
        })
    {
        return Err(illegal_argument("invalid HTTP method name"));
    }
    if ["connection", "content-length", "expect", "host", "upgrade"]
        .iter()
        .any(|forbidden| name.eq_ignore_ascii_case(forbidden))
    {
        return Err(illegal_argument("restricted header name"));
    }
    Ok(())
}

fn value_enum(value: &Value, owner: &str, allowed: &[&str]) -> Option<String> {
    let Value::Ref(Some(text)) = value else {
        return None;
    };
    let prefix = format!("enum/{owner}/");
    let name = text.strip_prefix(&prefix)?;
    allowed.contains(&name).then(|| name.to_owned())
}

pub(crate) fn static_call(
    runtime: &mut StdHost,
    r: &MethodRef,
    args: &[Value],
) -> Option<Result<Value, NativeError>> {
    match (r.owner.as_str(), r.name.as_str(), r.descriptor.as_str()) {
        ("java/net/http/HttpClient", "newBuilder", "()Ljava/net/http/HttpClient$Builder;") => {
            let value = allocation(HostKind::HttpClientBuilder, HTTP_CLIENT_BUILDER);
            if let Value::Host(handle) = &value {
                save_state(
                    runtime,
                    "client-builder",
                    handle.key(),
                    serde_json::json!({"timeout": null, "version": "HTTP_2", "redirects": "NEVER"}),
                );
            }
            Some(Ok(value))
        }
        ("java/net/http/HttpRequest", "newBuilder", "()Ljava/net/http/HttpRequest$Builder;") => {
            Some(Ok(new_request_builder(runtime, None)))
        }
        (
            "java/net/http/HttpRequest",
            "newBuilder",
            "(Ljava/net/URI;)Ljava/net/http/HttpRequest$Builder;",
        ) => {
            let uri = match args.first() {
                Some(Value::Ref(None)) => return Some(Err(null_pointer())),
                Some(value) => crate::host_jdk_value::uri_text(value).map(str::to_owned),
                None => None,
            };
            Some(match uri {
                Some(uri) => Ok(new_request_builder(runtime, Some(uri))),
                None => Err(null_pointer()),
            })
        }
        _ => None,
    }
}

fn new_request_builder(runtime: &mut StdHost, uri: Option<String>) -> Value {
    let value = allocation(HostKind::HttpRequestBuilder, HTTP_REQUEST_BUILDER);
    if let Value::Host(handle) = &value {
        save_state(
            runtime,
            "request-builder",
            handle.key(),
            serde_json::json!({"uri": uri, "timeout": null, "method": "GET", "headers": [], "expect": false}),
        );
    }
    value
}

fn receiver_key(receiver: &Value, kind: HostKind) -> Option<&str> {
    match receiver {
        Value::Host(handle) if handle.kind() == kind => Some(handle.key()),
        _ => None,
    }
}

pub(crate) fn instance_call(
    runtime: &mut StdHost,
    r: &MethodRef,
    receiver: &Value,
    args: &[Value],
) -> Option<Result<Value, NativeError>> {
    let kind = match receiver {
        Value::Host(handle) => handle.kind(),
        _ => return None,
    };
    match kind {
        HostKind::HttpClientBuilder => builder_call(runtime, r, receiver, args),
        HostKind::HttpClient => client_call(runtime, r, receiver),
        HostKind::HttpRequestBuilder => request_builder_call(runtime, r, receiver, args),
        HostKind::HttpRequest => request_call(runtime, r, receiver),
        _ => None,
    }
}

fn builder_call(
    runtime: &mut StdHost,
    r: &MethodRef,
    receiver: &Value,
    args: &[Value],
) -> Option<Result<Value, NativeError>> {
    let key = receiver_key(receiver, HostKind::HttpClientBuilder)?;
    let mut current = state(runtime, "client-builder", key).unwrap_or_else(
        || serde_json::json!({"timeout": null, "version": "HTTP_2", "redirects": "NEVER"}),
    );
    match (r.name.as_str(), r.descriptor.as_str()) {
        ("connectTimeout", "(Ljava/time/Duration;)Ljava/net/http/HttpClient$Builder;") => {
            let nanos = match args.first() {
                Some(Value::Ref(None)) => return Some(Err(null_pointer())),
                Some(value) => match invalid_duration(value) {
                    Ok(nanos) => nanos,
                    Err(error) => return Some(Err(error)),
                },
                None => return None,
            };
            current["timeout"] = serde_json::Value::String(nanos.to_string());
            save_state(runtime, "client-builder", key, current);
            Some(Ok(receiver.clone()))
        }
        ("version", "(Ljava/net/http/HttpClient$Version;)Ljava/net/http/HttpClient$Builder;") => {
            let Some(version) = args.first().and_then(|v| {
                value_enum(
                    v,
                    "java/net/http/HttpClient$Version",
                    &["HTTP_1_1", "HTTP_2"],
                )
            }) else {
                return Some(Err(null_pointer()));
            };
            current["version"] = serde_json::Value::String(version);
            save_state(runtime, "client-builder", key, current);
            Some(Ok(receiver.clone()))
        }
        (
            "followRedirects",
            "(Ljava/net/http/HttpClient$Redirect;)Ljava/net/http/HttpClient$Builder;",
        ) => {
            let Some(redirects) = args.first().and_then(|v| {
                value_enum(
                    v,
                    "java/net/http/HttpClient$Redirect",
                    &["NEVER", "NORMAL", "ALWAYS"],
                )
            }) else {
                return Some(Err(null_pointer()));
            };
            current["redirects"] = serde_json::Value::String(redirects);
            save_state(runtime, "client-builder", key, current);
            Some(Ok(receiver.clone()))
        }
        ("executor", "(Ljava/util/concurrent/Executor;)Ljava/net/http/HttpClient$Builder;")
        | ("proxy", "(Ljava/net/ProxySelector;)Ljava/net/http/HttpClient$Builder;")
        | ("authenticator", "(Ljava/net/Authenticator;)Ljava/net/http/HttpClient$Builder;")
        | ("sslContext", "(Ljavax/net/ssl/SSLContext;)Ljava/net/http/HttpClient$Builder;")
        | ("sslParameters", "(Ljavax/net/ssl/SSLParameters;)Ljava/net/http/HttpClient$Builder;")
        | ("priority", "(I)Ljava/net/http/HttpClient$Builder;") => Some(Ok(receiver.clone())),
        ("build", "()Ljava/net/http/HttpClient;") => {
            let client = allocation(HostKind::HttpClient, HTTP_CLIENT);
            if let Value::Host(handle) = &client {
                save_state(runtime, "client", handle.key(), current);
            }
            Some(Ok(client))
        }
        _ => None,
    }
}

fn client_call(
    runtime: &mut StdHost,
    r: &MethodRef,
    receiver: &Value,
) -> Option<Result<Value, NativeError>> {
    let key = receiver_key(receiver, HostKind::HttpClient)?;
    let current = state(runtime, "client", key).unwrap_or_else(
        || serde_json::json!({"timeout": null, "version": "HTTP_2", "redirects": "NEVER"}),
    );
    match (r.name.as_str(), r.descriptor.as_str()) {
        ("connectTimeout", "()Ljava/util/Optional;") => Some(Ok(current["timeout"]
            .as_str()
            .map_or_else(crate::host_optional::empty_optional, |text| {
                crate::host_optional::optional_wrap(crate::host_jdk_value::duration(
                    text.parse().unwrap_or(0),
                ))
            }))),
        ("version", "()Ljava/net/http/HttpClient$Version;") => Some(Ok(enum_value(
            "java/net/http/HttpClient$Version",
            current["version"].as_str().unwrap_or("HTTP_2"),
        ))),
        ("followRedirects", "()Ljava/net/http/HttpClient$Redirect;") => Some(Ok(enum_value(
            "java/net/http/HttpClient$Redirect",
            current["redirects"].as_str().unwrap_or("NEVER"),
        ))),
        ("proxy", "()Ljava/util/Optional;")
        | ("authenticator", "()Ljava/util/Optional;")
        | ("executor", "()Ljava/util/Optional;") => {
            Some(Ok(crate::host_optional::empty_optional()))
        }
        // `send` and `sendAsync` deliberately fall through to the counted host boundary.
        _ => None,
    }
}

fn request_builder_call(
    runtime: &mut StdHost,
    r: &MethodRef,
    receiver: &Value,
    args: &[Value],
) -> Option<Result<Value, NativeError>> {
    let key = receiver_key(receiver, HostKind::HttpRequestBuilder)?;
    let mut current = state(runtime, "request-builder", key).unwrap_or_else(|| {
        serde_json::json!({"uri": null, "timeout": null, "method": "GET", "headers": [], "expect": false})
    });
    match (r.name.as_str(), r.descriptor.as_str()) {
        ("uri", "(Ljava/net/URI;)Ljava/net/http/HttpRequest$Builder;") => {
            let value = args.first()?;
            let Some(uri) = crate::host_jdk_value::uri_text(value) else {
                return Some(Err(null_pointer()));
            };
            current["uri"] = serde_json::Value::String(uri.to_owned());
            save_state(runtime, "request-builder", key, current);
            Some(Ok(receiver.clone()))
        }
        ("timeout", "(Ljava/time/Duration;)Ljava/net/http/HttpRequest$Builder;") => {
            let value = args.first()?;
            let nanos = match invalid_duration(value) {
                Ok(nanos) => nanos,
                Err(error) => return Some(Err(error)),
            };
            current["timeout"] = serde_json::Value::String(nanos.to_string());
            save_state(runtime, "request-builder", key, current);
            Some(Ok(receiver.clone()))
        }
        ("expectContinue", "(Z)Ljava/net/http/HttpRequest$Builder;") => {
            current["expect"] = serde_json::Value::Bool(
                args.first()
                    .and_then(|v| match v {
                        Value::Int(i) => Some(*i != 0),
                        _ => None,
                    })
                    .unwrap_or(false),
            );
            save_state(runtime, "request-builder", key, current);
            Some(Ok(receiver.clone()))
        }
        ("GET", "()Ljava/net/http/HttpRequest$Builder;") => {
            current["method"] = serde_json::Value::String("GET".into());
            save_state(runtime, "request-builder", key, current);
            Some(Ok(receiver.clone()))
        }
        ("header", "(Ljava/lang/String;Ljava/lang/String;)Ljava/net/http/HttpRequest$Builder;") => {
            let Some(name) = string_arg(args.first()) else {
                return Some(Err(null_pointer()));
            };
            let Some(value) = string_arg(args.get(1)) else {
                return Some(Err(null_pointer()));
            };
            if !valid_header_name(name) || !valid_header_value(value) {
                return Some(Err(illegal_argument("invalid header")));
            }
            current["headers"]
                .as_array_mut()?
                .push(serde_json::json!([name, value]));
            save_state(runtime, "request-builder", key, current);
            Some(Ok(receiver.clone()))
        }
        (
            "setHeader",
            "(Ljava/lang/String;Ljava/lang/String;)Ljava/net/http/HttpRequest$Builder;",
        ) => {
            let Some(name) = string_arg(args.first()) else {
                return Some(Err(null_pointer()));
            };
            let Some(value) = string_arg(args.get(1)) else {
                return Some(Err(null_pointer()));
            };
            if !valid_header_name(name) || !valid_header_value(value) {
                return Some(Err(illegal_argument("invalid header")));
            }
            let headers = current["headers"].as_array_mut()?;
            headers.retain(|entry| {
                entry
                    .get(0)
                    .and_then(serde_json::Value::as_str)
                    .is_none_or(|old| !old.eq_ignore_ascii_case(name))
            });
            headers.push(serde_json::json!([name, value]));
            save_state(runtime, "request-builder", key, current);
            Some(Ok(receiver.clone()))
        }
        ("build", "()Ljava/net/http/HttpRequest;") => {
            let Some(uri) = current["uri"].as_str() else {
                return Some(Err(NativeError::UncaughtException {
                    class: "java/lang/IllegalStateException".into(),
                    message: Some("uri is required".into()),
                    frame: None,
                }));
            };
            let request = allocation(HostKind::HttpRequest, HTTP_REQUEST);
            if let Value::Host(handle) = &request {
                current["uri"] = serde_json::Value::String(uri.into());
                save_state(runtime, "request", handle.key(), current);
            }
            Some(Ok(request))
        }
        ("version", "(Ljava/net/http/HttpClient$Version;)Ljava/net/http/HttpRequest$Builder;") => {
            Some(Ok(receiver.clone()))
        }
        (
            "method",
            "(Ljava/lang/String;Ljava/net/http/HttpRequest$BodyPublisher;)Ljava/net/http/HttpRequest$Builder;",
        ) => {
            let Some(method) = string_arg(args.first()) else {
                return Some(Err(null_pointer()));
            };
            if let Err(error) = method_name(method) {
                return Some(Err(error));
            }
            if matches!(args.get(1), Some(Value::Ref(None)) | None) {
                return Some(Err(null_pointer()));
            }
            current["method"] = serde_json::Value::String(method.into());
            save_state(runtime, "request-builder", key, current);
            Some(Ok(receiver.clone()))
        }
        _ => None,
    }
}

fn request_call(
    runtime: &mut StdHost,
    r: &MethodRef,
    receiver: &Value,
) -> Option<Result<Value, NativeError>> {
    let key = receiver_key(receiver, HostKind::HttpRequest)?;
    let current = state(runtime, "request", key)?;
    match (r.name.as_str(), r.descriptor.as_str()) {
        ("uri", "()Ljava/net/URI;") => current["uri"]
            .as_str()
            .map(|uri| Ok(crate::host_jdk_value::uri(uri))),
        ("method", "()Ljava/lang/String;") => current["method"]
            .as_str()
            .map(|method| Ok(Value::Ref(Some(method.into())))),
        ("timeout", "()Ljava/util/Optional;") => Some(Ok(current["timeout"].as_str().map_or_else(
            crate::host_optional::empty_optional,
            |text| {
                crate::host_optional::optional_wrap(crate::host_jdk_value::duration(
                    text.parse().unwrap_or(0),
                ))
            },
        ))),
        ("expectContinue", "()Z") => Some(Ok(Value::Int(i32::from(
            current["expect"].as_bool().unwrap_or(false),
        )))),
        _ => None,
    }
}
