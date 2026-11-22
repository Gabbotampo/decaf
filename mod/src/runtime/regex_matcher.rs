//! `java.util.regex.Matcher`: `find()`, `find(int)`, `matches()`, `reset()`,
//! `group`, `start`, `end` and `groupCount`, answered over the position the
//! matcher's earlier calls left (`java_regex::Cursor`).
//!
//! A `Matcher` handle is a value (`host_jdk_constant::matcher`): its pattern
//! and input never change, but `find()` must resume where the last one
//! stopped, so the position lives here, by the handle's serial. A fresh
//! matcher has no entry. The table is process-wide because a handle outlives
//! one invocation (a jar keeps a `Matcher` in a field) and a serial is unique
//! for the process. It holds at most [`MAX_TRACKED`] matchers: past that it is
//! emptied, and every matcher made before then is [`Tracked::Unknown`], like
//! one this module cannot follow.
//!
//! A matcher this module cannot follow answers nothing more: its next calls
//! are counted, never guessed. That is a matcher after a `find()` the engine
//! refuses (an unsupported pattern or input, a budget), after the `find()`
//! past an empty match at the end of the input
//! (`java_regex::Found::NoThenUnmodelled`), one over a `StringBuilder`'s text
//! (Java's would see later appends), and one a call changed in a way not
//! modelled: `lookingAt`, `region`, `usePattern`, `reset(CharSequence)`,
//! `replaceAll`, `appendReplacement`, `results` and the rest.

use std::collections::BTreeMap;
use std::sync::{Mutex, MutexGuard};

use crate::runtime::host_jdk_constant::{MatcherParts, matcher_parts, next_matcher_serial};
use crate::runtime::java_regex::{self, Cursor, Found, Regex};
use crate::runtime::regex_calls::thrown;
use crate::runtime::{MethodRef, NativeError, StdHost, Value, expect_int};

/// Matchers whose position is kept; at the bound the table is emptied.
const MAX_TRACKED: usize = 16_384;

/// One matcher's state.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Tracked {
    Cursor(Cursor),
    /// Not followed: a call that needs its position is counted.
    Unknown,
}

#[derive(Debug)]
struct Table {
    tracked: BTreeMap<u64, Tracked>,
    /// A matcher with a smaller serial and no entry may have been dropped
    /// when the table was emptied.
    forgotten_below: u64,
}

static TABLE: Mutex<Table> = Mutex::new(Table {
    tracked: BTreeMap::new(),
    forgotten_below: 0,
});

fn table() -> MutexGuard<'static, Table> {
    TABLE
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

impl Table {
    /// The state of matcher `serial`; a fresh one has no entry.
    fn load(&self, serial: u64) -> Tracked {
        match self.tracked.get(&serial) {
            Some(state) => state.clone(),
            None if serial < self.forgotten_below => Tracked::Unknown,
            None => Tracked::Cursor(Cursor::default()),
        }
    }

    /// Keep `state` for `serial`. Past `max` matchers the table is emptied:
    /// every matcher made before `next_serial` that is not stored again is
    /// then unknown.
    fn store(&mut self, serial: u64, state: Tracked, max: usize, next_serial: u64) {
        if state == Tracked::Cursor(Cursor::default()) {
            self.tracked.remove(&serial);
            return;
        }
        if self.tracked.len() >= max && !self.tracked.contains_key(&serial) {
            self.tracked.clear();
            self.forgotten_below = next_serial;
        }
        self.tracked.insert(serial, state);
    }
}

fn load(serial: u64) -> Tracked {
    table().load(serial)
}

fn store(serial: u64, state: Tracked) {
    table().store(serial, state, MAX_TRACKED, next_matcher_serial());
}

fn boolean(value: bool) -> Option<Value> {
    Some(Value::Int(i32::from(value)))
}

/// A call on a `Matcher` handle; `Ok(None)` leaves it counted.
pub(crate) fn invoke(
    r: &MethodRef,
    receiver: Option<&Value>,
    args: &[Value],
) -> Result<Option<Value>, NativeError> {
    let Some(parts) = receiver.and_then(matcher_parts) else {
        return Ok(None);
    };
    let int_argument = || expect_int(r, StdHost::argument(r, args, 0)?);
    match (r.name.as_str(), r.descriptor.as_str()) {
        ("matches", "()Z") => matches(&parts),
        ("find", "()Z") => find(&parts, None),
        ("find", "(I)Z") => find(&parts, Some(int_argument()?)),
        ("reset", "()Ljava/util/regex/Matcher;") => reset(&parts, receiver),
        ("group", "()Ljava/lang/String;") => query(&parts, Query::Group(0)),
        ("group", "(I)Ljava/lang/String;") => query(&parts, Query::Group(int_argument()?)),
        ("start", "()I") => query(&parts, Query::Start(0)),
        ("start", "(I)I") => query(&parts, Query::Start(int_argument()?)),
        ("end", "()I") => query(&parts, Query::End(0)),
        ("end", "(I)I") => query(&parts, Query::End(int_argument()?)),
        ("groupCount", "()I") => query(&parts, Query::GroupCount),
        // Anything else may change the matcher (`lookingAt`, `region`,
        // `usePattern`, `reset(CharSequence)`, `replaceAll`...) in a way its
        // position does not describe.
        _ => {
            if let Some(receiver) = receiver {
                note_host_call(r, receiver);
            }
            Ok(None)
        }
    }
}

/// A call on a matcher that no model here answered and that reaches the host
/// (which counts it): unless it only reads the matcher (`toString`,
/// `group(String)`, `regionEnd`...), the matcher may have changed, so its
/// position is no longer known and its later calls are counted too.
pub(crate) fn note_host_call(r: &MethodRef, receiver: &Value) {
    if r.owner != "java/util/regex/Matcher" || reads_only(r.name.as_str()) {
        return;
    }
    if let Some(parts) = matcher_parts(receiver)
        && parts.fixed
    {
        store(parts.serial, Tracked::Unknown);
    }
}

/// `Matcher` methods that read the matcher and change nothing.
fn reads_only(name: &str) -> bool {
    matches!(
        name,
        "toString"
            | "hashCode"
            | "equals"
            | "getClass"
            | "pattern"
            | "group"
            | "start"
            | "end"
            | "groupCount"
            | "hasTransparentBounds"
            | "hasAnchoringBounds"
            | "regionStart"
            | "regionEnd"
            | "hitEnd"
            | "requireEnd"
            | "hasMatch"
            | "toMatchResult"
            | "namedGroups"
    )
}

/// The position to continue from; `None` when this matcher cannot be followed.
fn followed(parts: &MatcherParts) -> Option<Cursor> {
    if !parts.fixed {
        return None;
    }
    match load(parts.serial) {
        Tracked::Cursor(cursor) => Some(cursor),
        Tracked::Unknown => None,
    }
}

/// `find()` (`from` is `None`) and `find(int)`.
fn find(parts: &MatcherParts, from: Option<i32>) -> Result<Option<Value>, NativeError> {
    let Some(mut cursor) = followed(parts) else {
        return Ok(None);
    };
    let outcome = Regex::compile(&parts.pattern, 0).and_then(|regex| match from {
        None => cursor.find(&regex, &parts.input).map(Ok),
        Some(from) => cursor.find_from(&regex, &parts.input, from),
    });
    match outcome {
        Ok(Ok(found)) => {
            store(
                parts.serial,
                if found == Found::NoThenUnmodelled {
                    Tracked::Unknown
                } else {
                    Tracked::Cursor(cursor)
                },
            );
            Ok(boolean(found == Found::Yes))
        }
        // `find(int)` out of range throws and leaves the matcher as it was.
        Ok(Err(exception)) => Err(thrown(exception)),
        Err(_) => {
            store(parts.serial, Tracked::Unknown);
            Ok(None)
        }
    }
}

/// `matches()`: its answer needs no position, but it leaves one. A matcher
/// that is not followed is answered from its text alone, as before.
fn matches(parts: &MatcherParts) -> Result<Option<Value>, NativeError> {
    let Some(mut cursor) = followed(parts) else {
        return Ok(java_regex::matches(&parts.pattern, &parts.input)
            .ok()
            .and_then(boolean));
    };
    let outcome =
        Regex::compile(&parts.pattern, 0).and_then(|regex| cursor.matches(&regex, &parts.input));
    match outcome {
        Ok(matched) => {
            store(parts.serial, Tracked::Cursor(cursor));
            Ok(boolean(matched))
        }
        Err(_) => {
            store(parts.serial, Tracked::Unknown);
            Ok(None)
        }
    }
}

fn reset(parts: &MatcherParts, receiver: Option<&Value>) -> Result<Option<Value>, NativeError> {
    let Some(mut cursor) = followed(parts) else {
        return Ok(None);
    };
    cursor.reset();
    store(parts.serial, Tracked::Cursor(cursor));
    Ok(receiver.cloned())
}

#[derive(Clone, Copy)]
enum Query {
    Group(i32),
    Start(i32),
    End(i32),
    GroupCount,
}

fn query(parts: &MatcherParts, query: Query) -> Result<Option<Value>, NativeError> {
    let Some(cursor) = followed(parts) else {
        return Ok(None);
    };
    let Ok(regex) = Regex::compile(&parts.pattern, 0) else {
        return Ok(None);
    };
    let answer = match query {
        Query::GroupCount => return Ok(Some(Value::Int(regex.group_count() as i32))),
        Query::Group(n) => cursor
            .group(&regex, &parts.input, n)
            .map(|group| group.map(Value::Ref)),
        Query::Start(n) => cursor.start(&regex, n).map(|start| start.map(Value::Int)),
        Query::End(n) => cursor.end(&regex, n).map(|end| end.map(Value::Int)),
    };
    match answer {
        Ok(Ok(value)) => Ok(Some(value)),
        Ok(Err(exception)) => Err(thrown(exception)),
        // A group whose value is not Java's to promise, or an input the
        // engine cannot read.
        Err(_) => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::host_jdk_constant::matcher;

    const HEX: &str = "#([0-9a-fA-F]{6})";

    fn call(receiver: &Value, name: &str, descriptor: &str, args: &[Value]) -> Option<Value> {
        let r = MethodRef {
            owner: "java/util/regex/Matcher".into(),
            name: name.into(),
            descriptor: descriptor.into(),
        };
        invoke(&r, Some(receiver), args).unwrap()
    }

    fn find(receiver: &Value) -> Option<Value> {
        call(receiver, "find", "()Z", &[])
    }

    fn group(receiver: &Value) -> Option<Value> {
        call(receiver, "group", "()Ljava/lang/String;", &[])
    }

    #[test]
    fn find_and_group_follow_the_matcher() {
        let m = matcher(HEX, "#111111 #222222", true);
        assert_eq!(find(&m), Some(Value::Int(1)));
        assert_eq!(group(&m), Some(Value::Ref(Some("#111111".into()))));
        assert_eq!(find(&m), Some(Value::Int(1)));
        assert_eq!(group(&m), Some(Value::Ref(Some("#222222".into()))));
        assert_eq!(find(&m), Some(Value::Int(0)));
    }

    #[test]
    fn a_call_that_changes_the_matcher_unmodelled_counts_the_rest() {
        let m = matcher(HEX, "#111111 #222222", true);
        assert_eq!(find(&m), Some(Value::Int(1)));
        assert_eq!(call(&m, "lookingAt", "()Z", &[]), None);
        assert_eq!(find(&m), None);
        assert_eq!(group(&m), None);
        assert_eq!(call(&m, "groupCount", "()I", &[]), None);
        assert_eq!(call(&m, "reset", "()Ljava/util/regex/Matcher;", &[]), None);
    }

    #[test]
    fn a_call_that_only_reads_leaves_the_position() {
        let m = matcher(HEX, "#111111 #222222", true);
        assert_eq!(find(&m), Some(Value::Int(1)));
        assert_eq!(call(&m, "regionEnd", "()I", &[]), None);
        assert_eq!(call(&m, "hitEnd", "()Z", &[]), None);
        assert_eq!(group(&m), Some(Value::Ref(Some("#111111".into()))));
    }

    #[test]
    fn a_builders_text_may_change_so_only_matches_answers() {
        let m = matcher(HEX, "#111111", false);
        assert_eq!(call(&m, "matches", "()Z", &[]), Some(Value::Int(1)));
        assert_eq!(find(&m), None);
        assert_eq!(group(&m), None);
        assert_eq!(call(&m, "groupCount", "()I", &[]), None);
    }

    #[test]
    fn an_unsupported_pattern_is_counted_and_stays_so() {
        // A group Java refuses to compile here is the engine's to reject.
        let m = matcher("(?<n>a", "a", true);
        assert_eq!(find(&m), None);
        assert_eq!(find(&m), None);
        assert_eq!(group(&m), None);
    }

    #[test]
    fn a_search_past_an_empty_match_at_the_end_answers_false_then_counts() {
        let m = matcher("x*", "ab", true);
        for _ in 0..3 {
            assert_eq!(find(&m), Some(Value::Int(1)));
        }
        assert_eq!(find(&m), Some(Value::Int(0)));
        assert_eq!(find(&m), None);
        assert_eq!(call(&m, "start", "()I", &[]), None);
    }

    #[test]
    fn a_tainted_group_is_counted_without_losing_the_position() {
        // Group 1 is set inside a lookahead: its value is not promised.
        let m = matcher("(?=(a))a", "aa", true);
        assert_eq!(find(&m), Some(Value::Int(1)));
        assert_eq!(
            call(&m, "group", "(I)Ljava/lang/String;", &[Value::Int(1)]),
            None
        );
        assert_eq!(group(&m), Some(Value::Ref(Some("a".into()))));
        assert_eq!(find(&m), Some(Value::Int(1)));
    }

    #[test]
    fn a_miss_that_moved_last_through_a_lookahead_answers_false_then_counts() {
        // The lookahead's end moves `last` on the way to a miss; which start
        // positions Java tried decides where it stops, so only `false` is promised.
        let m = matcher("(?=a)b", "ab", true);
        assert_eq!(find(&m), Some(Value::Int(0)));
        assert_eq!(find(&m), None);
        assert_eq!(group(&m), None);
    }

    #[test]
    fn a_method_the_host_counts_unknowns_the_matcher() {
        let m = matcher(HEX, "#111111 #222222", true);
        assert_eq!(find(&m), Some(Value::Int(1)));
        let r = MethodRef {
            owner: "java/util/regex/Matcher".into(),
            name: "region".into(),
            descriptor: "(II)Ljava/util/regex/Matcher;".into(),
        };
        note_host_call(&r, &m);
        assert_eq!(find(&m), None);
        let reading = MethodRef {
            owner: "java/util/regex/Matcher".into(),
            name: "regionEnd".into(),
            descriptor: "()I".into(),
        };
        let n = matcher(HEX, "#111111", true);
        assert_eq!(find(&n), Some(Value::Int(1)));
        note_host_call(&reading, &n);
        assert_eq!(group(&n), Some(Value::Ref(Some("#111111".into()))));
    }

    #[test]
    fn the_table_forgets_by_serial_when_full() {
        let mut table = Table {
            tracked: BTreeMap::new(),
            forgotten_below: 0,
        };
        let followed = Tracked::Cursor(Cursor::default());
        let moved = {
            let mut cursor = Cursor::default();
            let regex = Regex::compile("a", 0).unwrap();
            cursor.find(&regex, "aa").unwrap();
            Tracked::Cursor(cursor)
        };
        assert_ne!(followed, moved);
        table.store(1, moved.clone(), 2, 10);
        table.store(2, moved.clone(), 2, 10);
        assert_eq!(table.load(1), moved);
        // A third matcher empties the table: 1 and 2 are forgotten, 3 stands,
        // and a matcher made since is fresh.
        table.store(3, moved.clone(), 2, 10);
        assert_eq!(table.load(1), Tracked::Unknown);
        assert_eq!(table.load(2), Tracked::Unknown);
        assert_eq!(table.load(3), moved);
        assert_eq!(table.load(10), followed);
        // A matcher back at its start needs no entry.
        table.store(3, followed.clone(), 2, 10);
        assert!(table.tracked.is_empty());
    }
}
