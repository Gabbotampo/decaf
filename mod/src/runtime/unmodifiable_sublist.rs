//! `UnmodifiableList.subList` delegates before wrapping (#251).
//!
//! Java 25 bytecode: `Collections$UnmodifiableList.subList`, offsets 4–15,
//! calls the backing `List.subList` then constructs an unmodifiable wrapper.
//! Dispatch the backing call through the runtime so array windows, jar list
//! bodies and range exceptions keep their own behavior.

use super::*;

impl<H: Host> Runtime<'_, H> {
    pub(crate) fn invoke_unmodifiable_sublist(
        &mut self,
        r: &MethodRef,
        op: u8,
        receiver: Option<&Value>,
        args: &[Value],
    ) -> Result<Option<Value>, NativeError> {
        if !matches!(op, 0xb6 | 0xb9)
            || r.name != "subList"
            || r.descriptor != "(II)Ljava/util/List;"
        {
            return Ok(None);
        }
        let Some(receiver) = receiver else {
            return Ok(None);
        };
        if !matches!(
            crate::runtime::jar_object::object_class(receiver),
            Some(
                "java/util/Collections$UnmodifiableList"
                    | "java/util/Collections$UnmodifiableRandomAccessList"
            )
        ) {
            return Ok(None);
        }
        let Some(source) =
            expect_ref(receiver).and_then(|key| self.host.unmodifiable_view_source(key))
        else {
            return self.host.unknown_operand_call(r).map(Some);
        };
        let source = crate::runtime::jar_object::stored_value(&source);
        let part = match self.call_virtual(
            crate::runtime::jdk_call::Site::Interface,
            "java/util/List",
            "subList",
            &r.descriptor,
            &source,
            args,
        ) {
            Err(NativeError::Metadata(message)) if message == crate::runtime::jdk_call::ABANDONED => {
                return Ok(Some(Value::Ref(None)));
            }
            result => result?,
        };
        self.call_static(
            "java/util/Collections",
            "unmodifiableList",
            "(Ljava/util/List;)Ljava/util/List;",
            &[part],
        )
        .map(Some)
    }
}
