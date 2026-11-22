//! A `Matcher`'s search position: what `find()`, `find(int)`, `matches()` and
//! `reset()` leave for the next `find()` and for `group`, `start` and `end`.
//!
//! Java's `Matcher` keeps `first` (where the last match began, `-1` when the
//! last search failed or none ran), `last` (where it ended) and the group
//! bounds. `find()` resumes at `last`, one `char` later when the last match
//! was empty (`first == last`); a search that fails clears `first` but leaves
//! `last`, so after an empty match is followed by a miss, the next `find()`
//! searches the same position again (`(?=a)` over `ab` answers true, false,
//! true, false ...). A failed `matches()` does the same. These are the
//! behaviours `tests/fixtures/matcher_find` pins against Java 25.
//!
//! Three edges are not followed.
//!
//! A `find()` whose resume position is past the end of the input (an empty
//! match at the end was just found) answers `false`, and Java leaves `first`
//! and the group bounds in a state `start()` and `group()` read differently
//! from a plain miss: [`Found::NoThenUnmodelled`] tells the caller, which then
//! counts every later call on that matcher.
//!
//! A failed attempt may leave `last` somewhere this module cannot name. Java's
//! quantified atoms (`a?`, `a*`, `a{2}`, `(ab)+`, lazy and possessive forms)
//! end in the shared accept node, which sets `last` on every iteration, and
//! the engine does not write it there; a lookaround that fails after moving
//! `last` leaves one that depends on which start positions Java tried. After
//! such a miss the cursor is `last_lost`: the next `find()` is counted
//! ([`Found::Counted`]). What does not read `last` is still answered, and what
//! sets it again ends the doubt: `group`/`start`/`end` (the miss left no
//! match), `matches()` (it attempts from the start and, when it succeeds, sets
//! `last` itself), `find(int)` and `reset()` (both reset `last` to `0` first).
//!
//! A region, `usePattern`, `lookingAt` and the like are not modelled either;
//! the caller counts every later call after one.

use super::engine::{Limits, Matcher};
use super::{Match, R, Regex, Thrown, text, units};

/// A pattern and the input it searches, ready to run: compiled once and the
/// input read as Java `char`s once, however many calls a matcher makes.
#[derive(Debug)]
pub(crate) struct Compiled {
    regex: Regex,
    chars: Vec<char>,
}

impl Compiled {
    /// Compile `pattern` (flags `0`) and read `input` as `char`s; an error
    /// where either is outside the engine's subset.
    pub(crate) fn new(pattern: &str, input: &str) -> R<Self> {
        Ok(Self {
            regex: Regex::compile(pattern, 0)?,
            chars: units(input)?,
        })
    }

    /// The input's length in `char`s: what the value weighs in a cache.
    pub(crate) fn len(&self) -> usize {
        self.chars.len()
    }

    /// `Matcher.groupCount()`.
    pub(crate) fn group_count(&self) -> usize {
        self.regex.group_count()
    }
}

/// The answer of one `find()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Found {
    Yes,
    No,
    /// Not answered: the cursor's `last` is lost (see the module docs), so the
    /// search's start is not known. The cursor no longer describes Java's
    /// matcher once the call has run.
    Counted,
    /// `false`, from a resume position past the end of the input; the cursor
    /// no longer describes Java's matcher.
    NoThenUnmodelled,
}

/// A matcher's `first`/`last`/groups: a fresh matcher is [`Cursor::default`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Cursor {
    /// The last match's group bounds (`first` is `groups[0]`); `None` when
    /// `first` is `-1`.
    groups: Option<Vec<usize>>,
    /// Where the last successful match ended (`0` before one).
    last: usize,
    /// A failed attempt left Java's `last` somewhere the engine does not
    /// follow: `last` is not to be trusted until `matches()`, `find(int)` or
    /// `reset()` sets it again.
    last_lost: bool,
}

fn illegal_state() -> Thrown {
    Thrown {
        class: "java/lang/IllegalStateException",
        message: "No match found".into(),
    }
}

fn index_out_of_bounds(message: String) -> Thrown {
    Thrown {
        class: "java/lang/IndexOutOfBoundsException",
        message,
    }
}

impl Cursor {
    /// `Matcher.find()`.
    pub(crate) fn find(&mut self, subject: &Compiled) -> R<Found> {
        if self.last_lost {
            return Ok(Found::Counted);
        }
        let mut from = self.last;
        if self.groups.as_ref().is_some_and(|groups| groups[0] == from) {
            from += 1;
        }
        if from > subject.chars.len() {
            return Ok(Found::NoThenUnmodelled);
        }
        self.search(subject, from)
    }

    /// `Matcher.find(int)`: `reset()`, then a search from `start`. An index
    /// outside the input throws and leaves the matcher as it was.
    pub(crate) fn find_from(&mut self, subject: &Compiled, start: i32) -> R<Result<Found, Thrown>> {
        let Some(from) = usize::try_from(start)
            .ok()
            .filter(|from| *from <= subject.chars.len())
        else {
            return Ok(Err(index_out_of_bounds("Illegal start index".into())));
        };
        *self = Self::default();
        self.search(subject, from).map(Ok)
    }

    /// `Matcher.matches()`.
    pub(crate) fn matches(&mut self, subject: &Compiled) -> R<bool> {
        let mut matcher = Matcher::new(&subject.regex.program, &subject.chars, Limits::JAVA);
        matcher.seed_last(self.last);
        let matched = matcher.match_at_start(true)?;
        if matched {
            self.last = matcher.last();
            self.last_lost = false;
            self.groups = Some(matcher.groups);
        } else {
            // Java makes the one attempt the engine made, so a `last` the
            // engine moved (a lookahead's end it reached) is Java's too, but
            // a quantified atom it passed moved Java's and not the engine's.
            self.last = matcher.last();
            self.last_lost |= subject.regex.repeats();
            self.groups = None;
        }
        Ok(matched)
    }

    /// `Matcher.reset()`.
    pub(crate) fn reset(&mut self) {
        *self = Self::default();
    }

    /// The leftmost match at or after `from`.
    ///
    /// A search that fails clears `first` and leaves `last` as the atoms and
    /// sub-patterns it reached left it. Java does not try every position the
    /// engine does (it skips those too near the end for the pattern's minimum
    /// length), and sets `last` in quantified atoms the engine does not write,
    /// so a miss over a pattern with either leaves a `last` that is not Java's
    /// to trust: the answer `false` stands and the cursor is `last_lost`.
    fn search(&mut self, subject: &Compiled, from: usize) -> R<Found> {
        let mut matcher = Matcher::new(&subject.regex.program, &subject.chars, Limits::JAVA);
        matcher.seed_last(self.last);
        if matcher.search(from)? {
            self.last = matcher.groups[1];
            self.last_lost = false;
            self.groups = Some(matcher.groups);
            return Ok(Found::Yes);
        }
        self.groups = None;
        if matcher.last() != self.last || subject.regex.repeats() {
            self.last = matcher.last();
            self.last_lost = true;
        }
        Ok(Found::No)
    }

    /// Group `n`'s bounds, with Java's checks: no match, then an index out of
    /// range. `None` when the group did not take part.
    fn bounds(&self, subject: &Compiled, n: i32) -> R<Result<Option<(usize, usize)>, Thrown>> {
        let Some(groups) = &self.groups else {
            return Ok(Err(illegal_state()));
        };
        let Some(n) = usize::try_from(n)
            .ok()
            .filter(|n| *n <= subject.regex.group_count())
        else {
            return Ok(Err(index_out_of_bounds(format!("No group {n}"))));
        };
        let found = Match {
            groups: std::borrow::Cow::Borrowed(groups),
            tainted: &subject.regex.program.tainted,
        };
        found.group(n).map(Ok)
    }

    /// `Matcher.group(int)` (and `group()` for `0`): `None` is Java's `null`.
    pub(crate) fn group(&self, subject: &Compiled, n: i32) -> R<Result<Option<String>, Thrown>> {
        Ok(match self.bounds(subject, n)? {
            Ok(Some((start, end))) => Ok(Some(text(&subject.chars[start..end]))),
            Ok(None) => Ok(None),
            Err(thrown) => Err(thrown),
        })
    }

    /// `Matcher.start(int)` (`start()` for `0`): `-1` for a group that did
    /// not take part.
    pub(crate) fn start(&self, subject: &Compiled, n: i32) -> R<Result<i32, Thrown>> {
        Ok(self
            .bounds(subject, n)?
            .map(|bounds| bounds.map_or(-1, |(start, _)| start as i32)))
    }

    /// `Matcher.end(int)` (`end()` for `0`).
    pub(crate) fn end(&self, subject: &Compiled, n: i32) -> R<Result<i32, Thrown>> {
        Ok(self
            .bounds(subject, n)?
            .map(|bounds| bounds.map_or(-1, |(_, end)| end as i32)))
    }
}
