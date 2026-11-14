//! A bound on the host's own recursive walks over jar values (#250).
//!
//! The host prints, hashes, compares and copies nested values with Rust recursion: a list's
//! `toString` prints each element, which may be a list, whose `toString`... The depth of that
//! nesting is the jar's to choose, and a cycle makes it infinite: `l.add(l); l.toString()`
//! overflowed the native stack and aborted the whole process. On the JVM the same walks are Java
//! recursion, and a cycle ends in a `StackOverflowError` the jar can catch.
//!
//! Every walk that can nest on jar-controlled depth enters one [`Level`] per nesting step. A
//! level is refused, with Java's `StackOverflowError` (an ordinary throw: the nearest jar
//! handler for it, or for `Error`/`Throwable`, catches it), once less than
//! [`frame_bound::FRAME_STACK_FLOOR`] of the thread's stack is left: the floor a new jar frame
//! needs, so a refused walk always leaves room for the handler that catches it and for the host
//! work beneath it. Where the platform does not report the stack, [`UNKNOWN_STACK_DEPTH`]
//! levels; on any stack, at most [`MAX_HOST_WALK_DEPTH`].
//!
//! The cyclic cases Java prints instead of recursing (`(this Collection)`, `(this Map)`,
//! `Arrays.deepToString`'s `[...]`) are answered by the walks themselves, before they recurse.

use std::cell::Cell;

use crate::NativeError;
use crate::frame_bound::FRAME_STACK_FLOOR;
use crate::java_regex::stack;

/// The most nested walk levels on any stack: a cycle on a very large stack is refused here
/// rather than filling it.
pub(crate) const MAX_HOST_WALK_DEPTH: usize = 4096;

/// The levels allowed where the platform does not report the thread's stack: a few hundred
/// nested elements, as a 2 MiB thread fits with room to spare.
pub(crate) const UNKNOWN_STACK_DEPTH: usize = 256;

thread_local! {
    static DEPTH: Cell<usize> = const { Cell::new(0) };
}

/// One level of a nested walk; dropping it leaves the level.
#[must_use]
pub(crate) struct Level(());

impl Drop for Level {
    fn drop(&mut self) {
        DEPTH.with(|depth| depth.set(depth.get().saturating_sub(1)));
    }
}

/// The address of a fresh local: where the stack is now.
#[inline(always)]
fn stack_position() -> usize {
    let marker = 0u8;
    std::hint::black_box(&marker) as *const u8 as usize
}

/// Enter one level of a nested walk, or Java's `StackOverflowError` where the thread's stack
/// (or the level count) is spent.
pub(crate) fn enter() -> Result<Level, NativeError> {
    let depth = DEPTH.with(Cell::get);
    let spent = depth >= MAX_HOST_WALK_DEPTH
        || match stack::remaining(stack_position()) {
            Some(left) => left < FRAME_STACK_FLOOR,
            None => depth >= UNKNOWN_STACK_DEPTH,
        };
    if spent {
        return Err(stack_overflow());
    }
    DEPTH.with(|cell| cell.set(depth + 1));
    Ok(Level(()))
}

/// Java's `StackOverflowError`, as a throw the jar can catch.
pub(crate) fn stack_overflow() -> NativeError {
    NativeError::UncaughtException {
        class: STACK_OVERFLOW.into(),
        message: None,
        frame: None,
    }
}

pub(crate) const STACK_OVERFLOW: &str = "java/lang/StackOverflowError";

/// Whether `error` is a `StackOverflowError` a walk threw: a caller that otherwise answers a
/// failed nested read by counting it must let this one through, as Java's would.
pub(crate) fn is_stack_overflow(error: &NativeError) -> bool {
    matches!(error, NativeError::UncaughtException { class, .. } if class == STACK_OVERFLOW)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A walk that never ends is refused with a `StackOverflowError`, and leaving every level
    /// puts the count back where it started.
    #[test]
    fn an_endless_walk_is_refused_and_unwinds() {
        fn walk(levels: &mut usize) -> Result<(), NativeError> {
            let _level = enter()?;
            *levels += 1;
            walk(levels)
        }
        let before = DEPTH.with(Cell::get);
        let mut levels = 0;
        let error = walk(&mut levels).unwrap_err();
        assert!(is_stack_overflow(&error), "{error:?}");
        assert!(levels > 0 && levels <= MAX_HOST_WALK_DEPTH, "{levels}");
        assert_eq!(DEPTH.with(Cell::get), before);
    }

    /// A composed predicate as deep as a jar builds it (`p = p.negate()` in a loop) ends in a
    /// `StackOverflowError`, not a native overflow; a shallow one answers.
    #[test]
    fn a_deep_predicate_chain_is_refused() {
        let deep = format!("{}@indy:x", "@fn:predicate.not:".repeat(200_000));
        let error = crate::predicate_marker_value(&deep).unwrap_err();
        assert!(is_stack_overflow(&error), "{error:?}");
        let shallow = format!("{}@indy:x", "@fn:predicate.not:".repeat(3));
        assert_eq!(crate::predicate_marker_value(&shallow).ok(), Some(false));
    }
}
