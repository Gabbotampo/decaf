//! `UnmodifiableList.subList` delegates before wrapping (#251).
//!
//! Java 25 bytecode: `Collections$UnmodifiableList.subList` (and
//! `$UnmodifiableRandomAccessList.subList`) is
//! `new <its own class>(list.subList(from, to))`. The backing call goes
//! through the runtime, so array windows, jar list bodies and range
//! exceptions keep their own behavior. The wrapper is then made with `new`,
//! not through `Collections.unmodifiableList`: its class is the receiver's
//! own, whether or not the part is `RandomAccess`, and it is a new object even
//! when the part already is an unmodifiable list (where the factory would
//! hand the part back).

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
        let class = match crate::runtime::jar_object::object_class(receiver) {
            Some(
                class @ ("java/util/Collections$UnmodifiableList"
                | "java/util/Collections$UnmodifiableRandomAccessList"),
            ) => class.to_owned(),
            _ => return Ok(None),
        };
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
        // `new UnmodifiableList(null)` throws NPE in `UnmodifiableCollection`'s
        // constructor: a jar `subList` that answers `null` gets Java's error.
        if matches!(part, Value::Ref(None)) {
            return Err(NativeError::UncaughtException {
                class: "java/lang/NullPointerException".into(),
                message: None,
                frame: None,
            });
        }
        match self.host.unmodifiable_list_view_as(&class, &part) {
            Some(view) => Ok(Some(view)),
            None => self.host.unknown_operand_call(r).map(Some),
        }
    }
}
