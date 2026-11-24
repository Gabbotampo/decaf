//! Virtual dispatch in the JVM's order: the receiver's own body before any
//! host model.
//!
//! `invokevirtual`/`invokeinterface` select the method from the receiver's
//! runtime class (JVMS §5.4.6): the class itself, then its superclasses, then
//! the maximally specific superinterface default. The symbolic owner only
//! fixes the name and descriptor. The interpreter used to ask its host models
//! first and resolve the receiver's class only for the calls none of them
//! answered, so a model keyed by owner or member name answered a call the jar
//! had overridden. Two shapes this produced:
//!
//! * `Map.containsKey` on an object of a jar class that implements `Map`
//!   (a library's own map) went to the host's map model, which knows nothing
//!   of that object and answered from an empty row.
//! * `isEmpty()` on a jar subclass of `AbstractSet` that overrides `isEmpty`
//!   answered `size() == 0` from the `AbstractCollection` template instead of
//!   the override.
//!
//! [`Runtime::jvm_selected_jar_body`] runs before the model chain (after Mixin
//! anchors and invoke hooks, which replace the call site itself, and after the
//! null check). It answers only when the selected method is a jar or linked
//! class's body; a member the receiver inherits from a JDK or game class keeps
//! the existing chain, which models that class's contract.
//!
//! The superinterface step needs to know whether a JDK superclass declares the
//! member, since a superclass declaration beats any interface default. The
//! runtime holds no JDK class files, so [`JDK_SKELETONS`] lists the instance
//! methods the JDK's abstract collection skeletons declare, which are the JDK
//! superclasses library code extends most (fastutil's and Guava's collections
//! extend them). Any other JDK superclass is treated as unknown and keeps the
//! old answer.
//!
//! The superinterface step itself walks the JDK's interfaces as well as the
//! jar's (#627): a jar class that extends `AbstractCollection` inherits
//! `Iterable.forEach`, so a jar interface's `default forEach` beside it
//! conflicts on Java 25. [`Runtime::maximally_specific_default`] runs
//! `HotSpot`'s walk (`defaultMethods.cpp`) over the jar's class files and
//! `jdk_interface_methods.rs`'s Java 25 rows. An interface neither holds (a
//! game's or a loader's) is unknown, and a selection it could change is
//! counted ([`JvmSelection::Uncertain`]), never guessed.

use super::*;
use std::borrow::Cow;

/// `ACC_PRIVATE`, `ACC_PROTECTED`, `ACC_PUBLIC`.
const ACC_PRIVATE: u16 = 0x0002;
const ACC_PROTECTED: u16 = 0x0004;
const ACC_PUBLIC: u16 = 0x0001;
/// `ACC_STATIC`.
const ACC_STATIC: u16 = 0x0008;

/// The non-static, non-private methods each JDK skeleton class declares
/// itself (not inherited), with its JDK superclass.
///
/// Read from JDK 25.0.5 by reflection (`getDeclaredMethods()`, static and
/// private ones dropped): the class's public API, not its code.
pub(crate) const JDK_SKELETONS: &[(&str, &str, &[&str])] = &[
    (
        "java/util/AbstractCollection",
        "java/lang/Object",
        &[
            "add(Ljava/lang/Object;)Z",
            "addAll(Ljava/util/Collection;)Z",
            "clear()V",
            "contains(Ljava/lang/Object;)Z",
            "containsAll(Ljava/util/Collection;)Z",
            "isEmpty()Z",
            "iterator()Ljava/util/Iterator;",
            "remove(Ljava/lang/Object;)Z",
            "removeAll(Ljava/util/Collection;)Z",
            "retainAll(Ljava/util/Collection;)Z",
            "size()I",
            "toArray()[Ljava/lang/Object;",
            "toArray([Ljava/lang/Object;)[Ljava/lang/Object;",
            "toString()Ljava/lang/String;",
        ],
    ),
    (
        "java/util/AbstractList",
        "java/util/AbstractCollection",
        &[
            "add(ILjava/lang/Object;)V",
            "add(Ljava/lang/Object;)Z",
            "addAll(ILjava/util/Collection;)Z",
            "clear()V",
            "equals(Ljava/lang/Object;)Z",
            "get(I)Ljava/lang/Object;",
            "hashCode()I",
            "indexOf(Ljava/lang/Object;)I",
            "iterator()Ljava/util/Iterator;",
            "lastIndexOf(Ljava/lang/Object;)I",
            "listIterator()Ljava/util/ListIterator;",
            "listIterator(I)Ljava/util/ListIterator;",
            "remove(I)Ljava/lang/Object;",
            "removeRange(II)V",
            "set(ILjava/lang/Object;)Ljava/lang/Object;",
            "subList(II)Ljava/util/List;",
        ],
    ),
    (
        "java/util/AbstractSequentialList",
        "java/util/AbstractList",
        &[
            "add(ILjava/lang/Object;)V",
            "addAll(ILjava/util/Collection;)Z",
            "get(I)Ljava/lang/Object;",
            "iterator()Ljava/util/Iterator;",
            "listIterator(I)Ljava/util/ListIterator;",
            "remove(I)Ljava/lang/Object;",
            "set(ILjava/lang/Object;)Ljava/lang/Object;",
        ],
    ),
    (
        "java/util/AbstractSet",
        "java/util/AbstractCollection",
        &[
            "equals(Ljava/lang/Object;)Z",
            "hashCode()I",
            "removeAll(Ljava/util/Collection;)Z",
        ],
    ),
    (
        "java/util/AbstractQueue",
        "java/util/AbstractCollection",
        &[
            "add(Ljava/lang/Object;)Z",
            "addAll(Ljava/util/Collection;)Z",
            "clear()V",
            "element()Ljava/lang/Object;",
            "remove()Ljava/lang/Object;",
        ],
    ),
    (
        "java/util/AbstractMap",
        "java/lang/Object",
        &[
            "clear()V",
            "clone()Ljava/lang/Object;",
            "containsKey(Ljava/lang/Object;)Z",
            "containsValue(Ljava/lang/Object;)Z",
            "entrySet()Ljava/util/Set;",
            "equals(Ljava/lang/Object;)Z",
            "get(Ljava/lang/Object;)Ljava/lang/Object;",
            "hashCode()I",
            "isEmpty()Z",
            "keySet()Ljava/util/Set;",
            "put(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
            "putAll(Ljava/util/Map;)V",
            "remove(Ljava/lang/Object;)Ljava/lang/Object;",
            "size()I",
            "toString()Ljava/lang/String;",
            "values()Ljava/util/Collection;",
        ],
    ),
    (
        "java/util/AbstractMap$SimpleEntry",
        "java/lang/Object",
        &[
            "equals(Ljava/lang/Object;)Z",
            "getKey()Ljava/lang/Object;",
            "getValue()Ljava/lang/Object;",
            "hashCode()I",
            "setValue(Ljava/lang/Object;)Ljava/lang/Object;",
            "toString()Ljava/lang/String;",
        ],
    ),
    (
        "java/util/AbstractMap$SimpleImmutableEntry",
        "java/lang/Object",
        &[
            "equals(Ljava/lang/Object;)Z",
            "getKey()Ljava/lang/Object;",
            "getValue()Ljava/lang/Object;",
            "hashCode()I",
            "setValue(Ljava/lang/Object;)Ljava/lang/Object;",
            "toString()Ljava/lang/String;",
        ],
    ),
];

/// Whether the known JDK skeleton chain proves that `class` is not an enum.
///
/// A class absent from this table is deliberately unknown: the runtime has no
/// JDK class files and must leave contracts such as `EnumSet.allOf` counted.
pub(crate) fn jdk_skeleton_proves_not_enum(class: &str) -> bool {
    let mut current = class;
    let mut known = false;
    for _ in 0..=JDK_SKELETONS.len() {
        if matches!(current, "java/lang/Object" | "java/lang/Record") {
            return known;
        }
        let Some((_, superclass, _)) = JDK_SKELETONS
            .iter()
            .find(|(skeleton, _, _)| *skeleton == current)
        else {
            return false;
        };
        known = true;
        current = superclass;
    }
    false
}

/// The instance methods `java/lang/Object` declares (JDK 25.0.5's `javap -p`,
/// the private ones left out). `Record` declares only abstract `equals`,
/// `hashCode` and `toString`, which are among them.
const OBJECT_METHODS: &[(&str, &str)] = &[
    ("getClass", "()Ljava/lang/Class;"),
    ("hashCode", "()I"),
    ("equals", "(Ljava/lang/Object;)Z"),
    ("clone", "()Ljava/lang/Object;"),
    ("toString", "()Ljava/lang/String;"),
    ("notify", "()V"),
    ("notifyAll", "()V"),
    ("wait", "()V"),
    ("wait", "(J)V"),
    ("wait", "(JI)V"),
    ("finalize", "()V"),
];

/// Whether the JDK class `class`, or a JDK superclass of it, declares the
/// instance method `name descriptor` itself, so that it is selected before
/// any superinterface default. `None` when the runtime does not know the
/// class's declarations.
///
/// `Object` (and `Record`, whose three abstract methods are `Object`'s)
/// declares only [`OBJECT_METHODS`]: a jar interface's abstract
/// `String toString();` beside them leaves `Object.toString` selected on
/// Java 25 (`HotSpot`'s walk stops at a class declaring the member, protected
/// `clone` and `finalize` included), not an `AbstractMethodError`. For every
/// other member the defaults are the next step. `Enum`'s own methods are
/// final (`name`, `ordinal`...).
pub(crate) fn jdk_superclass_declares(class: &str, name: &str, descriptor: &str) -> Option<bool> {
    let mut current = class;
    for _ in 0..JDK_SKELETONS.len() + 1 {
        match current {
            "java/lang/Object" | "java/lang/Record" => {
                return Some(OBJECT_METHODS.contains(&(name, descriptor)));
            }
            "java/lang/Enum" if ENUM_DECLARED_METHODS.contains(&name) => return Some(true),
            "java/lang/Enum" => {
                current = "java/lang/Object";
                continue;
            }
            _ => {}
        }
        let (_, superclass, declared) = JDK_SKELETONS
            .iter()
            .find(|(skeleton, _, _)| *skeleton == current)?;
        if declared.iter().any(|member| {
            member
                .strip_prefix(name)
                .is_some_and(|rest| rest == descriptor)
        }) {
            return Some(true);
        }
        current = superclass;
    }
    None
}

/// Libraries the runtime answers with hand models (`guava_model.rs`,
/// `library_dispatch.rs`'s fastutil shapes, `attachment_codec.rs`), even when
/// a jar ships or links its own copy (the `MapMaker` fixture). Their objects
/// keep the models' answers: running a copy's bytecode for some calls while
/// the models answer the rest would mix two implementations in one object.
/// Interpreting a linked library wholly is a separate decision.
pub(crate) fn library_modelled(class: &str) -> bool {
    [
        "com/google/common/",
        "it/unimi/dsi/fastutil/",
        "com/mojang/serialization/",
        "com/mojang/datafixers/",
    ]
    .iter()
    .any(|root| class.starts_with(root))
}

/// The package part of an internal class name (`a/b/C` → `a/b`).
fn package_of(class: &str) -> &str {
    class.rsplit_once('/').map_or("", |(package, _)| package)
}

/// Where JVMS §5.4.6 selection of a call on a jar object leads
/// ([`Runtime::jvm_selection`]).
pub(crate) enum JvmSelection {
    /// A jar or linked class's body: run it.
    Body(String),
    /// A member of a JDK or game class (or one the runtime cannot read the
    /// receiver's class for): the model chain answers it, as before.
    Host,
    /// A superinterface default whose selection depends on an interface the
    /// runtime cannot read (a game's or a loader's): which method Java runs,
    /// or whether it throws, is unknown, so the call is counted.
    Uncertain,
}

/// [`Runtime::jvm_dispatch_or_count`]'s answer for a call site that runs it.
pub(crate) enum Dispatched {
    /// Run this jar or linked class's body.
    Body(String),
    /// The call was counted (`Host::unknown_operand_call`); this is its answer.
    Counted(Value),
}

/// Interface visits one default-method walk may make. `HotSpot`'s walk
/// revisits an interface once per path that reaches it; the JDK's deepest
/// hierarchies (`NavigableMap`, `BlockingDeque`) take a few dozen. A walk
/// that runs out is counted, never cut short into a guess.
const MAX_DEFAULT_WALK_VISITS: usize = 4096;

impl<H: Host> Runtime<'_, H> {
    /// [`Self::jvm_selection`] for a caller that only runs a selected body:
    /// the jar or linked class whose body an `invokevirtual`/
    /// `invokeinterface` of `r` on `receiver` selects, or `Ok(None)` to leave
    /// the call to the model chain (and then to [`Self::dispatch_owner`]).
    ///
    /// An uncertain selection refuses with [`NativeError::UnknownHostCall`]
    /// naming `r`: these callers (menu and screen models, a JDK body's
    /// callback) have no answer to give for it. The interpreter's own call
    /// sites count it instead ([`Self::jvm_dispatch_or_count`]).
    pub(crate) fn jvm_selected_jar_body(
        &self,
        r: &MethodRef,
        opcode: u8,
        receiver: Option<&Value>,
    ) -> Result<Option<String>, NativeError> {
        match self.jvm_selection(r, opcode, receiver)? {
            JvmSelection::Body(owner) => Ok(Some(owner)),
            JvmSelection::Host => Ok(None),
            JvmSelection::Uncertain => Err(NativeError::UnknownHostCall(r.clone())),
        }
    }

    /// [`Self::jvm_selection`] for a call site that answers the call: a jar
    /// body to run, the counted answer of an uncertain selection, or
    /// `Ok(None)` for the model chain. Out of line, so the interpreter's frame
    /// keeps its measured size (`fixture_deep_recursion`).
    #[inline(never)]
    pub(crate) fn jvm_dispatch_or_count(
        &mut self,
        r: &MethodRef,
        opcode: u8,
        receiver: Option<&Value>,
    ) -> Result<Option<Dispatched>, NativeError> {
        match self.jvm_selection(r, opcode, receiver)? {
            JvmSelection::Body(owner) => Ok(Some(Dispatched::Body(owner))),
            JvmSelection::Host => Ok(None),
            JvmSelection::Uncertain => self
                .host
                .unknown_operand_call(r)
                .map(|value| Some(Dispatched::Counted(value))),
        }
    }

    /// Where an `invokevirtual`/`invokeinterface` of `r` on `receiver` goes:
    /// a jar or linked class's body, the model chain, or (when it depends on
    /// an interface the runtime cannot read) nowhere certain.
    ///
    /// The receiver must be an object this run constructed of a class the
    /// runtime holds. Selection follows JVMS §5.4.6:
    ///
    /// 1. the most derived class on the receiver's superclass chain whose
    ///    declaration overrides the resolved method. Overriding is JVMS
    ///    §5.4.5's, transitive included: a package-private method is
    ///    overridden only from its own package, unless an overrider in that
    ///    package widened it to public or protected, after which any package
    ///    overrides it;
    /// 2. otherwise, when the chain leaves the jar at a JDK superclass known
    ///    not to declare the member, the maximally specific superinterface
    ///    default ([`Self::maximally_specific_default`]), the JDK's interfaces
    ///    included. A jar default runs; a JDK one is the model chain's. Two
    ///    of them throw `IncompatibleClassChangeError`; only abstract ones
    ///    throw `AbstractMethodError`, with HotSpot's messages. Any of these
    ///    that an unreadable interface could change is
    ///    [`JvmSelection::Uncertain`].
    ///
    /// A selected abstract class method is the model chain's (the existing
    /// chain keeps its answer). A resolved method that is private or static
    /// is not dispatched virtually at all.
    pub(crate) fn jvm_selection(
        &self,
        r: &MethodRef,
        opcode: u8,
        receiver: Option<&Value>,
    ) -> Result<JvmSelection, NativeError> {
        use JvmSelection::{Body, Host, Uncertain};
        if !matches!(opcode, 0xb6 | 0xb9) || r.name.starts_with('<') {
            return Ok(Host);
        }
        let Some(class) = receiver.and_then(crate::runtime::jar_object::object_class) else {
            return Ok(Host);
        };
        // A jar's own copy of a host-modelled class (an unrelocated Gson) is
        // answered by the host's model, as `new` and every other path treat it.
        if self.host.models_class(class) || library_modelled(class) || self.class(class).is_none() {
            return Ok(Host);
        }
        // The resolved method (JVMS §5.4.3.3): its access decides whether the
        // call is virtual and which declarations override it.
        let resolved = self
            .member_owner(&r.owner, &r.name, &r.descriptor)
            .and_then(|(owner, jar)| jar.then_some(owner))
            .and_then(|owner| {
                let access = self.class(&owner)?.method(&r.name, &r.descriptor)?.access;
                Some((owner, access))
            });
        if let Some((_, access)) = &resolved
            && access & (ACC_PRIVATE | ACC_STATIC) != 0
        {
            return Ok(Host);
        }
        // The receiver's superclass chain while the runtime holds it, and the
        // platform class it leaves the jar at.
        let mut chain = Vec::new();
        let mut left_at = None;
        let mut current = class.to_owned();
        for _ in 0..MAX_SUPERCLASS_WALK {
            let Some(file) = self.class(&current) else {
                left_at = Some(current);
                break;
            };
            let superclass = file.super_name.clone();
            chain.push(current);
            if superclass.is_empty() {
                break;
            }
            current = superclass;
        }
        let declaration = |class: &str| {
            self.class(class)
                .and_then(|file| file.method(&r.name, &r.descriptor))
                .filter(|method| !method.is_static() && method.access & ACC_PRIVATE == 0)
        };
        let selected = match &resolved {
            Some((owner, access)) if chain.contains(owner) => {
                // Walk down from the resolved class, keeping every declaration
                // that overrides it (with its package and whether it is
                // public/protected); a declaration overrides when one of them
                // is open to it.
                let position = chain.iter().position(|class| class == owner).unwrap_or(0);
                let mut overridden = vec![(package_of(owner), is_open(*access))];
                let mut selected = owner.clone();
                for class in chain[..position].iter().rev() {
                    if let Some(method) = declaration(class)
                        && overridden
                            .iter()
                            .any(|(package, open)| *open || *package == package_of(class))
                    {
                        overridden.push((package_of(class), is_open(method.access)));
                        selected = class.clone();
                    }
                }
                Some(selected)
            }
            // An interface's (public) method, or a platform one: the first
            // declaration on the chain overrides it.
            _ => chain
                .iter()
                .find(|class| declaration(class).is_some())
                .cloned(),
        };
        if let Some(selected) = selected {
            let has_body = declaration(&selected).is_some_and(|method| method.code.is_some());
            return Ok(if has_body { Body(selected) } else { Host });
        }
        // A JDK superclass that declares the member (or one the runtime
        // cannot tell about) answers it through the model chain.
        if let Some(platform) = &left_at
            && jdk_superclass_declares(platform, &r.name, &r.descriptor) != Some(false)
        {
            return Ok(Host);
        }
        match self.maximally_specific_default(&chain, left_at.as_deref(), &r.name, &r.descriptor) {
            DefaultSelection::Jar(owner) => Ok(Body(owner)),
            // `Iterable.forEach` on a jar collection: the JDK's default, which
            // the model chain answers.
            DefaultSelection::Jdk | DefaultSelection::None => Ok(Host),
            DefaultSelection::Uncertain => Ok(Uncertain),
            DefaultSelection::Conflict(owners) => Err(NativeError::UncaughtException {
                class: "java/lang/IncompatibleClassChangeError".into(),
                message: Some(format!(
                    "Conflicting default methods: {}",
                    owners
                        .iter()
                        .map(|owner| format!("{owner}.{}", r.name))
                        .collect::<Vec<_>>()
                        .join(" ")
                )),
                frame: None,
            }),
            DefaultSelection::Abstract => Err(NativeError::UncaughtException {
                class: "java/lang/AbstractMethodError".into(),
                message: Some(format!(
                    "Method {class}.{}{} is abstract",
                    r.name, r.descriptor
                )),
                frame: None,
            }),
        }
    }

    /// JVMS §5.4.3.3's maximally specific superinterface methods of the
    /// classes in `chain` and of the JDK superclasses from `left_at` up, for
    /// `name descriptor`, and §5.4.6 step 3's choice among them.
    ///
    /// `chain` is the receiver's superclass chain while the runtime holds it
    /// (the receiver first); `left_at` is the JDK class it leaves the jar at
    /// (`java/util/AbstractCollection`), which must be one
    /// [`jdk_superclass_declares`] knows not to declare the member. No class
    /// on either part declares it, so the walk is the superinterfaces'.
    ///
    /// This is `HotSpot`'s walk (`defaultMethods.cpp`,
    /// `FindMethodsByErasedSig`), because its order is observable: the
    /// conflict message lists every declaration it meets (disqualified and
    /// abstract ones too) in the order it meets them. It is depth first, a
    /// class's superclass (and all above it) before the class's own
    /// interfaces, each interface before its superinterfaces in class-file
    /// order, an interface reached twice visited twice. A declaration is
    /// disqualified when a declaring interface lies between it and the class
    /// on some path (it is not maximally specific); static and private ones
    /// do not count. Exactly one qualified default is selected; two or more
    /// conflict; abstract ones alone throw `HotSpot`'s overpass error
    /// (`Method <class>.<name><descriptor> is abstract`). That error exists
    /// only when some interface in the hierarchy declares a concrete instance
    /// method (`_has_nonstatic_concrete_methods`: no default anywhere, and
    /// `HotSpot` generates no overpass, so the link resolver's own message
    /// is thrown instead); without one the outcome is
    /// [`DefaultSelection::Uncertain`] rather than the wrong text.
    ///
    /// A JDK interface's declarations are Java 25's rows
    /// (`jdk_interface_methods.rs`), which win over a jar's copy of the
    /// same interface (the JDK's is the one ever loaded); a jar's are its
    /// class files. An interface neither holds is unknown: what it or its
    /// superinterfaces declare is not known. Below a declaring interface
    /// (the jar's `interface Mine extends GameApi { default .. }`) it cannot
    /// change a selection or an abstract-only answer, only a conflict's
    /// message. Anywhere else it could add a default, so the outcome is
    /// [`DefaultSelection::Uncertain`], unless nothing at all declares the
    /// member, which leaves it to the model chain as before.
    pub(crate) fn maximally_specific_default(
        &self,
        chain: &[String],
        left_at: Option<&str>,
        name: &str,
        descriptor: &str,
    ) -> DefaultSelection {
        // The classes' direct interfaces, topmost class first: HotSpot
        // finishes a class's superclass before its interfaces.
        let mut classes: Vec<Vec<Cow<'static, str>>> = Vec::new();
        let mut jdk = left_at;
        while let Some(class) = jdk {
            let Some(row) = crate::runtime::jdk_interface_methods::superclass(class) else {
                // A JDK superclass without a row: its interfaces are unknown.
                return DefaultSelection::Uncertain;
            };
            if classes.len() > MAX_SUPERCLASS_WALK {
                return DefaultSelection::Uncertain;
            }
            classes.push(row.interfaces.iter().copied().map(Cow::Borrowed).collect());
            jdk = row.superclass;
        }
        classes.reverse();
        // `chain` ends without a JDK superclass only at a jar class with no
        // superclass (`java/lang/Object` itself); a walk that ran out of its
        // bound first did not see the whole hierarchy.
        if left_at.is_none()
            && chain
                .last()
                .and_then(|class| self.class(class))
                .is_some_and(|file| !file.super_name.is_empty())
        {
            return DefaultSelection::Uncertain;
        }
        for class in chain.iter().rev() {
            let Some(file) = self.class(class) else {
                return DefaultSelection::Uncertain;
            };
            classes.push(file.interface_names().into_iter().map(Cow::Owned).collect());
        }
        let mut walk = DefaultWalk::default();
        for interfaces in classes {
            if !self.walk_superinterfaces(interfaces, name, descriptor, &mut walk) {
                return DefaultSelection::Uncertain;
            }
        }
        walk.outcome()
    }

    /// One class's part of [`Self::maximally_specific_default`]'s walk:
    /// `interfaces` (its direct ones, in order) and everything above them,
    /// depth first, into `walk`. `false` when the walk ran out of
    /// [`MAX_DEFAULT_WALK_VISITS`].
    fn walk_superinterfaces(
        &self,
        interfaces: Vec<Cow<'static, str>>,
        name: &str,
        descriptor: &str,
        walk: &mut DefaultWalk,
    ) -> bool {
        // `(interface, qualified)`: whether no declaring interface lies
        // between it and the class on this path. Children are pushed in
        // reverse, so they pop in class-file order: a recursive walk's
        // preorder without its native stack.
        let mut stack: Vec<(Cow<'static, str>, bool)> = interfaces
            .into_iter()
            .rev()
            .map(|interface| (interface, true))
            .collect();
        while let Some((interface, qualified)) = stack.pop() {
            if walk.visits >= MAX_DEFAULT_WALK_VISITS {
                return false;
            }
            walk.visits += 1;
            let (declared, parents): (Option<bool>, Vec<Cow<'static, str>>) =
                if let Some(row) = crate::runtime::jdk_interface_methods::interface(&interface) {
                    use crate::runtime::jdk_interface_methods::Kind;
                    let declared = match row.declares(name, descriptor) {
                        Some(Kind::Default) => Some(true),
                        Some(Kind::Abstract) => Some(false),
                        Some(Kind::Static | Kind::Private) | None => None,
                    };
                    walk.concrete |= row.has_concrete_instance_method();
                    let parents = row.superinterfaces.iter().copied().map(Cow::Borrowed);
                    (declared, parents.collect())
                } else if let Some(file) = self.class(&interface) {
                    let declared = file
                        .method(name, descriptor)
                        .filter(|method| !method.is_static() && method.access & ACC_PRIVATE == 0)
                        .map(|method| method.code.is_some());
                    // `HotSpot`'s `declares_nonstatic_concrete_methods`: a
                    // default or a private instance method.
                    walk.concrete |= file
                        .methods
                        .iter()
                        .any(|method| !method.is_static() && method.code.is_some());
                    (
                        declared,
                        file.interface_names().into_iter().map(Cow::Owned).collect(),
                    )
                } else {
                    walk.unknown = true;
                    walk.unknown_qualified |= qualified;
                    continue;
                };
            let mut above = qualified;
            if let Some(is_default) = declared {
                walk.record(&interface, is_default, qualified);
                above = false;
            }
            stack.extend(parents.into_iter().rev().map(|parent| (parent, above)));
        }
        true
    }
}

/// The declarations [`Runtime::maximally_specific_default`]'s walk met.
#[derive(Default)]
struct DefaultWalk {
    /// `(interface, has a body, qualified)`, in the order first met: a
    /// declaration met again keeps its place, and is disqualified if any
    /// path disqualifies it.
    members: Vec<(String, bool, bool)>,
    /// An unknown interface was met.
    unknown: bool,
    /// An unknown interface was met with no declaring interface below it on
    /// that path, so it could hold a maximally specific declaration.
    unknown_qualified: bool,
    /// An interface the walk met declares a concrete instance method, of any
    /// name: `HotSpot` then runs its default-method pass for the class, which
    /// is what throws the overpass `AbstractMethodError`.
    concrete: bool,
    visits: usize,
}

impl DefaultWalk {
    fn record(&mut self, interface: &str, has_body: bool, qualified: bool) {
        match self
            .members
            .iter_mut()
            .find(|(owner, _, _)| owner == interface)
        {
            Some(member) => member.2 &= qualified,
            None => self
                .members
                .push((interface.to_owned(), has_body, qualified)),
        }
    }

    /// JVMS §5.4.6 step 3 over the walk.
    fn outcome(self) -> DefaultSelection {
        if self.members.is_empty() {
            // Nothing the runtime can read declares it: the model chain
            // answers, as for a member of a game interface.
            return DefaultSelection::None;
        }
        let mut defaults = self
            .members
            .iter()
            .filter(|(_, has_body, qualified)| *has_body && *qualified);
        let first = defaults.next();
        match (first, defaults.next()) {
            (Some(_), Some(_)) if self.unknown => DefaultSelection::Uncertain,
            (Some(_), Some(_)) => DefaultSelection::Conflict(
                self.members
                    .into_iter()
                    .map(|(owner, _, _)| owner)
                    .collect(),
            ),
            _ if self.unknown_qualified => DefaultSelection::Uncertain,
            (Some((owner, _, _)), None) => match crate::runtime::jdk_interface_methods::interface(owner) {
                Some(_) => DefaultSelection::Jdk,
                None => DefaultSelection::Jar(owner.clone()),
            },
            (None, _) if self.concrete => DefaultSelection::Abstract,
            // No default anywhere (an unknown interface could hold one): the
            // link resolver's message, which names the call site's resolved
            // method; not modelled, so counted.
            (None, _) => DefaultSelection::Uncertain,
        }
    }
}

/// [`Runtime::maximally_specific_default`]'s outcome.
pub(crate) enum DefaultSelection {
    /// The one non-abstract maximally specific default, a jar or linked
    /// interface's.
    Jar(String),
    /// The one non-abstract maximally specific default, a JDK interface's
    /// (`java/lang/Iterable`): the model chain answers it.
    Jdk,
    /// Nothing the runtime can read declares the member.
    None,
    /// Two or more non-abstract maximally specific defaults: every
    /// declaration the walk met, in `HotSpot`'s order (its message lists
    /// them all).
    Conflict(Vec<String>),
    /// Only abstract maximally specific declarations, in a hierarchy where
    /// `HotSpot` throws its overpass `AbstractMethodError` for them.
    Abstract,
    /// The outcome depends on an interface the runtime cannot read.
    Uncertain,
}

/// Whether a method's access lets any package override it (public or
/// protected).
fn is_open(access: u16) -> bool {
    access & (ACC_PUBLIC | ACC_PROTECTED) != 0
}

impl<H: Host> Runtime<'_, H> {
    /// Whether `getfield r` on `receiver` reads a field the JVM gave its
    /// default value: an instance field a jar or linked class declares, on an
    /// object this run constructed of that class or a subclass, that nothing
    /// has written yet (JVMS §2.3, §2.4: every field starts at `0`, `false` or
    /// `null`).
    ///
    /// Such a read used to go to the host as if the field belonged to a
    /// platform class, which counted an unknown call for an ordinary
    /// `count++` on a fresh object. `r.owner` is the declaring class
    /// (`Runtime::field_owner` resolved it). A platform superclass's field
    /// is still the host's to answer.
    pub(crate) fn reads_unwritten_jar_field(&self, r: &MethodRef, receiver: &Value) -> bool {
        let Some(class) = crate::runtime::jar_object::object_class(receiver) else {
            return false;
        };
        let declared = self.class(&r.owner).is_some_and(|file| {
            file.fields.iter().any(|field| {
                field.name == r.name && field.descriptor == r.descriptor && !field.is_static()
            })
        }) || jdk_skeletons::skeleton_field_owner(&r.owner, &r.name, &r.descriptor)
            == Some(r.owner.as_str());
        declared && self.class_is_instance(class, &r.owner)
    }
}
