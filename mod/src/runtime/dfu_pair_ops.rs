//! Runtime-owned operations for DataFixerUpper Pair.
use super::*;

const OWNER: &str = "com/mojang/datafixers/util/Pair";

pub(crate) fn runtime_call<H: Host>(
    runtime: &mut Runtime<'_, H>,
    r: &MethodRef,
    receiver: Option<&Value>,
    args: &[Value],
    stack: &mut [Value],
    locals: &mut [Value],
) -> Result<Option<Value>, NativeError> {
    if r.owner != OWNER {
        return Ok(None);
    }
    if r.name == "<init>"
        && r.descriptor == "(Ljava/lang/Object;Ljava/lang/Object;)V"
        && let Some(receiver) = receiver
        && jar_object::is_object(receiver)
    {
        let [first, second] = args else {
            return Ok(None);
        };
        let pair = crate::runtime::host_codec::pair(first, second);
        for slot in stack.iter_mut().chain(locals.iter_mut()) {
            if slot == receiver {
                *slot = pair.clone();
            }
        }
        return Ok(Some(Value::Void));
    }
    let Some(receiver) = receiver else {
        return Ok(None);
    };
    let Some((first, second)) = crate::runtime::host_codec::pair_parts(receiver) else {
        return Ok(None);
    };
    let callback = args.first();
    match r.name.as_str() {
        "mapFirst"
            if r.descriptor
                == "(Ljava/util/function/Function;)Lcom/mojang/datafixers/util/Pair;" =>
        {
            let Some(callback) = callback else {
                return Ok(None);
            };
            let Some(mapped) = runtime.invoke_callback(callback, &[first])? else {
                return Ok(None);
            };
            Ok(Some(crate::runtime::host_codec::pair(&mapped, &second)))
        }
        "mapSecond"
            if r.descriptor
                == "(Ljava/util/function/Function;)Lcom/mojang/datafixers/util/Pair;" =>
        {
            let Some(callback) = callback else {
                return Ok(None);
            };
            let Some(mapped) = runtime.invoke_callback(callback, &[second])? else {
                return Ok(None);
            };
            Ok(Some(crate::runtime::host_codec::pair(&first, &mapped)))
        }
        _ => Ok(None),
    }
}
