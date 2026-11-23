//! Mixin's class-loading rule: which classes of a mod real Fabric refuses to load.
//!
//! sponge-mixin 0.17.5 (`MixinProcessor.applyMixins`) runs on every class Knot loads.
//! A class whose binary name starts with a selected Mixin config's package
//! (`MixinConfig.packageMatch`, the config's `package` plus `.`) is refused with
//! `IllegalClassLoadError` ("... cannot be referenced directly" / "... is in a defined
//! mixin package ..."), unless:
//!
//! - a coprocessor passes it through: an accessor mixin a selected config lists
//!   (`MixinCoprocessorPassthrough` registers `MixinInfo.isLoadable()`, true only for
//!   `SubType.Accessor`, an interface mixin whose methods are all `@Accessor`/`@Invoker`);
//! - it is an `InjectionPoint` subclass, or an `ITargetSelectorDynamic` implementation
//!   (`ClassInfo.hasSuperClass`, interfaces included only for the interface).
//!
//! fabric-loader 0.20.1's Knot rethrows the error as `RuntimeException: Mixin
//! transformation of <class> failed` from the class load. What a later load of the same
//! class does is not modelled (the processor's state after the throw is not), so the
//! runtime counts such a load rather than throwing, and runs nothing for it.
//!
//! A mixin's own code is merged into its target, and Mixin rewrites a merged method's
//! references to its own class (`MixinTargetContext.transformMethodRef` /
//! `transformFieldRef`, and inner classes through `innerClasses`): those never load the
//! mixin class. So a reference is refused only from code that is not merged with the
//! class it names ([`Runtime::is_unloadable_reference`]).
//!
//! The one point that sees every bytecode class reference is the constant pool: `new`,
//! `anewarray`, `multianewarray`, `checkcast`, `instanceof`, `ldc` of a class, method type
//! or handle, field and method references and `invokedynamic` all resolve a pool entry.
//! Each frame looks up once which of its class's pool entries would load a refused class
//! ([`Runtime::unloadable_pool_entries`]); an instruction resolving one is counted and
//! skipped with its stack effect ([`Runtime::skip_unloadable_instruction`]). A reflective
//! load (`Class.forName`) and a loader's entrypoint load go through
//! [`Runtime::is_unloadable_by_name`], with no merged exemption: Mixin rewrites bytecode
//! references, not strings.
//!
//! Mixin config plugins run inside Mixin's own selection and application, where the
//! processor is re-entrant and loads pass through: nothing is refused while one runs.

use super::*;
use std::rc::Rc;

/// Mixin's custom injection-point base class; a subclass may live in a mixin package.
const INJECTION_POINT: &str = "org/spongepowered/asm/mixin/injection/InjectionPoint";
/// Mixin's dynamic target-selector interface; an implementation may live there too.
const TARGET_SELECTOR_DYNAMIC: &str =
    "org/spongepowered/asm/mixin/injection/selectors/ITargetSelectorDynamic";

/// The name of a counted class load ([`Runtime::skip_unloadable_instruction`]).
const LOAD: &str = "<load>";

/// The classes resolving pool entry `index` of `file` loads (JVMS §5.4.3): a class
/// entry's class (an array's element class), a field or method reference's class, a
/// method handle's member's class, every class a method type's descriptor names, and for
/// an `invokedynamic` its call-site descriptor's classes, bootstrap method and static
/// arguments.
pub(crate) fn pool_entry_classes(file: &ClassFile, index: u16) -> Vec<String> {
    let mut out = Vec::new();
    collect_pool_entry_classes(file, index, &mut out, 0);
    out
}

fn collect_pool_entry_classes(file: &ClassFile, index: u16, out: &mut Vec<String>, depth: u8) {
    if depth > 4 {
        return;
    }
    match file.pool.get(index as usize) {
        Some(Cp::Class(name)) => {
            if let Ok(name) = utf8(&file.pool, *name) {
                match name.strip_prefix('[') {
                    Some(_) => out.extend(descriptor_classes(&name)),
                    None => out.push(name),
                }
            }
        }
        Some(
            Cp::FieldRef { class, .. }
            | Cp::MethodRef { class, .. }
            | Cp::InterfaceMethodRef { class, .. },
        ) => collect_pool_entry_classes(file, *class, out, depth + 1),
        Some(Cp::MethodHandle {
            reference_index, ..
        }) => collect_pool_entry_classes(file, *reference_index, out, depth + 1),
        Some(Cp::MethodType { descriptor }) => {
            if let Ok(descriptor) = utf8(&file.pool, *descriptor) {
                out.extend(descriptor_classes(&descriptor));
            }
        }
        Some(Cp::InvokeDynamic { bootstrap, nt }) => {
            if let Some(Cp::NameType { descriptor, .. }) = file.pool.get(*nt as usize)
                && let Ok(descriptor) = utf8(&file.pool, *descriptor)
            {
                out.extend(descriptor_classes(&descriptor));
            }
            if let Some(method) = file.bootstrap_methods.get(*bootstrap as usize) {
                collect_pool_entry_classes(file, method.method_ref, out, depth + 1);
                for argument in &method.arguments {
                    collect_pool_entry_classes(file, *argument, out, depth + 1);
                }
            }
        }
        _ => {}
    }
}

/// Every class a field or method descriptor names (`Lfoo/Bar;`, arrays' elements).
fn descriptor_classes(descriptor: &str) -> Vec<String> {
    let mut classes = Vec::new();
    let mut rest = descriptor;
    while let Some(start) = rest.find('L') {
        let after = &rest[start + 1..];
        let Some(end) = after.find(';') else {
            break;
        };
        classes.push(after[..end].to_owned());
        rest = &after[end + 1..];
    }
    classes
}

/// Opcodes whose operand is a constant-pool index they resolve, with the index's width
/// and the instruction's length after the opcode byte.
fn pool_operand(op: u8) -> Option<(bool, usize)> {
    match op {
        // ldc: a one-byte index.
        0x12 => Some((false, 1)),
        // ldc_w; get/putstatic, get/putfield, invokevirtual/special/static; new;
        // anewarray; checkcast; instanceof.
        0x13 | 0xb2..=0xb8 | 0xbb | 0xbd | 0xc0 | 0xc1 => Some((true, 2)),
        // invokeinterface and invokedynamic carry two more bytes.
        0xb9 | 0xba => Some((true, 4)),
        // multianewarray: index and dimensions.
        0xc5 => Some((true, 3)),
        _ => None,
    }
}

impl<H: Host> Runtime<'_, H> {
    /// The internal-name prefixes (`demo/`) of every Mixin config selected on this side,
    /// the primary jar's and each linked jar's.
    fn mixin_package_prefixes(&self) -> &[String] {
        self.mixin_index.package_prefixes.get_or_init(|| {
            let Some(side) = self.mixin_index.side else {
                return Vec::new();
            };
            let selected = |configs: &[MixinConfig], sides: &BTreeMap<String, MixinSide>| {
                configs
                    .iter()
                    .filter(|config| sides.get(&config.path).is_none_or(|only| *only == side))
                    .filter_map(|config| config.package.as_deref())
                    .filter(|package| !package.is_empty())
                    .map(|package| format!("{}/", package.replace('.', "/").trim_end_matches('/')))
                    .collect::<Vec<_>>()
            };
            let mut prefixes = selected(&self.mixin_index.configs, &self.mixin_index.config_sides);
            for set in self.linked.mixin_config_sets() {
                prefixes.extend(selected(&set.configs, &set.config_sides));
            }
            prefixes.sort();
            prefixes.dedup();
            prefixes
        })
    }

    /// Whether loading class `name` (internal name) by name, from anywhere, is refused:
    /// it is in a selected mixin package and no coprocessor or library-type exemption
    /// applies (module docs).
    #[inline(never)]
    pub(crate) fn is_unloadable_by_name(&self, name: &str) -> bool {
        if crate::mixin_plugins::running_config_plugins() {
            return false;
        }
        let name = match name.strip_prefix('[') {
            Some(_) => match descriptor_classes(name).into_iter().next() {
                Some(element) => element,
                None => return false,
            },
            None => name.to_owned(),
        };
        self.mixin_package_prefixes()
            .iter()
            .any(|prefix| name.starts_with(prefix.as_str()))
            && !self.is_passthrough_accessor(&name)
            && !self.is_mixin_library_subtype(&name)
    }

    /// Whether code in `frame_class` resolving a reference to class `named` is refused:
    /// [`Self::is_unloadable_by_name`], unless the frame's code is merged with that class
    /// (module docs): the class itself, a nested class of the same mixin (its inner and
    /// Decaf's woven `M$$HorizonBridge$...` classes), one of the mixin's targets (Decaf's
    /// woven callback calls stand where Mixin merged the callback), or a subclass.
    pub(crate) fn is_unloadable_reference(&self, frame_class: &str, named: &str) -> bool {
        if frame_class == named || !self.is_unloadable_by_name(named) {
            return false;
        }
        let Some(mixin) = self.mixin_root(named) else {
            return true;
        };
        let nested_in_mixin = frame_class == mixin
            || frame_class
                .strip_prefix(mixin.as_str())
                .is_some_and(|rest| rest.starts_with('$'));
        let targets_frame = self
            .mixin_targets(&mixin)
            .iter()
            .any(|target| target == frame_class);
        !(nested_in_mixin || targets_frame || self.extends_jar_class(frame_class, &mixin))
    }

    /// The `@Mixin` class `class` is or is nested in (`M`, `M$1`, `M$$HorizonBridge$...`).
    fn mixin_root(&self, class: &str) -> Option<String> {
        let mut candidate = class;
        loop {
            let is_mixin = self.class(candidate).is_some_and(|file| {
                file.annotations
                    .iter()
                    .any(|annotation| annotation.descriptor == MIXIN_ANNOTATION_DESCRIPTOR)
            });
            if is_mixin {
                return Some(candidate.to_owned());
            }
            candidate = &candidate[..candidate.rfind('$')?];
        }
    }

    /// Whether jar class `class`'s superclass chain includes `ancestor`.
    fn extends_jar_class(&self, class: &str, ancestor: &str) -> bool {
        let mut current = self.class(class).map(|file| file.super_name.clone());
        for _ in 0..MAX_SUPERCLASS_WALK {
            let Some(name) = current else {
                return false;
            };
            if name == ancestor {
                return true;
            }
            current = self.class(&name).map(|file| file.super_name.clone());
        }
        false
    }

    /// An accessor mixin a selected config lists: an interface `@Mixin` whose every
    /// non-synthetic method is an `@Accessor` or `@Invoker` (`MixinInfo.getVariant`'s
    /// `ACCESSOR`), which `MixinCoprocessorPassthrough` lets load.
    fn is_passthrough_accessor(&self, class: &str) -> bool {
        let Some(file) = self.class(class) else {
            return false;
        };
        let interface =
            crate::checkcast::class_access(file).is_some_and(|access| access & 0x0200 != 0);
        let is_mixin = file
            .annotations
            .iter()
            .any(|annotation| annotation.descriptor == MIXIN_ANNOTATION_DESCRIPTOR);
        if !interface || !is_mixin {
            return false;
        }
        let (_, methods) = crate::game_hooks::member_annotations(file).unwrap_or_default();
        let all_accessors = file
            .methods
            .iter()
            .filter(|method| method.access & 0x1000 == 0)
            .all(|method| {
                methods
                    .get(&format!("{}|{}", method.name, method.descriptor))
                    .is_some_and(|descriptors| {
                        descriptors.iter().any(|descriptor| {
                            descriptor == MIXIN_ACCESSOR_DESCRIPTOR
                                || descriptor == MIXIN_INVOKER_DESCRIPTOR
                        })
                    })
            });
        let listed = self.mixin_index.side.is_some_and(|side| {
            crate::listing_mixin_config(
                &self.mixin_index.configs,
                &self.mixin_index.config_sides,
                class,
                side,
            )
            .is_some()
        }) || self
            .linked
            .mixin_config_sets()
            .iter()
            .any(|set| set.classes.iter().any(|listed| listed == class));
        all_accessors && listed
    }

    /// An `InjectionPoint` subclass (superclass chain) or an `ITargetSelectorDynamic`
    /// implementation (any supertype), which Mixin lets a mixin package hold.
    fn is_mixin_library_subtype(&self, class: &str) -> bool {
        let mut superclass = self.class(class).map(|file| file.super_name.clone());
        for _ in 0..MAX_SUPERCLASS_WALK {
            let Some(name) = superclass else {
                break;
            };
            if name == INJECTION_POINT {
                return true;
            }
            superclass = self.class(&name).map(|file| file.super_name.clone());
        }
        let mut pending = vec![class.to_owned()];
        let mut seen = BTreeSet::new();
        while let Some(current) = pending.pop() {
            if current == TARGET_SELECTOR_DYNAMIC {
                return true;
            }
            if seen.len() >= MAX_SUPERCLASS_WALK || !seen.insert(current.clone()) {
                continue;
            }
            if let Some(file) = self.class(&current) {
                pending.push(file.super_name.clone());
                pending.extend(file.interface_names());
            }
        }
        false
    }

    /// The pool entries of `class` (whose frames run `file`) that load a class this code
    /// may not load ([`Self::is_unloadable_reference`]), or `None` when there are none.
    /// Cached per class for the runtime's life.
    #[inline(never)]
    pub(crate) fn unloadable_pool_entries(
        &self,
        class: &str,
        file: &ClassFile,
    ) -> Option<Rc<BTreeSet<u16>>> {
        if crate::mixin_plugins::running_config_plugins()
            || self.mixin_package_prefixes().is_empty()
        {
            return None;
        }
        if let Some(cached) = self.mixin_index.unloadable_pools.borrow().get(class) {
            return cached.clone();
        }
        let mut refused = BTreeSet::new();
        for index in 1..file.pool.len() {
            let Ok(index) = u16::try_from(index) else {
                break;
            };
            if pool_entry_classes(file, index)
                .iter()
                .any(|named| self.is_unloadable_reference(class, named))
            {
                refused.insert(index);
            }
        }
        let entries = (!refused.is_empty()).then(|| Rc::new(refused));
        self.mixin_index
            .unloadable_pools
            .borrow_mut()
            .insert(class.to_owned(), entries.clone());
        entries
    }

    /// Count an instruction of `class` (whose code is `code`, the opcode `op` just read,
    /// `pc` past it) that resolves a refused pool entry, and give it its stack effect with
    /// the counted placeholder as its result, so nothing it names runs. Returns the next
    /// instruction's `pc`, or `None` when the instruction resolves no refused entry.
    #[inline(never)]
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn skip_unloadable_instruction(
        &mut self,
        refused: &BTreeSet<u16>,
        class: &str,
        file: &ClassFile,
        op: u8,
        code: &[u8],
        pc: usize,
        stack: &mut Vec<Value>,
    ) -> Result<Option<usize>, NativeError> {
        let Some((wide, length)) = pool_operand(op) else {
            return Ok(None);
        };
        let index = if wide {
            u16::from_be_bytes([
                *code.get(pc).ok_or(NativeError::Truncated)?,
                *code.get(pc + 1).ok_or(NativeError::Truncated)?,
            ])
        } else {
            u16::from(*code.get(pc).ok_or(NativeError::Truncated)?)
        };
        if !refused.contains(&index) {
            return Ok(None);
        }
        // `checkcast` and `instanceof` of `null` resolve nothing (JVMS §6.5): the
        // instruction runs as usual.
        if matches!(op, 0xc0 | 0xc1) && matches!(stack.last(), Some(Value::Ref(None))) {
            return Ok(None);
        }
        let named = pool_entry_classes(file, index)
            .into_iter()
            .find(|named| self.is_unloadable_reference(class, named))
            .unwrap_or_default();
        let element = |name: &str| {
            if name.starts_with('[') {
                name.to_owned()
            } else {
                format!("L{name};")
            }
        };
        // (values popped, result descriptor)
        let (pops, result): (usize, Option<String>) = match op {
            0x12 | 0x13 => (
                0,
                Some(
                    match file.pool.get(index as usize) {
                        Some(Cp::MethodType { .. }) => "Ljava/lang/invoke/MethodType;",
                        Some(Cp::MethodHandle { .. }) => "Ljava/lang/invoke/MethodHandle;",
                        _ => "Ljava/lang/Class;",
                    }
                    .to_owned(),
                ),
            ),
            0xb2 => (0, Some(file.field_ref(index)?.descriptor)),
            0xb3 => (1, None),
            0xb4 => (1, Some(file.field_ref(index)?.descriptor)),
            0xb5 => (2, None),
            0xb6..=0xb9 => {
                let descriptor = file.method_ref(index)?.descriptor;
                let receiver = usize::from(op != 0xb8);
                let returns = return_desc(&descriptor);
                (
                    arg_count(&descriptor) + receiver,
                    (returns != "V").then(|| returns.to_owned()),
                )
            }
            0xba => {
                let descriptor = file.invoke_dynamic_descriptor(index)?;
                let returns = return_desc(&descriptor);
                (
                    arg_count(&descriptor),
                    (returns != "V").then(|| returns.to_owned()),
                )
            }
            0xbb => (0, Some(element(&named))),
            0xbd => (1, Some(format!("[{}", element(&named)))),
            0xc0 => (1, Some(element(&named))),
            0xc1 => (1, Some("I".to_owned())),
            0xc5 => {
                let dimensions = *code.get(pc + 2).ok_or(NativeError::Truncated)?;
                let array = match file.pool.get(index as usize) {
                    Some(Cp::Class(name)) => utf8(&file.pool, *name).ok(),
                    _ => None,
                }
                .unwrap_or_else(|| format!("[{}", element(&named)));
                (usize::from(dimensions), Some(array))
            }
            _ => return Ok(None),
        };
        for _ in 0..pops {
            stack.pop().ok_or_else(|| NativeError::StackUnderflow {
                class: class.to_owned(),
                method: String::new(),
                pc,
            })?;
        }
        let load = MethodRef {
            owner: named,
            name: LOAD.to_owned(),
            descriptor: format!("(){}", result.as_deref().unwrap_or("V")),
        };
        let counted = self.host.unknown_operand_call(&load)?;
        if result.is_some() {
            stack.push(counted);
        }
        Ok(Some(pc + length))
    }

    /// Count a reflective load (`Class.forName`) of a class real Fabric refuses
    /// ([`Self::is_unloadable_by_name`]): the counted placeholder stands for the class.
    #[inline(never)]
    pub(crate) fn count_unloadable_class_by_name(
        &mut self,
        name: &str,
    ) -> Result<Option<Value>, NativeError> {
        if !self.is_unloadable_by_name(name) {
            return Ok(None);
        }
        self.host
            .unknown_operand_call(&MethodRef {
                owner: name.to_owned(),
                name: LOAD.to_owned(),
                descriptor: "()Ljava/lang/Class;".to_owned(),
            })
            .map(Some)
    }
}
