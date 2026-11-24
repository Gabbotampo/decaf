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
//! emptied, and every matcher made before then is [`Tracked::Changed`]: it may
//! have been changed before the table forgot it.
//!
//! A matcher this module cannot follow answers nothing more but `matches()`:
//! its next calls are counted, never guessed. That is a matcher after a
//! `find()` the engine refuses (an unsupported pattern or input, a budget),
//! after the `find()` past an empty match at the end of the input
//! (`java_regex::Found::NoThenUnmodelled`), and one over a `StringBuilder`'s
//! text (Java's would see later appends). `matches()` reads neither the
//! position nor anything a lost position stands for, so it is still answered
//! from the pattern and input the handle holds.
//!
//! A matcher a call changed in a way not modelled (`lookingAt`, `region`,
//! `usePattern`, `reset(CharSequence)`, `replaceAll`, `appendReplacement`,
//! `results` and the rest; [`Tracked::Changed`]) answers nothing at all, not
//! even `matches()`: the handle's pattern, input and region are no longer the
//! matcher's. The mark is never taken back.
//!
//! A matcher whose last attempt failed over a pattern with a quantifier (or
//! after a lookaround moved `last`) is followed less far: its `find()` is
//! counted, because Java's `last` is not the engine's (see
//! `java_regex::Cursor`); what does not read `last` is still answered.
//!
//! The compiled pattern and the input read as `char`s are kept per matcher
//! ([`Subjects`]), so a `while (m.find())` loop costs what each search costs,
//! not the length of the input and the pattern's compile on every call.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, MutexGuard};

use crate::host_jdk_constant::{matcher_identity, matcher_parts, next_matcher_serial};
use crate::java_regex::{self, Compiled, Cursor, Found};
use crate::regex_calls::thrown;
use crate::{MethodRef, NativeError, StdHost, Value, expect_int};

/// Matchers whose position is kept; at the bound the table is emptied.
const MAX_TRACKED: usize = 16_384;
/// Matchers whose compiled pattern and input are kept, and the `char`s of
/// input they may hold in all; past either the cache is emptied (a later call
/// compiles again, which is slower, never different).
const MAX_SUBJECTS: usize = 256;
const MAX_SUBJECT_CHARS: usize = 4 << 20;

/// One matcher's state.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Tracked {
    Cursor(Cursor),
    /// Not followed: a call that needs its position is counted. The matcher
    /// still searches the handle's pattern and input over the whole input, so
    /// `matches()` is answered.
    Unknown,
    /// A call changed the matcher (its region, pattern or input, or something
    /// not modelled), or it may have: every call on it is counted, `matches()`
    /// too. Never replaced by another state.
    Changed,
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
            // It may have been changed before the table forgot it.
            None if serial < self.forgotten_below => Tracked::Changed,
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

/// The compiled pattern and the input of the matchers recently called, by
/// serial. A matcher's pattern and input never change, so an entry is right
/// for as long as it lives; one that is gone is made again from the handle.
#[derive(Debug, Default)]
struct Subjects {
    by_serial: BTreeMap<u64, Arc<Compiled>>,
    /// `char`s of input held in all.
    chars: usize,
}

static SUBJECTS: Mutex<Subjects> = Mutex::new(Subjects {
    by_serial: BTreeMap::new(),
    chars: 0,
});

impl Subjects {
    fn insert(&mut self, serial: u64, subject: Arc<Compiled>, max: usize, max_chars: usize) {
        if self.by_serial.len() >= max || self.chars + subject.len() > max_chars {
            self.by_serial.clear();
            self.chars = 0;
        }
        self.chars += subject.len();
        if let Some(old) = self.by_serial.insert(serial, subject) {
            self.chars -= old.len();
        }
    }
}

/// The subject of matcher `receiver`; `None` for a pattern or an input the
/// engine does not run.
fn subject(receiver: &Value, serial: u64) -> Option<Arc<Compiled>> {
    let cached = SUBJECTS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .by_serial
        .get(&serial)
        .cloned();
    if cached.is_some() {
        return cached;
    }
    let parts = matcher_parts(receiver)?;
    let subject = Arc::new(Compiled::new(&parts.pattern, &parts.input).ok()?);
    SUBJECTS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .insert(serial, subject.clone(), MAX_SUBJECTS, MAX_SUBJECT_CHARS);
    Some(subject)
}

fn boolean(value: bool) -> Option<Value> {
    Some(Value::Int(i32::from(value)))
}

/// A matcher handle's serial and whether its input is fixed.
#[derive(Clone, Copy)]
struct Id {
    serial: u64,
    fixed: bool,
}

/// A call on a `Matcher` handle; `Ok(None)` leaves it counted.
pub(crate) fn invoke(
    r: &MethodRef,
    receiver: Option<&Value>,
    args: &[Value],
) -> Result<Option<Value>, NativeError> {
    let Some(receiver) = receiver else {
        return Ok(None);
    };
    let Some((serial, fixed)) = matcher_identity(receiver) else {
        return Ok(None);
    };
    let id = Id { serial, fixed };
    let int_argument = || expect_int(r, StdHost::argument(r, args, 0)?);
    match (r.name.as_str(), r.descriptor.as_str()) {
        ("matches", "()Z") => matches(id, receiver),
        ("find", "()Z") => find(id, receiver, None),
        ("find", "(I)Z") => find(id, receiver, Some(int_argument()?)),
        ("reset", "()Ljava/util/regex/Matcher;") => reset(id, receiver),
        ("group", "()Ljava/lang/String;") => query(id, receiver, Query::Group(0)),
        ("group", "(I)Ljava/lang/String;") => query(id, receiver, Query::Group(int_argument()?)),
        ("start", "()I") => query(id, receiver, Query::Start(0)),
        ("start", "(I)I") => query(id, receiver, Query::Start(int_argument()?)),
        ("end", "()I") => query(id, receiver, Query::End(0)),
        ("end", "(I)I") => query(id, receiver, Query::End(int_argument()?)),
        ("groupCount", "()I") => query(id, receiver, Query::GroupCount),
        // Anything else may change the matcher (`lookingAt`, `region`,
        // `usePattern`, `reset(CharSequence)`, `replaceAll`...) in a way its
        // position does not describe.
        _ => {
            note_host_call(r, receiver);
            Ok(None)
        }
    }
}

/// A call on a matcher that no model here answered and that reaches the host
/// (which counts it). Unless it only reads the matcher (`toString`,
/// `group(String)`, `regionEnd`...) the matcher may have changed:
/// - `find`, `find(int)`, `matches` and `reset()` are this module's own calls,
///   which change the position and nothing else: a followed matcher is no
///   longer followed (an unfollowed one already is, and a changed one stays
///   changed);
/// - any other call (`region`, `usePattern`, `reset(CharSequence)`,
///   `lookingAt`, `replaceAll`...) may have changed what the matcher searches:
///   it is [`Tracked::Changed`], and `matches()` is counted too.
pub(crate) fn note_host_call(r: &MethodRef, receiver: &Value) {
    if r.owner != "java/util/regex/Matcher" || reads_only(r.name.as_str()) {
        return;
    }
    // Over a builder's text too: `matches()` is answered there from the
    // handle's pattern and input, which `region` or `usePattern` made wrong.
    let Some((serial, fixed)) = matcher_identity(receiver) else {
        return;
    };
    if moves_position_only(r) {
        if fixed && matches!(load(serial), Tracked::Cursor(_)) {
            store(serial, Tracked::Unknown);
        }
        return;
    }
    store(serial, Tracked::Changed);
}

/// The calls of this module that move a matcher's position and nothing else.
fn moves_position_only(r: &MethodRef) -> bool {
    matches!(
        (r.name.as_str(), r.descriptor.as_str()),
        ("find", "()Z")
            | ("find", "(I)Z")
            | ("matches", "()Z")
            | ("reset", "()Ljava/util/regex/Matcher;")
    )
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
fn followed(id: Id) -> Option<Cursor> {
    if !id.fixed {
        return None;
    }
    match load(id.serial) {
        Tracked::Cursor(cursor) => Some(cursor),
        Tracked::Unknown | Tracked::Changed => None,
    }
}

/// `find()` (`from` is `None`) and `find(int)`.
fn find(id: Id, receiver: &Value, from: Option<i32>) -> Result<Option<Value>, NativeError> {
    let Some(mut cursor) = followed(id) else {
        return Ok(None);
    };
    let Some(subject) = subject(receiver, id.serial) else {
        store(id.serial, Tracked::Unknown);
        return Ok(None);
    };
    let outcome = match from {
        None => cursor.find(&subject).map(Ok),
        Some(from) => cursor.find_from(&subject, from),
    };
    match outcome {
        Ok(Ok(Found::Counted)) => {
            // Java searches from somewhere this cursor does not know, and it
            // will have moved: the matcher is followed no more.
            store(id.serial, Tracked::Unknown);
            Ok(None)
        }
        Ok(Ok(found)) => {
            store(
                id.serial,
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
            store(id.serial, Tracked::Unknown);
            Ok(None)
        }
    }
}

/// `matches()`: its answer needs no position, but it leaves one. A matcher
/// that lost its position, or whose input is a builder's, is answered from the
/// handle's pattern and input; one a call changed is not answered at all.
fn matches(id: Id, receiver: &Value) -> Result<Option<Value>, NativeError> {
    if load(id.serial) == Tracked::Changed {
        return Ok(None);
    }
    let Some(mut cursor) = followed(id) else {
        return Ok(matcher_parts(receiver).and_then(|parts| {
            java_regex::matches(&parts.pattern, &parts.input)
                .ok()
                .and_then(boolean)
        }));
    };
    let Some(subject) = subject(receiver, id.serial) else {
        store(id.serial, Tracked::Unknown);
        return Ok(None);
    };
    match cursor.matches(&subject) {
        Ok(matched) => {
            store(id.serial, Tracked::Cursor(cursor));
            Ok(boolean(matched))
        }
        Err(_) => {
            store(id.serial, Tracked::Unknown);
            Ok(None)
        }
    }
}

fn reset(id: Id, receiver: &Value) -> Result<Option<Value>, NativeError> {
    let Some(mut cursor) = followed(id) else {
        return Ok(None);
    };
    cursor.reset();
    store(id.serial, Tracked::Cursor(cursor));
    Ok(Some(receiver.clone()))
}

#[derive(Clone, Copy)]
enum Query {
    Group(i32),
    Start(i32),
    End(i32),
    GroupCount,
}

fn query(id: Id, receiver: &Value, query: Query) -> Result<Option<Value>, NativeError> {
    let Some(cursor) = followed(id) else {
        return Ok(None);
    };
    let Some(subject) = subject(receiver, id.serial) else {
        return Ok(None);
    };
    let answer = match query {
        Query::GroupCount => return Ok(Some(Value::Int(subject.group_count() as i32))),
        Query::Group(n) => cursor.group(&subject, n).map(|group| group.map(Value::Ref)),
        Query::Start(n) => cursor.start(&subject, n).map(|start| start.map(Value::Int)),
        Query::End(n) => cursor.end(&subject, n).map(|end| end.map(Value::Int)),
    };
    match answer {
        Ok(Ok(value)) => Ok(Some(value)),
        Ok(Err(exception)) => Err(thrown(exception)),
        // A group whose value is not Java's to promise.
        Err(_) => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host_jdk_constant::matcher;

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
        assert_eq!(call(&m, "matches", "()Z", &[]), None);
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
    fn matches_is_counted_after_a_call_that_changed_the_matcher() {
        // Java answers `matches()` over the region or pattern the call left,
        // not over the handle's: region(0,1) on "ab" makes `a` match, and
        // usePattern(b) makes `a` over "a" not.
        let region = MethodRef {
            owner: "java/util/regex/Matcher".into(),
            name: "region".into(),
            descriptor: "(II)Ljava/util/regex/Matcher;".into(),
        };
        let use_pattern = MethodRef {
            owner: "java/util/regex/Matcher".into(),
            name: "usePattern".into(),
            descriptor: "(Ljava/util/regex/Pattern;)Ljava/util/regex/Matcher;".into(),
        };
        let reset_input = MethodRef {
            owner: "java/util/regex/Matcher".into(),
            name: "reset".into(),
            descriptor: "(Ljava/lang/CharSequence;)Ljava/util/regex/Matcher;".into(),
        };
        for (changing, fixed) in [
            (&region, true),
            (&use_pattern, true),
            (&reset_input, true),
            (&region, false),
        ] {
            let m = matcher("a", "ab", fixed);
            // What the host's counting does, as `invoke` does it.
            assert_eq!(invoke(changing, Some(&m), &[]).ok(), Some(None));
            assert_eq!(matches(&m), None, "{} {fixed}", changing.name);
            assert_eq!(find(&m), None);
            assert_eq!(group(&m), None);
            assert_eq!(matches(&m), None, "never taken back");
        }
        // Nothing changed it: the builder's matcher still answers `matches()`.
        let builder = matcher("a", "a", false);
        assert_eq!(matches(&builder), Some(Value::Int(1)));
    }

    #[test]
    fn a_changed_matcher_is_never_followed_again_by_a_position_loss() {
        let m = matcher("a", "ab", true);
        store(matcher_identity(&m).unwrap().0, Tracked::Changed);
        assert_eq!(matches(&m), None);
        assert_eq!(call(&m, "reset", "()Ljava/util/regex/Matcher;", &[]), None);
        assert_eq!(find_from(&m, 0), None);
    }

    #[test]
    fn a_find_the_host_counts_loses_the_position_not_the_text() {
        // Java's `find()` here resumes from a `last` the engine does not know:
        // it is counted, the host is told, and `matches()` is still answered
        // from the handle's text (Java: false, the whole input is not `a?b`).
        let m = matcher("a?b", "abab", true);
        assert_eq!(matches(&m), Some(Value::Int(0)));
        assert_eq!(find(&m), None);
        let r = MethodRef {
            owner: "java/util/regex/Matcher".into(),
            name: "find".into(),
            descriptor: "()Z".into(),
        };
        note_host_call(&r, &m);
        assert_eq!(matches(&m), Some(Value::Int(0)));
        assert_eq!(find(&m), None);
        assert_eq!(group(&m), None);
    }

    #[test]
    fn matches_is_still_answered_after_the_position_is_lost() {
        // The `false` past an empty match at the end loses the position; the
        // matcher still searches the handle's text, so `matches()` is Java's.
        let m = matcher("x*", "", true);
        assert_eq!(find(&m), Some(Value::Int(1)));
        assert_eq!(find(&m), Some(Value::Int(0)));
        assert_eq!(find(&m), None);
        assert_eq!(matches(&m), Some(Value::Int(1)));
    }

    fn matches(receiver: &Value) -> Option<Value> {
        call(receiver, "matches", "()Z", &[])
    }

    fn find_from(receiver: &Value, from: i32) -> Option<Value> {
        call(receiver, "find", "(I)Z", &[Value::Int(from)])
    }

    fn start(receiver: &Value) -> Option<Value> {
        call(receiver, "start", "()I", &[])
    }

    #[test]
    fn a_quantified_atom_that_failed_leaves_last_unknown_so_find_is_counted() {
        // Java's `a?` sets `last` to 1 on the way to the failed `matches()`
        // and the next `find()` resumes there; the engine does not follow it.
        for (pattern, input) in [
            ("a?b", "abab"),
            ("a{2}", "aab"),
            (".?", "aa"),
            ("a.*?b", "aba\u{85}"),
            ("a*+b", "aabx"),
            ("(ab)*c", "abab"),
            (HEX, "#ff0000 #00ff00"),
        ] {
            let m = matcher(pattern, input, true);
            assert!(matches(&m).is_some(), "{pattern}");
            assert_eq!(find(&m), None, "{pattern}");
            assert_eq!(group(&m), None, "{pattern}");
        }
    }

    #[test]
    fn what_does_not_read_last_is_still_answered_after_such_a_miss() {
        // The failed `matches()` left no match: `group()` throws, `matches()`
        // tries again from the start, `find(int)` and `reset()` start over.
        let m = matcher("a?b", "abab", true);
        assert_eq!(matches(&m), Some(Value::Int(0)));
        let r = MethodRef {
            owner: "java/util/regex/Matcher".into(),
            name: "group".into(),
            descriptor: "()Ljava/lang/String;".into(),
        };
        assert!(invoke(&r, Some(&m), &[]).is_err());
        assert_eq!(matches(&m), Some(Value::Int(0)));
        assert_eq!(find_from(&m, 1), Some(Value::Int(1)));
        assert_eq!(start(&m), Some(Value::Int(1)));
        assert_eq!(find(&m), Some(Value::Int(1)));
        assert_eq!(start(&m), Some(Value::Int(2)));
        // A failed `find(int)` loses `last` again; `reset()` finds it.
        assert_eq!(find_from(&m, 4), Some(Value::Int(0)));
        let n = matcher("a?b", "abab", true);
        assert_eq!(matches(&n), Some(Value::Int(0)));
        call(&n, "reset", "()Ljava/util/regex/Matcher;", &[]).unwrap();
        assert_eq!(find(&n), Some(Value::Int(1)));
        assert_eq!(start(&n), Some(Value::Int(0)));
    }

    #[test]
    fn a_successful_matches_after_a_lost_last_follows_again() {
        let m = matcher("a?b", "ab", true);
        assert_eq!(find_from(&m, 1), Some(Value::Int(1)));
        assert_eq!(find_from(&m, 2), Some(Value::Int(0)));
        assert_eq!(matches(&m), Some(Value::Int(1)));
        assert_eq!(find(&m), Some(Value::Int(0)));
    }

    #[test]
    fn a_pattern_without_quantifiers_still_follows_a_failed_attempt() {
        // No repeated atom sets `last`: Java's resume position is known.
        let m = matcher("a", "ab", true);
        assert_eq!(matches(&m), Some(Value::Int(0)));
        assert_eq!(find(&m), Some(Value::Int(1)));
        assert_eq!(start(&m), Some(Value::Int(0)));
    }

    #[test]
    fn a_long_search_compiles_and_reads_the_input_once() {
        let input = "ab".repeat(20_000);
        let m = matcher("ab", &input, true);
        let mut found = 0;
        while find(&m) == Some(Value::Int(1)) {
            found += 1;
        }
        assert_eq!(found, 20_000);
    }

    #[test]
    fn the_subject_cache_forgets_when_full() {
        let mut subjects = Subjects::default();
        let one = Arc::new(Compiled::new("a", "aaa").unwrap());
        subjects.insert(1, one.clone(), 2, 100);
        subjects.insert(2, one.clone(), 2, 100);
        assert_eq!(subjects.by_serial.len(), 2);
        assert_eq!(subjects.chars, 6);
        subjects.insert(3, one.clone(), 2, 100);
        assert_eq!(subjects.by_serial.len(), 1);
        assert_eq!(subjects.chars, 3);
        // By weight: 3 + 3 fits in 8, a third 3 does not.
        subjects.insert(4, one.clone(), 9, 8);
        assert_eq!(subjects.by_serial.len(), 2);
        assert_eq!(subjects.chars, 6);
        subjects.insert(5, one, 9, 8);
        assert_eq!(subjects.by_serial.len(), 1);
        assert_eq!(subjects.chars, 3);
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
            let subject = Compiled::new("a", "aa").unwrap();
            cursor.find(&subject).unwrap();
            Tracked::Cursor(cursor)
        };
        assert_ne!(followed, moved);
        table.store(1, moved.clone(), 2, 10);
        table.store(2, moved.clone(), 2, 10);
        assert_eq!(table.load(1), moved);
        // A third matcher empties the table: 1 and 2 are forgotten, 3 stands,
        // and a matcher made since is fresh.
        table.store(3, moved.clone(), 2, 10);
        assert_eq!(table.load(1), Tracked::Changed);
        assert_eq!(table.load(2), Tracked::Changed);
        assert_eq!(table.load(3), moved);
        assert_eq!(table.load(10), followed);
        // A matcher back at its start needs no entry.
        table.store(3, followed.clone(), 2, 10);
        assert!(table.tracked.is_empty());
    }
}
