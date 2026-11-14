//! A jar's own `BlockPos`, `MutableBlockPos` or `Vec3i` and `LevelResource`
//! (`new BlockPos(1, 2, 3)`, a jar object of a game class the runtime does
//! not hold), equal to the game's position and resource handles as Java's
//! are (#245).
//!
//! The game's constructors run here on the jar object: `Vec3i(int, int, int)`
//! stores `Vec3i.x/y/z`, `LevelResource(String)` stores the record's `id`, in
//! the object's own fields (the slots `getfield` reads). Every answer below
//! reads those fields, as vanilla 26.3's bodies do (javap, JDK 25.0.5):
//!
//! * `Vec3i.equals`: `this == o`, then `o instanceof Vec3i`, then
//!   `getX()`/`getY()`/`getZ()` of both, so a `BlockPos` and a
//!   `MutableBlockPos` at one place are equal;
//! * `Vec3i.hashCode`: `(getY() + getZ() * 31) * 31 + getX()`;
//! * `Vec3i.compareTo`: by `y`, then `z`, then `x`, as `int` differences;
//! * `Vec3i.toString`: Guava's `toStringHelper`, the class's simple name;
//! * `MutableBlockPos`'s constructors and setters (`set` through `setX`/
//!   `setY`/`setZ`, doubles through `Mth.floor`);
//! * `LevelResource`, a final record: `equals` by `id`, `hashCode` the id's,
//!   `toString` `"/" + id`.
//!
//! A host handle (`block-pos/x,y,z`, `level-resource/<id>`,
//! `server_values.rs`) and a jar object compare by the same key both ways:
//! the handle's row asks [`crate::runtime::host_ref::Nested::server_value_key`].
//! Where an answer needs what the run does not hold, it is counted, never a
//! silent `false`: an object whose constructor ran elsewhere (fields never
//! written), a jar subclass that overrides a member Java's answer calls, or
//! another value that may be a `Vec3i` the run cannot see into.
//!
//! A `MutableBlockPos` changed after it went into a `HashMap` is found as
//! Java finds it: the table keeps the hash it was put with, and each lookup
//! compares the key's fields as they are now.

use super::*;

const VEC3I: &str = "net/minecraft/core/Vec3i";
const BLOCK_POS: &str = "net/minecraft/core/BlockPos";
const MUTABLE_BLOCK_POS: &str = "net/minecraft/core/BlockPos$MutableBlockPos";
const LEVEL_RESOURCE: &str = "net/minecraft/world/level/storage/LevelResource";

/// The members a jar subclass may override that the answers here call
/// (`Vec3i.equals` reads `getX()`, `MutableBlockPos.set` calls `setX`...):
/// with one overridden, the answer is the subclass's, which is counted.
const VEC3I_MEMBERS: &[(&str, &str)] = &[
    ("getX", "()I"),
    ("getY", "()I"),
    ("getZ", "()I"),
    ("equals", "(Ljava/lang/Object;)Z"),
    ("hashCode", "()I"),
    ("compareTo", "(Lnet/minecraft/core/Vec3i;)I"),
    ("toString", "()Ljava/lang/String;"),
    ("setX", "(I)Lnet/minecraft/core/BlockPos$MutableBlockPos;"),
    ("setY", "(I)Lnet/minecraft/core/BlockPos$MutableBlockPos;"),
    ("setZ", "(I)Lnet/minecraft/core/BlockPos$MutableBlockPos;"),
];

/// How deep a jar class chain is followed to its game superclass.
const MAX_CHAIN: usize = 64;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Family {
    Vec3i { mutable: bool },
    LevelResource,
}

/// What another value is to a family member's `equals`/`compareTo`.
enum Other {
    /// A value of the family: its handle key (`block-pos/1,2,3`).
    Key(String),
    /// Surely no `Vec3i`/`LevelResource` (null, a `String`, a JDK object).
    Foreign,
    /// May be one, whose fields the run cannot read: counted.
    Unknown,
}

fn field(owner: &str, name: &str, descriptor: &str) -> MethodRef {
    MethodRef {
        owner: owner.into(),
        name: name.into(),
        descriptor: descriptor.into(),
    }
}

const COORDS: [&str; 3] = ["x", "y", "z"];

/// `Mth.floor(double)`: `(int) d`, one less when that rounded up.
fn floor(value: f64) -> i32 {
    let truncated = value as i32;
    if value < f64::from(truncated) {
        truncated.wrapping_sub(1)
    } else {
        truncated
    }
}

/// `Class.getSimpleName` of a named class: after the package and any `$`.
fn simple_name(class: &str) -> &str {
    let name = class.rsplit('/').next().unwrap_or(class);
    name.rsplit('$').next().unwrap_or(name)
}

impl<H: Host> Runtime<'_, H> {
    /// The family of `value` when it is a jar object whose class chain
    /// reaches one, and whether a jar class on the way overrides a member the
    /// family's answers call.
    fn server_value_family(&self, value: &Value) -> Option<(Family, bool)> {
        let mut current = crate::runtime::jar_object::object_class(value)?.to_owned();
        let mut overridden = false;
        for _ in 0..MAX_CHAIN {
            match current.as_str() {
                VEC3I | BLOCK_POS => return Some((Family::Vec3i { mutable: false }, overridden)),
                MUTABLE_BLOCK_POS => return Some((Family::Vec3i { mutable: true }, overridden)),
                LEVEL_RESOURCE => return Some((Family::LevelResource, overridden)),
                _ => {}
            }
            let file = self.class(&current)?;
            overridden |= VEC3I_MEMBERS
                .iter()
                .any(|(name, descriptor)| file.method(name, descriptor).is_some());
            if file.super_name.is_empty() {
                return None;
            }
            current = file.super_name.clone();
        }
        None
    }

    fn coords(&self, value: &Value) -> Option<[i32; 3]> {
        let mut out = [0; 3];
        for (slot, name) in out.iter_mut().zip(COORDS) {
            let key = instance_field_key(value, &field(VEC3I, name, "I"))?;
            *slot = match self.fields.get(&key)? {
                Value::Int(coordinate) => *coordinate,
                _ => return None,
            };
        }
        Some(out)
    }

    fn set_coords(&mut self, value: &Value, coords: [Option<i32>; 3]) {
        for (coordinate, name) in coords.into_iter().zip(COORDS) {
            if let (Some(coordinate), Some(key)) = (
                coordinate,
                instance_field_key(value, &field(VEC3I, name, "I")),
            ) {
                self.fields.insert(key, Value::Int(coordinate));
            }
        }
    }

    fn resource_id(&self, value: &Value) -> Option<String> {
        let key = instance_field_key(value, &field(LEVEL_RESOURCE, "id", "Ljava/lang/String;"))?;
        match self.fields.get(&key)? {
            Value::Ref(Some(id)) => Some(id.clone()),
            _ => None,
        }
    }

    /// The key a game handle of the same value has (`block-pos/x,y,z`,
    /// `level-resource/<id>`, `server_values.rs`): `None` for anything but a
    /// jar object of the family, `Some(None)` where Java's answer needs what
    /// the run does not hold (fields written elsewhere, a jar override).
    pub(crate) fn jar_server_value_key(&self, value: &Value) -> Option<Option<String>> {
        let (family, overridden) = self.server_value_family(value)?;
        if overridden {
            return Some(None);
        }
        Some(match family {
            Family::Vec3i { .. } => self
                .coords(value)
                .map(|[x, y, z]| format!("{}{x},{y},{z}", crate::runtime::server_values::BLOCK_POS)),
            Family::LevelResource => self
                .resource_id(value)
                .map(|id| format!("{}{id}", crate::runtime::server_values::LEVEL_RESOURCE)),
        })
    }

    /// What `other` is to a family member's `equals`/`compareTo`.
    fn other_server_value(&self, other: &Value) -> Other {
        match other {
            Value::Ref(None)
            | Value::Int(_)
            | Value::Long(_)
            | Value::Float(_)
            | Value::Boxed(_) => Other::Foreign,
            Value::Host(handle) if !crate::runtime::jar_object::is_object_handle(handle) => {
                let key = handle.key();
                if handle.kind() == HostKind::GameValue {
                    if key.starts_with(crate::runtime::server_values::BLOCK_POS)
                        || key.starts_with(crate::runtime::server_values::LEVEL_RESOURCE)
                    {
                        return Other::Key(key.to_owned());
                    }
                    return Other::Foreign;
                }
                // A game object that is no value (an entity, a level, a
                // registry entry) is never a `Vec3i`; a transient one names
                // a class the run may not know.
                match handle.kind() {
                    HostKind::GameTransient => Other::Unknown,
                    _ => Other::Foreign,
                }
            }
            _ => match self.jar_server_value_key(other) {
                Some(Some(key)) => Other::Key(key),
                Some(None) => Other::Unknown,
                None => match other {
                    Value::Ref(Some(text)) if crate::runtime::value_class::is_plain_string(text) => {
                        Other::Foreign
                    }
                    // A jar object whose chain leaves the jar at a JDK class
                    // is no game value; one that leaves it at another game
                    // class (`SectionPos extends Vec3i`) may be.
                    _ => match self.chain_exit(other) {
                        Some(exit) if exit.starts_with("java/") => Other::Foreign,
                        _ => Other::Unknown,
                    },
                },
            },
        }
    }

    /// The first class not held in `value`'s class chain.
    fn chain_exit(&self, value: &Value) -> Option<String> {
        let mut current = crate::runtime::jar_object::object_class(value)?.to_owned();
        for _ in 0..MAX_CHAIN {
            let Some(file) = self.class(&current) else {
                return Some(current);
            };
            if file.super_name.is_empty() {
                return Some("java/lang/Object".into());
            }
            current = file.super_name.clone();
        }
        None
    }

    fn count(&mut self, owner: &str, name: &str, descriptor: &str) -> Result<(), NativeError> {
        self.host
            .unknown_operand_call(&field(owner, name, descriptor))
            .map(|_| ())
    }

    /// `a.equals(b)` for a jar object `a` of the family; `None` for any other
    /// `a`.
    pub(crate) fn jar_server_value_equals(
        &mut self,
        a: &Value,
        b: &Value,
    ) -> Result<Option<bool>, NativeError> {
        let Some(own) = self.jar_server_value_key(a) else {
            return Ok(None);
        };
        if a == b {
            return Ok(Some(true));
        }
        let (Some(own), other) = (own, self.other_server_value(b)) else {
            self.count(VEC3I, "equals", "(Ljava/lang/Object;)Z")?;
            return Ok(Some(false));
        };
        Ok(Some(match other {
            // Keys of the two families never spell each other.
            Other::Key(key) => key == own,
            Other::Foreign => false,
            Other::Unknown => {
                self.count(VEC3I, "equals", "(Ljava/lang/Object;)Z")?;
                false
            }
        }))
    }

    /// `a.hashCode()` for a jar object `a` of the family: `Some(None)` where
    /// it is counted.
    pub(crate) fn jar_server_value_hash(
        &mut self,
        value: &Value,
    ) -> Result<Option<Option<i32>>, NativeError> {
        let Some(own) = self.jar_server_value_key(value) else {
            return Ok(None);
        };
        match own.as_deref().and_then(crate::runtime::server_values::hash_code) {
            Some(hash) => Ok(Some(Some(hash))),
            None => {
                self.count(VEC3I, "hashCode", "()I")?;
                Ok(Some(None))
            }
        }
    }

    /// `a.compareTo(b)` for a jar object `a` of the family (`Vec3i.compareTo`
    /// of the two positions); `None` for any other `a`. Counted where either
    /// position is not known.
    pub(crate) fn jar_server_value_compare(
        &mut self,
        a: &Value,
        b: &Value,
    ) -> Option<Result<i32, NativeError>> {
        let own = self.jar_server_value_key(a)?;
        let other = match self.other_server_value(b) {
            Other::Key(key) => Some(key),
            Other::Foreign | Other::Unknown => None,
        };
        Some(
            match own
                .zip(other)
                .and_then(|(own, other)| crate::runtime::server_values::compare_keys(&own, &other))
            {
                Some(order) => Ok(order),
                None => self.count_compare_to(),
            },
        )
    }

    /// `String.valueOf(value)` for a jar object of the family.
    pub(crate) fn jar_server_value_text(&mut self, value: &Value) -> Option<String> {
        let key = self.jar_server_value_key(value)??;
        let class = crate::runtime::jar_object::object_class(value)?;
        if let Some([x, y, z]) = crate::runtime::server_values::block_pos(&key) {
            return Some(format!("{}{{x={x}, y={y}, z={z}}}", simple_name(class)));
        }
        crate::runtime::server_values::to_string(&key)
    }

    /// The game's own members on a jar object of the family (the module
    /// doc); `None` for any other call, which the host answers (counted).
    pub(crate) fn invoke_jar_server_value(
        &mut self,
        r: &MethodRef,
        op: u8,
        receiver: Option<&Value>,
        args: &[Value],
    ) -> Result<Option<Value>, NativeError> {
        let Some(receiver) = receiver else {
            return Ok(None);
        };
        let Some((family, overridden)) = self.server_value_family(receiver) else {
            return Ok(None);
        };
        let int = |index: usize| match args.get(index) {
            Some(Value::Int(value)) => Some(*value),
            _ => None,
        };
        let double = |index: usize| match args.get(index) {
            Some(Value::Float(value)) => Some(floor(*value)),
            _ => None,
        };
        if op == 0xb7 && r.name == "<init>" {
            let coords = match (family, r.owner.as_str(), r.descriptor.as_str()) {
                (Family::Vec3i { .. }, VEC3I | BLOCK_POS | MUTABLE_BLOCK_POS, "(III)V") => {
                    [int(0), int(1), int(2)]
                }
                (Family::Vec3i { .. }, MUTABLE_BLOCK_POS, "()V") => [Some(0); 3],
                (Family::Vec3i { .. }, MUTABLE_BLOCK_POS, "(DDD)V") => {
                    [double(0), double(1), double(2)]
                }
                (Family::LevelResource, LEVEL_RESOURCE, "(Ljava/lang/String;)V") => {
                    let Some(id) = args.first().cloned() else {
                        return Ok(None);
                    };
                    if let Some(key) = instance_field_key(
                        receiver,
                        &field(LEVEL_RESOURCE, "id", "Ljava/lang/String;"),
                    ) {
                        self.fields.insert(key, id);
                    }
                    // The host records the id too, for the game calls that
                    // take the resource (`getWorldPath`): its constructor
                    // still reaches it.
                    return Ok(None);
                }
                _ => return Ok(None),
            };
            if coords.iter().any(Option::is_none) {
                return Ok(None);
            }
            self.set_coords(receiver, coords);
            return Ok(Some(Value::Void));
        }
        // A member a jar subclass overrides runs as its own (above this
        // model); one Java's answer reaches through it is the subclass's,
        // which is counted, never answered by identity.
        if overridden {
            let counted = matches!(
                (r.name.as_str(), r.descriptor.as_str()),
                ("equals", "(Ljava/lang/Object;)Z")
                    | ("hashCode", "()I")
                    | ("toString", "()Ljava/lang/String;")
            ) || (r.name == "compareTo"
                && crate::runtime::object_methods::is_compare_to(&r.descriptor));
            return if counted {
                self.host.unknown_operand_call(r).map(Some)
            } else {
                Ok(None)
            };
        }
        match family {
            Family::LevelResource => Ok(match (r.name.as_str(), r.descriptor.as_str()) {
                ("id", "()Ljava/lang/String;") => {
                    self.resource_id(receiver).map(|id| Value::Ref(Some(id)))
                }
                _ => self.object_member(r, receiver, args)?,
            }),
            Family::Vec3i { mutable } => {
                let index = ["getX", "getY", "getZ"]
                    .iter()
                    .position(|name| *name == r.name);
                if let (Some(index), "()I") = (index, r.descriptor.as_str()) {
                    return Ok(self
                        .coords(receiver)
                        .map(|coords| Value::Int(coords[index])));
                }
                if mutable
                    && r.descriptor
                        .ends_with(")Lnet/minecraft/core/BlockPos$MutableBlockPos;")
                {
                    let Some(now) = self.coords(receiver) else {
                        return Ok(None);
                    };
                    let mut coords = now.map(Some);
                    match (
                        r.name.as_str(),
                        &r.descriptor[..r.descriptor.find(')').unwrap_or(0) + 1],
                    ) {
                        ("setX", "(I)") => coords[0] = int(0),
                        ("setY", "(I)") => coords[1] = int(0),
                        ("setZ", "(I)") => coords[2] = int(0),
                        ("set", "(III)") => coords = [int(0), int(1), int(2)],
                        ("set", "(DDD)") => coords = [double(0), double(1), double(2)],
                        ("set", "(Lnet/minecraft/core/Vec3i;)") => {
                            let Other::Key(key) = self.other_server_value(&args[0]) else {
                                return Ok(None);
                            };
                            let Some(other) = crate::runtime::server_values::block_pos(&key) else {
                                return Ok(None);
                            };
                            coords = other.map(Some);
                        }
                        _ => return Ok(None),
                    }
                    if coords.iter().any(Option::is_none) {
                        return Ok(None);
                    }
                    self.set_coords(receiver, coords);
                    return Ok(Some(receiver.clone()));
                }
                if r.name == "compareTo"
                    && crate::runtime::object_methods::is_compare_to(&r.descriptor)
                    && let [other] = args
                {
                    return self
                        .jar_server_value_compare(receiver, other)
                        .transpose()
                        .map(|order| order.map(Value::Int));
                }
                self.object_member(r, receiver, args)
            }
        }
    }

    /// `equals`, `hashCode` and `toString` called on a family member.
    fn object_member(
        &mut self,
        r: &MethodRef,
        receiver: &Value,
        args: &[Value],
    ) -> Result<Option<Value>, NativeError> {
        Ok(match (r.name.as_str(), r.descriptor.as_str(), args) {
            ("equals", "(Ljava/lang/Object;)Z", [other]) => self
                .jar_server_value_equals(receiver, other)?
                .map(|equal| Value::Int(i32::from(equal))),
            ("hashCode", "()I", []) => self
                .jar_server_value_hash(receiver)?
                .flatten()
                .map(Value::Int),
            ("toString", "()Ljava/lang/String;", []) => self
                .jar_server_value_text(receiver)
                .map(|text| Value::Ref(Some(text))),
            _ => None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn floor_rounds_down_and_simple_names_drop_the_package() {
        // `Mth.floor`: toward negative infinity, saturating like `(int)`.
        assert_eq!(floor(1.5), 1);
        assert_eq!(floor(-1.5), -2);
        assert_eq!(floor(-2.0), -2);
        assert_eq!(floor(1e300), i32::MAX);
        assert_eq!(simple_name(MUTABLE_BLOCK_POS), "MutableBlockPos");
        assert_eq!(simple_name("demo/MyPos"), "MyPos");
    }
}
