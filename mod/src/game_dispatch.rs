//! Public bridge seam for generated Sunrise game-point listeners.
//!
//! PR 2 will generate one typed listener per reviewed Sunrise point.  Those listeners should only
//! translate the point's record into [`GameDispatch`] and apply [`DispatchAnswer`]; all Java/Mixin
//! ordering and callback semantics stay in the resident runtime chain.

use std::cell::RefCell;
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use mod_api::ServerHost;

use crate::runtime::{GameHookContext, GameHookEffect, GameObject, NativeError, Value};
use crate::{HorizonJava, JAR_BUDGET, State};

/// A stable identifier supplied by a generated listener.
///
/// The borrowed form is intentional: a point listener passes a static generated id and the
/// no-plan path hashes it without allocating.  PR 2 owns the id-to-point manifest.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MethodId<'a>(pub &'a str);

impl<'a> From<&'a str> for MethodId<'a> {
    fn from(value: &'a str) -> Self {
        Self(value)
    }
}

/// The phase at which a generated point listener fires.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Phase {
    Head,
    Return,
    Tail,
}

/// The stable game-object identity handed to Java as the receiver.
///
/// This is the existing host-owned object description.  Its marker is derived from the stable
/// game key, never from a transient Rust address, so a callback can retain and compare it across
/// firings while the host says the object is live.
pub type GameObjectHandle = GameObject;

/// The input to one generated game-point listener.
pub struct GameDispatch<'a> {
    pub method: MethodId<'a>,
    pub phase: Phase,
    pub receiver: Option<GameObjectHandle>,
    /// API-record values copied into the runtime's Java argument shape.
    pub args: &'a mut [Value],
    /// The method result at `RETURN`/`TAIL`; `None` at `HEAD`.
    pub actual: Option<Value>,
    /// The Sunrise host used to apply effects produced by Java callbacks.
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

/// What the Java callback chain decided at a point.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DispatchAnswer {
    /// A cancellable HEAD callback stopped the point body, or a RETURN callback replaced it.
    pub cancelled: bool,
    /// The value to use when a callback cancelled or overrode the method result.
    pub returned: Option<Value>,
    /// Number of Java callbacks that ran across all matching jars.
    pub callbacks: usize,
}

/// An error raised while adapting or running a game-point callback chain.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DispatchError(String);

impl DispatchError {
    fn from_error(error: impl std::fmt::Display) -> Self {
        Self(error.to_string())
    }
}

impl std::fmt::Display for DispatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for DispatchError {}

/// A compact relaxed bitset.  False positives are harmless (the exact registration lookup still
/// follows); a false negative would skip a Java plan, so every loaded method spelling is marked.
#[derive(Debug)]
pub(crate) struct PlanBits([AtomicU64; 4]);

impl Default for PlanBits {
    fn default() -> Self {
        Self(std::array::from_fn(|_| AtomicU64::new(0)))
    }
}

impl PlanBits {
    fn clear(&self) {
        for word in &self.0 {
            word.store(0, Ordering::Relaxed);
        }
    }

    pub(crate) fn mark(&self, id: &str) {
        let (word, bit) = bit_for(id);
        self.0[word].fetch_or(1_u64 << bit, Ordering::Relaxed);
    }

    fn maybe_contains(&self, id: &str) -> bool {
        let (word, bit) = bit_for(id);
        self.0[word].load(Ordering::Relaxed) & (1_u64 << bit) != 0
    }
}

fn bit_for(id: &str) -> (usize, u32) {
    // FNV-1a over bytes is stable across builds and does not allocate.
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in id.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    (((hash >> 6) as usize) & 3, (hash & 63) as u32)
}

/// Hashes the alternate target spelling without constructing `owner#name descriptor`.
impl PlanBits {
    fn mark_target(&self, owner: &str, name: &str, descriptor: &str) {
        let (word, bit) = bit_for_parts(owner, name, descriptor);
        self.0[word].fetch_or(1_u64 << bit, Ordering::Relaxed);
    }
}

fn bit_for_parts(owner: &str, name: &str, descriptor: &str) -> (usize, u32) {
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
    (((hash >> 6) as usize) & 3, (hash & 63) as u32)
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
    let Some(rest) = id
        .strip_prefix(owner)
        .and_then(|rest| rest.strip_prefix('#'))
    else {
        return false;
    };
    let Some(rest) = rest
        .strip_prefix(name)
        .and_then(|rest| rest.strip_prefix(' '))
    else {
        return false;
    };
    rest == descriptor
}

struct ActiveFrame {
    owner: *const HorizonJava,
    state: std::cell::UnsafeCell<State>,
}

thread_local! {
    static ACTIVE_FRAMES: RefCell<Vec<ActiveFrame>> = const { RefCell::new(Vec::new()) };
}

impl HorizonJava {
    /// Dispatch one generated game-point listener through every matching Java plan.
    ///
    /// The relaxed bitset is the complete no-plan fast path: it runs before the dispatch mutex,
    /// state lock, context creation, or argument allocation.  A live frame is thread-local and
    /// reentrant.  A Java callback that reaches another generated point sees the same frame and
    /// mutably nests into it; the outer call resumes with the nested jar state already merged.
    pub fn dispatch_game_hook<'a>(
        &self,
        dispatch: GameDispatch<'a>,
    ) -> Result<DispatchAnswer, DispatchError> {
        if !self.plan_bits.maybe_contains(dispatch.method.0) {
            return Ok(DispatchAnswer::default());
        }

        let owner = self as *const HorizonJava;
        if let Some(state) = ACTIVE_FRAMES.with(|frames| {
            frames
                .borrow()
                .last()
                .filter(|frame| frame.owner == owner)
                .map(|frame| frame.state.get())
        }) {
            // SAFETY: the frame is on this thread's active stack, and nested dispatch is the
            // deliberate reentrant operation.  No other thread can observe its raw pointer.
            return unsafe { dispatch_state(&mut *state, dispatch) };
        }

        // Serialize top-level frames across threads.  The state mutex is released while Java is
        // running, so a callback can re-enter this object without a same-thread mutex deadlock.
        let _gate = self
            .dispatch_gate
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let state = {
            let mut locked = self.lock();
            std::mem::take(&mut *locked)
        };
        let frame = ActiveFrame {
            owner,
            state: std::cell::UnsafeCell::new(state),
        };
        ACTIVE_FRAMES.with(|frames| frames.borrow_mut().push(frame));
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            let state = ACTIVE_FRAMES.with(|frames| {
                frames
                    .borrow()
                    .last()
                    .expect("active dispatch frame")
                    .state
                    .get()
            });
            // SAFETY: the frame remains pinned in ACTIVE_FRAMES for this whole closure.
            unsafe { dispatch_state(&mut *state, dispatch) }
        }));
        let frame = ACTIVE_FRAMES.with(|frames| frames.borrow_mut().pop().expect("active frame"));
        let state = frame.state.into_inner();
        *self.lock() = state;
        match outcome {
            Ok(result) => result,
            Err(panic) => resume_unwind(panic),
        }
    }

    /// Recompute the fast-path index after loading or replacing Java plans.
    pub(crate) fn refresh_game_hook_plan_bits(&self) {
        self.plan_bits.clear();
        let state = self.lock();
        for loaded in &state.jars {
            for hook in loaded.hooks.hooks() {
                self.plan_bits.mark(hook);
                for entry in loaded.hooks.entries(hook) {
                    self.plan_bits.mark_target(
                        &entry.target.owner,
                        &entry.target.name,
                        &entry.target.descriptor,
                    );
                }
            }
        }
    }
}

#[cfg(test)]
static STATE_DISPATCHES: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

fn dispatch_state(
    state: &mut State,
    dispatch: GameDispatch<'_>,
) -> Result<DispatchAnswer, DispatchError> {
    #[cfg(test)]
    STATE_DISPATCHES.fetch_add(1, Ordering::Relaxed);
    let facts = Arc::clone(&state.server);
    let dedicated = state.dedicated;
    let mut answer = DispatchAnswer::default();
    let phase = dispatch.phase;
    for loaded in &mut state.jars {
        let matching_hooks: Vec<String> = loaded
            .hooks
            .hooks()
            .filter(|hook| {
                loaded.hooks.entries(hook).iter().any(|entry| {
                    matches_method(
                        dispatch.method.0,
                        hook,
                        &entry.target.owner,
                        &entry.target.name,
                        &entry.target.descriptor,
                    ) && phase_matches(entry.anchor.as_str(), phase)
                })
            })
            .map(str::to_owned)
            .collect();
        for hook in matching_hooks {
            let mut context = GameHookContext::new();
            context.set_server_facts(Arc::clone(&facts));
            context.insert(GameObject::server(dedicated));
            let receiver = dispatch
                .receiver
                .clone()
                .map(|object| context.insert(object));
            let actual = dispatch.actual.clone();
            let budget = crate::runtime::ExecutionBudget::within(JAR_BUDGET);
            let result = crate::caught(&format!("{} game hook {hook}", loaded.row.id), || {
                loaded.jar.run_game_hook_returning(
                    &loaded.hooks,
                    &mut loaded.hook_state,
                    &mut context,
                    &hook,
                    receiver,
                    dispatch.args,
                    actual,
                    &mut loaded.host,
                    &budget,
                )
            });
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
            for effect in context.take_effects() {
                apply_effect(dispatch.host, effect);
            }
            if run.cancelled {
                return Ok(answer);
            }
        }
    }
    Ok(answer)
}

fn phase_matches(anchor: &str, phase: Phase) -> bool {
    match phase {
        Phase::Head => anchor.is_empty() || anchor == "HEAD",
        Phase::Return => anchor == "RETURN",
        Phase::Tail => anchor == "TAIL" || anchor == "RETURN",
    }
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
        bits.mark_target("a/b/C", "m", "(I)V");
        assert!(bits.maybe_contains("a/b/C#m (I)V"));
    }

    #[test]
    fn phase_matching_preserves_head_return_and_tail_contracts() {
        assert!(phase_matches("HEAD", Phase::Head));
        assert!(phase_matches("RETURN", Phase::Return));
        assert!(phase_matches("RETURN", Phase::Tail));
        assert!(!phase_matches("TAIL", Phase::Return));
    }

    struct NoopHost;
    impl mod_api::Host for NoopHost {}
    mod_api::impl_host_areas!(NoopHost);
    impl mod_api::ServerHost for NoopHost {}
    mod_api::impl_server_host_areas!(NoopHost);

    #[test]
    fn no_plan_returns_before_state_dispatch() {
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
            .expect("no-plan dispatch is a successful no-op");
        assert_eq!(answer, DispatchAnswer::default());
        assert_eq!(STATE_DISPATCHES.load(Ordering::Relaxed), before);
    }
}
