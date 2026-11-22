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
//! Two edges are not followed. A `find()` whose resume position is past the
//! end of the input (an empty match at the end was just found) answers
//! `false`, and Java leaves `first` and the group bounds in a state `start()`
//! and `group()` read differently from a plain miss. And a `find()` that
//! fails after a sub-pattern (a lookaround, an atomic group) moved `last`
//! leaves a `last` that depends on which start positions Java tried.
//! [`Found::NoThenUnmodelled`] tells the caller, which then counts every later
//! call on that matcher. A region, `usePattern`, `lookingAt` and the like are
//! not modelled either; the caller does the same for them.

use super::engine::{Limits, Matcher};
use super::{Match, R, Regex, Thrown, text, units};

/// The answer of one `find()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Found {
    Yes,
    No,
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
    pub(crate) fn find(&mut self, regex: &Regex, input: &str) -> R<Found> {
        let chars = units(input)?;
        let mut from = self.last;
        if self.groups.as_ref().is_some_and(|groups| groups[0] == from) {
            from += 1;
        }
        if from > chars.len() {
            return Ok(Found::NoThenUnmodelled);
        }
        self.search(regex, &chars, from)
    }

    /// `Matcher.find(int)`: `reset()`, then a search from `start`. An index
    /// outside the input throws and leaves the matcher as it was.
    pub(crate) fn find_from(
        &mut self,
        regex: &Regex,
        input: &str,
        start: i32,
    ) -> R<Result<Found, Thrown>> {
        let chars = units(input)?;
        let Some(from) = usize::try_from(start)
            .ok()
            .filter(|from| *from <= chars.len())
        else {
            return Ok(Err(index_out_of_bounds("Illegal start index".into())));
        };
        *self = Self::default();
        self.search(regex, &chars, from).map(Ok)
    }

    /// `Matcher.matches()`.
    pub(crate) fn matches(&mut self, regex: &Regex, input: &str) -> R<bool> {
        let chars = units(input)?;
        let mut matcher = Matcher::new(&regex.program, &chars, Limits::JAVA);
        matcher.seed_last(self.last);
        let matched = matcher.match_at_start(true)?;
        // The one attempt Java makes is the one the engine made, so the `last`
        // a miss leaves (a lookahead's end it reached, say) is Java's too.
        self.last = matcher.last();
        self.groups = matched.then_some(matcher.groups);
        Ok(matched)
    }

    /// `Matcher.reset()`.
    pub(crate) fn reset(&mut self) {
        *self = Self::default();
    }

    /// The leftmost match at or after `from`.
    ///
    /// A search that fails clears `first` and leaves `last` as the
    /// sub-patterns it reached left it. Java does not try every position the
    /// engine does (it skips those too near the end for the pattern's minimum
    /// length), so a `last` that moved during a miss is not Java's to trust:
    /// the answer `false` stands and the cursor is [`Found::NoThenUnmodelled`].
    fn search(&mut self, regex: &Regex, chars: &[char], from: usize) -> R<Found> {
        let mut matcher = Matcher::new(&regex.program, chars, Limits::JAVA);
        matcher.seed_last(self.last);
        if matcher.search(from)? {
            self.last = matcher.groups[1];
            self.groups = Some(matcher.groups);
            return Ok(Found::Yes);
        }
        self.groups = None;
        Ok(if matcher.last() == self.last {
            Found::No
        } else {
            Found::NoThenUnmodelled
        })
    }

    /// Group `n`'s bounds, with Java's checks: no match, then an index out of
    /// range. `None` when the group did not take part.
    fn bounds(&self, regex: &Regex, n: i32) -> R<Result<Option<(usize, usize)>, Thrown>> {
        let Some(groups) = &self.groups else {
            return Ok(Err(illegal_state()));
        };
        let Some(n) = usize::try_from(n)
            .ok()
            .filter(|n| *n <= regex.group_count())
        else {
            return Ok(Err(index_out_of_bounds(format!("No group {n}"))));
        };
        let found = Match {
            groups: std::borrow::Cow::Borrowed(groups),
            tainted: &regex.program.tainted,
        };
        found.group(n).map(Ok)
    }

    /// `Matcher.group(int)` (and `group()` for `0`): `None` is Java's `null`.
    pub(crate) fn group(
        &self,
        regex: &Regex,
        input: &str,
        n: i32,
    ) -> R<Result<Option<String>, Thrown>> {
        Ok(match self.bounds(regex, n)? {
            Ok(Some((start, end))) => Ok(Some(text(&units(input)?[start..end]))),
            Ok(None) => Ok(None),
            Err(thrown) => Err(thrown),
        })
    }

    /// `Matcher.start(int)` (`start()` for `0`): `-1` for a group that did
    /// not take part.
    pub(crate) fn start(&self, regex: &Regex, n: i32) -> R<Result<i32, Thrown>> {
        Ok(self
            .bounds(regex, n)?
            .map(|bounds| bounds.map_or(-1, |(start, _)| start as i32)))
    }

    /// `Matcher.end(int)` (`end()` for `0`).
    pub(crate) fn end(&self, regex: &Regex, n: i32) -> R<Result<i32, Thrown>> {
        Ok(self
            .bounds(regex, n)?
            .map(|bounds| bounds.map_or(-1, |(_, end)| end as i32)))
    }
}
