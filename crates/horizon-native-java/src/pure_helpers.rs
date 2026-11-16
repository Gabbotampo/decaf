//! Small side-effect-free library helpers whose contracts are independent of host state.

use crate::{MethodRef, NativeError, StdHost, Value, expect_ref};

const LOG4J_STRINGS: &str = "org/apache/logging/log4j/util/Strings";
const LWJGL_MATH: &str = "org/lwjgl/system/MathUtil";

pub(crate) fn static_call(
    host: &mut StdHost,
    r: &MethodRef,
    args: &[Value],
) -> Option<Result<Value, NativeError>> {
    if r.owner == LWJGL_MATH {
        let [Value::Int(value)] = args else {
            return Some(host.unknown_host_call(r));
        };
        return Some(match (r.name.as_str(), r.descriptor.as_str()) {
            ("mathIsPoT", "(I)Z") => Ok(Value::Int(((*value as u32).count_ones() == 1) as i32)),
            ("mathRoundPoT", "(I)I") => {
                let predecessor = value.wrapping_sub(1) as u32;
                let shift = (32 - predecessor.leading_zeros()) & 31;
                Ok(Value::Int(1i32.wrapping_shl(shift)))
            }
            _ => host.unknown_host_call(r),
        });
    }
    if r.owner != LOG4J_STRINGS {
        return None;
    }
    let Some(value) = args.first() else {
        return Some(host.unknown_host_call(r));
    };
    Some(match (r.name.as_str(), r.descriptor.as_str()) {
        ("isEmpty", "(Ljava/lang/CharSequence;)Z") => Ok(Value::Int(is_empty(value) as i32)),
        ("isNotEmpty", "(Ljava/lang/CharSequence;)Z") => Ok(Value::Int((!is_empty(value)) as i32)),
        ("isBlank", "(Ljava/lang/String;)Z") => Ok(Value::Int(is_blank(value) as i32)),
        ("isNotBlank", "(Ljava/lang/String;)Z") => Ok(Value::Int((!is_blank(value)) as i32)),
        _ => host.unknown_host_call(r),
    })
}

pub(crate) fn empty_factory_class(r: &MethodRef) -> Option<&'static str> {
    matches!(
        (r.owner.as_str(), r.name.as_str(), r.descriptor.as_str()),
        (
            "org/apache/commons/compress/utils/Lists",
            "newArrayList",
            "()Ljava/util/ArrayList;"
        )
    )
    .then_some("java/util/ArrayList")
}

fn is_empty(value: &Value) -> bool {
    matches!(value, Value::Ref(None)) || expect_ref(value).is_some_and(String::is_empty)
}

fn is_blank(value: &Value) -> bool {
    match expect_ref(value) {
        None => true,
        Some(text) => text.is_empty() || text.chars().all(char::is_whitespace),
    }
}
