//! Public bridge seam for generated Sunrise game-point listeners.
//!
//! Generated listeners translate a typed record into [`GameDispatch`] and apply the returned
//! answer. This module owns one global Mixin order and keeps Java callback semantics in the
//! existing resident runtime. Java-to-engine re-entry is deliberately refused in PR 1: no
//! Sunrise dispatch token is available inside a Java callback yet.

use std::cell::RefCell;
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
use std::sync::Arc;
use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};

use mod_api::ServerHost;

use crate::runtime::{GameHookContext, GameHookEffect, GameObject, NativeError, Value};
use crate::{HorizonJava, JAR_BUDGET, State};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MethodId<'a>(pub &'a str);
impl<'a> From<&'a str> for MethodId<'a> {
    fn from(value: &'a str) -> Self {
        Self(value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Phase {
    Head,
    Return,
    Tail,
}

/// Stable host-owned identity; its marker is derived from the game's key, never a Rust address.
pub type GameObjectHandle = GameObject;

/// One generated Sunrise point firing.
pub struct GameDispatch<'a> {
    pub method: MethodId<'a>,
    pub phase: Phase,
    pub receiver: Option<GameObjectHandle>,
    pub args: &'a mut [Value],
    /// The real result at RETURN or TAIL. HEAD passes `None`.
    pub actual: Option<Value>,
    pub host: &'a mut dyn ServerHost,
}

impl<'a> GameDispatch<'a> {
    pub fn new(
        method: MethodId<'a>,
        phase: Phase,
        receiver: Option<GameObjectHandle>,
        args: &'a mut [Value],
        actual: Option<Value>,
        host: &'a mut dyn ServerHost,
    ) -> Self {
        Self {
            method,
            phase,
            receiver,
            args,
            actual,
            host,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DispatchAnswer {
    pub cancelled: bool,
    pub returned: Option<Value>,
    /// Copy-out record values. A generated listener writes these back to its Sunrise record.
    pub updated_args: Vec<Value>,
    pub callbacks: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DispatchError(String);
impl DispatchError {
    fn from_error(error: impl std::fmt::Display) -> Self {
        Self(error.to_string())
    }
    pub fn is_reentry_refusal(&self) -> bool {
        self.0.starts_with("Java game-hook re-entry is unsupported")
    }
}
impl std::fmt::Display for DispatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for DispatchError {}

struct PlanSnapshot {
    words: [u64; 4],
}
/// Immutable snapshots are intentionally leaked: an in-flight dispatch may hold any old pointer.
/// Refresh publishes one complete replacement with Release; it never clears live words in place.
#[derive(Debug)]
pub(crate) struct PlanBits {
    ptr: AtomicPtr<PlanSnapshot>,
}
impl Default for PlanBits {
    fn default() -> Self {
        Self {
            ptr: AtomicPtr::new(Box::into_raw(Box::new(PlanSnapshot { words: [0; 4] }))),
        }
    }
}
impl PlanBits {
    fn publish(&self, words: [u64; 4]) {
        self.ptr.store(
            Box::into_raw(Box::new(PlanSnapshot { words })),
            Ordering::Release,
        );
    }
    fn maybe_contains(&self, id: &str) -> bool {
        let (word, bit) = bit_for(id);
        let snapshot = self.ptr.load(Ordering::Acquire);
        // SAFETY: snapshots are never reclaimed, so every acquired pointer remains valid.
        unsafe { (*snapshot).words[word] & (1_u64 << bit) != 0 }
    }
}
fn bit_for(id: &str) -> (usize, u32) {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in id.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    (((hash >> 6) as usize) & 3, (hash & 63) as u32)
}
fn set_bit(words: &mut [u64; 4], id: &str) {
    let (word, bit) = bit_for(id);
    words[word] |= 1_u64 << bit;
}
fn set_target_bit(words: &mut [u64; 4], owner: &str, name: &str, descriptor: &str) {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in owner
        .bytes()
        .chain(std::iter::once(b'#'))
        .chain(name.bytes())
        .chain(std::iter::once(b' ').chain(descriptor.bytes()))
    {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    words[((hash >> 6) as usize) & 3] |= 1_u64 << (hash & 63);
}
pub(crate) fn matches_method(
    id: &str,
    hook: &str,
    owner: &str,
    name: &str,
    descriptor: &str,
) -> bool {
    if id == hook {
        return true;
    }
    let Some(rest) = id.strip_prefix(owner).and_then(|r| r.strip_prefix('#')) else {
        return false;
    };
    let Some(rest) = rest.strip_prefix(name).and_then(|r| r.strip_prefix(' ')) else {
        return false;
    };
    rest == descriptor
}

struct ActiveFrame {
    owner: *const HorizonJava,
    state: std::cell::UnsafeCell<State>,
}
thread_local! { static ACTIVE_FRAMES: RefCell<Vec<ActiveFrame>> = const { RefCell::new(Vec::new()) }; }

impl HorizonJava {
    /// Dispatch one point. The relaxed/immutable snapshot check precedes every lock or allocation.
    pub fn dispatch_game_hook<'a>(
        &self,
        dispatch: GameDispatch<'a>,
    ) -> Result<DispatchAnswer, DispatchError> {
        if !self.plan_bits.maybe_contains(dispatch.method.0) {
            return Ok(DispatchAnswer::default());
        }
        let owner = self as *const HorizonJava;
        if ACTIVE_FRAMES.with(|frames| {
            frames
                .borrow()
                .last()
                .is_some_and(|frame| frame.owner == owner)
        }) {
            return Err(DispatchError::from_error(
                "Java game-hook re-entry is unsupported in PR 1 (Sunrise dispatch token not available inside Java callbacks)",
            ));
        }
        let _gate = self
            .dispatch_gate
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let state = {
            let mut locked = self.lock();
            std::mem::take(&mut *locked)
        };
        ACTIVE_FRAMES.with(|frames| {
            frames.borrow_mut().push(ActiveFrame {
                owner,
                state: std::cell::UnsafeCell::new(state),
            })
        });
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            let state = ACTIVE_FRAMES
                .with(|frames| frames.borrow().last().expect("active frame").state.get());
            // SAFETY: the frame stays on this thread's stack for the closure.
            unsafe { dispatch_state(&mut *state, dispatch) }
        }));
        let frame = ACTIVE_FRAMES.with(|frames| frames.borrow_mut().pop().expect("active frame"));
        *self.lock() = frame.state.into_inner();
        match outcome {
            Ok(result) => result,
            Err(panic) => resume_unwind(panic),
        }
    }

    pub(crate) fn refresh_game_hook_plan_bits(&self) {
        let state = self.lock();
        let mut words = [0; 4];
        for loaded in &state.jars {
            for hook in loaded.hooks.hooks() {
                set_bit(&mut words, hook);
                for entry in loaded.hooks.entries(hook) {
                    set_target_bit(
                        &mut words,
                        &entry.target.owner,
                        &entry.target.name,
                        &entry.target.descriptor,
                    );
                }
            }
        }
        self.plan_bits.publish(words);
    }
}

struct HookTarget {
    jar: usize,
    hook: String,
    priority: i32,
    order: usize,
}
#[cfg(test)]
static STATE_DISPATCHES: AtomicUsize = AtomicUsize::new(0);
#[cfg(test)]
static REENTRY_REFUSALS: AtomicUsize = AtomicUsize::new(0);

fn dispatch_state(
    state: &mut State,
    mut dispatch: GameDispatch<'_>,
) -> Result<DispatchAnswer, DispatchError> {
    #[cfg(test)]
    STATE_DISPATCHES.fetch_add(1, Ordering::Relaxed);
    let facts = Arc::clone(&state.server);
    let dedicated = state.dedicated;
    let anchor = match dispatch.phase {
        Phase::Head => "HEAD",
        Phase::Return => "RETURN",
        Phase::Tail => "TAIL",
    };
    let mut targets = Vec::new();
    for (jar, loaded) in state.jars.iter().enumerate() {
        for hook in loaded.hooks.hooks() {
            let entries = loaded.hooks.entries(hook);
            if let Some(entry) = entries.iter().find(|entry| {
                matches_method(
                    dispatch.method.0,
                    hook,
                    &entry.target.owner,
                    &entry.target.name,
                    &entry.target.descriptor,
                ) && (if entry.anchor.is_empty() {
                    "HEAD"
                } else {
                    entry.anchor.as_str()
                }) == anchor
            }) {
                targets.push(HookTarget {
                    jar,
                    hook: hook.to_owned(),
                    priority: entry.priority,
                    order: entry.order,
                });
            }
        }
    }
    // Lower Mixin priority first, then load order, then declaration order. Hook names never enter
    // this comparator, so the BTreeMap cannot silently reorder callbacks.
    targets.sort_by_key(|target| (target.priority, target.jar, target.order));
    let mut answer = DispatchAnswer::default();
    for target in targets {
        let loaded = &mut state.jars[target.jar];
        let registrations = loaded.hooks.for_anchor(&target.hook, anchor);
        if registrations.is_empty() {
            continue;
        }
        let mut context = GameHookContext::new();
        context.set_server_facts(Arc::clone(&facts));
        context.insert(GameObject::server(dedicated));
        let receiver = dispatch
            .receiver
            .clone()
            .map(|object| context.insert(object));
        let result = crate::caught(
            &format!("{} game hook {}", loaded.row.id, target.hook),
            || {
                loaded.jar.run_game_hook_returning(
                    &registrations,
                    &mut loaded.hook_state,
                    &mut context,
                    &target.hook,
                    receiver,
                    dispatch.args,
                    dispatch.actual.clone(),
                    &mut loaded.host,
                    &crate::runtime::ExecutionBudget::within(JAR_BUDGET),
                )
            },
        );
        let run = match result {
            Ok(Ok(run)) => run,
            Ok(Err(error)) => {
                loaded.row.game_hooks.failed += 1;
                return Err(DispatchError::from_error(error));
            }
            Err(panic) => {
                loaded.row.game_hooks.failed += 1;
                return Err(DispatchError::from_error(format!("panicked: {panic}")));
            }
        };
        loaded.row.game_hooks.ran += run.ran;
        answer.callbacks += run.ran;
        answer.cancelled |= run.cancelled;
        if run.returned.is_some() {
            answer.returned = run.returned;
        }
        answer.updated_args = dispatch.args.to_vec();
        for effect in context.take_effects() {
            apply_effect(dispatch.host, effect);
        }
        if run.cancelled {
            break;
        }
    }
    Ok(answer)
}
fn apply_effect(host: &mut dyn ServerHost, effect: GameHookEffect) {
    match effect {
        GameHookEffect::SystemMessage { player, text } => {
            if let Some(target) = host.player(player) {
                let _ = target.tell(host, mod_api::Text::literal(text));
            }
        }
    }
}
impl From<NativeError> for DispatchError {
    fn from(error: NativeError) -> Self {
        Self::from_error(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn method_bitset_has_no_false_negative_for_target_spelling() {
        let bits = PlanBits::default();
        let mut words = [0; 4];
        set_target_bit(&mut words, "a/b/C", "m", "(I)V");
        bits.publish(words);
        assert!(bits.maybe_contains("a/b/C#m (I)V"));
    }
    #[test]
    fn phase_matching_is_exact() {
        assert_eq!(Phase::Head, Phase::Head);
        assert_ne!(Phase::Return, Phase::Tail);
    }
    #[test]
    fn no_plan_returns_before_state_dispatch() {
        struct NoopHost;
        impl mod_api::Host for NoopHost {}
        mod_api::impl_host_areas!(NoopHost);
        impl mod_api::ServerHost for NoopHost {}
        mod_api::impl_server_host_areas!(NoopHost);
        let java = HorizonJava::default();
        let before = STATE_DISPATCHES.load(Ordering::Relaxed);
        let mut args = [Value::Int(7)];
        let mut host = NoopHost;
        let answer = java
            .dispatch_game_hook(GameDispatch::new(
                MethodId("unplanned_method"),
                Phase::Head,
                None,
                &mut args,
                None,
                &mut host,
            ))
            .unwrap();
        assert_eq!(answer, DispatchAnswer::default());
        assert_eq!(STATE_DISPATCHES.load(Ordering::Relaxed), before);
    }
    #[test]
    fn java_reentry_is_explicitly_refused() {
        let error = DispatchError::from_error(
            "Java game-hook re-entry is unsupported in PR 1 (Sunrise dispatch token not available inside Java callbacks)",
        );
        assert!(error.is_reentry_refusal());
    }
    #[test]
    fn refresh_publishes_complete_snapshots() {
        let bits = PlanBits::default();
        let mut words = [0; 4];
        set_bit(&mut words, "live");
        bits.publish(words);
        let old = bits.maybe_contains("live");
        let mut words = [0; 4];
        set_bit(&mut words, "new");
        bits.publish(words);
        assert!(old);
        assert!(bits.maybe_contains("new"));
    }
}
