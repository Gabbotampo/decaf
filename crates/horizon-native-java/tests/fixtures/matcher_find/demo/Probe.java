package demo;

import java.util.function.BooleanSupplier;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

/**
 * Oracle for `java.util.regex.Matcher#find()` and the queries that read what it
 * found (`group`, `start`, `end`, `groupCount`), `find(int)`, `reset()`, and
 * what `matches()` leaves behind for them. fabric-carpet's call sites compile
 * `#([0-9a-fA-F]{6})` (Messenger's `colorExtract`) and `\[[^\[]*]$`
 * (NBTSerializableValue) with `Pattern.compile(String)`, then call `find()`,
 * `group()` and `group(1)`.
 *
 * One line per row: `name<TAB>result`; a row that throws prints the exception
 * class and message.
 */
public final class Probe {
    interface Body {
        String go() throws Exception;
    }

    static final String HEX = "#([0-9a-fA-F]{6})";

    static String attempt(Body body) {
        try {
            return body.go();
        } catch (Throwable e) {
            return e.getClass().getName() + ": " + e.getMessage();
        }
    }

    static StringBuilder out = new StringBuilder();

    static void row(String name, Body body) {
        out.append(name).append('\t').append(attempt(body)).append('\n');
    }

    static Matcher m(String pattern, String input) {
        return Pattern.compile(pattern).matcher(input);
    }

    /** `calls` successive find() calls: each answer, and the match's bounds when true. */
    static String trace(String pattern, String input, int calls) {
        Matcher m = m(pattern, input);
        StringBuilder b = new StringBuilder();
        for (int i = 0; i < calls; i++) {
            if (i > 0) {
                b.append(' ');
            }
            if (m.find()) {
                b.append('[').append(m.start()).append(',').append(m.end()).append(')');
            } else {
                b.append("false");
            }
        }
        return b.toString();
    }

    /** A call's text as printable ASCII, for a row's name and result. */
    static String esc(String s) {
        if (s == null) {
            return "null";
        }
        StringBuilder b = new StringBuilder();
        for (char c : s.toCharArray()) {
            if (c < 0x20 || c > 0x7e) {
                String hex = Integer.toHexString(c);
                b.append("\\u").append("0000".substring(hex.length())).append(hex);
            } else {
                b.append(c);
            }
        }
        return b.toString();
    }

    /**
     * One call of a call sequence: `f` find(), `m` matches(), `g` group(), `h`
     * group(1), `s` start(), `e` end(), `r` reset(), `c` groupCount(), a digit
     * `d` find(d).
     */
    static String step(Matcher m, char op) {
        try {
            switch (op) {
                case 'f': return "f=" + m.find();
                case 'm': return "m=" + m.matches();
                case 'g': return "g=" + esc(m.group());
                case 'h': return "h=" + esc(m.group(1));
                case 's': return "s=" + m.start();
                case 'e': return "e=" + m.end();
                case 'r': m.reset(); return "r";
                case 'c': return "c=" + m.groupCount();
                default: return op + "=" + m.find(op - '0');
            }
        } catch (Throwable e) {
            return op + "!" + e.getClass().getName() + ":" + e.getMessage();
        }
    }

    /**
     * `ops`, one call each on a fresh matcher; one result per call. Written
     * flat (no row lambda): the runtime admits only so many jar frames on a
     * debug test thread, and the oracle must not depend on the build.
     */
    static void sequence(String pattern, String input, String ops) {
        out.append("seq[").append(esc(pattern)).append('|').append(esc(input)).append('|')
            .append(ops).append("]\t");
        try {
            Matcher m = Pattern.compile(pattern).matcher(input);
            for (int i = 0; i < ops.length(); i++) {
                if (i > 0) {
                    out.append(' ');
                }
                out.append(step(m, ops.charAt(i)));
            }
        } catch (Throwable e) {
            out.append(e.getClass().getName()).append(": ").append(e.getMessage());
        }
        out.append('\n');
    }

    /**
     * Matchers whose first `matches()` fails over a pattern with a quantified
     * atom (`a?`, `a{2}`, `.?`, lazy, possessive, a repeated group): Java's
     * repeated atoms set `Matcher.last` on every iteration, so the next
     * `find()` resumes where the failed attempt left it.
     */
    static final String[][] FAILED_MATCHES = {
        {"a?b", "abab"}, {"a{2}", "aab"}, {".?", "aa"}, {"b?(?=a)", "ab"}, {"a?$", "aaa"},
        {"#([0-9a-fA-F]{6})", "#ff0000 #00ff00"}, {"a.*?b", "aba\u0085"}, {"a*+b", "aabx"},
        {"(?:ab)*c", "abab"}, {"(a)*", "aab"}, {"[a-c]{2,3}", "abcd"}, {"\\d{3}", "12 345"},
        {"(a{2})+b", "aaab"}, {"a?b?c?", "xyz"}, {"(?i)a?B", "aab"}, {"a??b", "abab"},
        {"a{2,}+", "aaab"}, {"(ab)?c", "abab"}, {"x*", "ab"},
    };

    /** Matchers whose first `find()` fails over a pattern with a quantified atom. */
    static final String[][] FAILED_FIND = {
        {"a{2}", "ab"}, {"a?b", "xx"}, {"#([0-9a-fA-F]{6})", "#ff0 zz"}, {"[a-c]{2,3}", "a x"},
        {"\\d{3}", "12 34"}, {"(a{2})+b", "ab"}, {"a+b", "aac"}, {"(?:ab)+c", "ababd"},
        {"\\[[^\\[]*]$", "foo[bar"},
    };

    /**
     * Matchers whose first `find()` fails after a lookaround or an atomic group
     * ran, which leaves `Matcher.last` at the end of the sub-pattern it
     * reached (no quantifier is involved).
     */
    static final String[][] FAILED_LOOK = {
        {"(?=a)b", "aab"}, {"(?=a)(?=a)b", "aab"}, {"(?>a)b", "aac"}, {"(?!b)c", "aab"},
    };

    /**
     * Matchers whose first `find()` fails after a lookbehind ran: it does not
     * set `last`, so the `find()` after it is answered (it is `false`: a search
     * that failed from the start fails from anywhere after it).
     */
    static final String[][] FAILED_BEHIND = {{"(?<=a)c", "aab"}, {"(?<!a)b", "aab"}};

    /** What a failed attempt leaves that nothing reads `last` for. */
    static final String[] AFTER_FAILED_MATCHES = {"mg", "mmh", "m1g", "mrfg", "m2e", "mmmc", "ms"};
    static final String[] AFTER_FAILED_FIND = {"fg", "fmg", "f0g", "frfg", "fs", "f9", "fr"};

    static String rows() {
        out = new StringBuilder();
        row("find_match", () -> "find=" + m(HEX, "say #ff0000 please").find());
        row("find_nomatch", () -> "find=" + m(HEX, "hello world").find());
        row("find_group", () -> {
            Matcher m = m(HEX, "#ff0000");
            boolean found = m.find();
            return "find=" + found + " group=" + m.group() + " group1=" + m.group(1)
                    + " group0=" + m.group(0);
        });
        row("find_start_end", () -> {
            Matcher m = m(HEX, "a #ff0000 b");
            boolean found = m.find();
            return "find=" + found + " start=" + m.start() + " end=" + m.end()
                    + " start1=" + m.start(1) + " end1=" + m.end(1)
                    + " start0=" + m.start(0) + " end0=" + m.end(0);
        });
        row("find_groupCount", () -> {
            Matcher m = m(HEX, "#ff0000");
            boolean found = m.find();
            return "find=" + found + " count=" + m.groupCount();
        });
        row("find_repeated", () -> {
            Matcher m = m(HEX, "#ff0000 #00ff00");
            boolean first = m.find();
            String firstGroup = m.group(1);
            boolean second = m.find();
            return "find1=" + first + " group1=" + firstGroup + " find2=" + second
                    + " last_group1=" + m.group(1) + " start=" + m.start() + " end=" + m.end();
        });
        row("find_exhausted", () -> {
            Matcher m = m(HEX, "#ff0000");
            boolean first = m.find();
            boolean second = m.find();
            return "find1=" + first + " find2=" + second;
        });
        row("reset_then_find", () -> {
            Matcher m = m(HEX, "#ff0000");
            boolean first = m.find();
            m.reset();
            return "find1=" + first + " after_reset=" + m.find() + " group=" + m.group();
        });
        row("reset_returns_receiver", () -> {
            Matcher m = m(HEX, "#ff0000");
            return "same=" + (m.reset() == m);
        });
        row("matcher_empty_input", () -> "find=" + m(HEX, "").find());
        row("matches_whole", () -> "matches=" + m(HEX, "#ff0000").matches());
        row("matches_partial", () -> "matches=" + m(HEX, "x #ff0000").matches());
        row("bmp_input", () -> {
            Matcher m = m(HEX, "名 é #abcdef");
            boolean found = m.find();
            return "find=" + found + " start=" + m.start() + " end=" + m.end()
                    + " group1=" + m.group(1);
        });

        // Before any find, or after one that failed.
        row("group_before_find", () -> "group=" + m(HEX, "#ff0000").group());
        row("group0_before_find", () -> "group=" + m(HEX, "#ff0000").group(0));
        row("group1_before_find", () -> "group=" + m(HEX, "#ff0000").group(1));
        row("start_before_find", () -> "start=" + m(HEX, "#ff0000").start());
        row("end_before_find", () -> "end=" + m(HEX, "#ff0000").end());
        row("start1_before_find", () -> "start=" + m(HEX, "#ff0000").start(1));
        row("end1_before_find", () -> "end=" + m(HEX, "#ff0000").end(1));
        row("groupCount_before_find", () -> "count=" + m(HEX, "#ff0000").groupCount());
        row("group_after_failed_find", () -> {
            Matcher m = m(HEX, "nothing");
            boolean found = m.find();
            return "find=" + found + " group=" + m.group();
        });
        row("start_after_failed_find", () -> {
            Matcher m = m(HEX, "nothing");
            boolean found = m.find();
            return "find=" + found + " start=" + m.start();
        });
        row("end_after_failed_find", () -> {
            Matcher m = m(HEX, "nothing");
            boolean found = m.find();
            return "find=" + found + " end=" + m.end();
        });
        row("group_after_exhausted", () -> {
            Matcher m = m(HEX, "#ff0000");
            m.find();
            boolean second = m.find();
            return "find2=" + second + " group=" + m.group();
        });
        row("group_after_reset", () -> {
            Matcher m = m(HEX, "#ff0000");
            m.find();
            m.reset();
            return "group=" + m.group();
        });

        // Group indexes out of range, and groups that did not take part.
        row("group_too_big", () -> {
            Matcher m = m(HEX, "#ff0000");
            m.find();
            return "group=" + m.group(2);
        });
        row("group_negative", () -> {
            Matcher m = m(HEX, "#ff0000");
            m.find();
            return "group=" + m.group(-1);
        });
        row("start_too_big", () -> {
            Matcher m = m(HEX, "#ff0000");
            m.find();
            return "start=" + m.start(2);
        });
        row("end_negative", () -> {
            Matcher m = m(HEX, "#ff0000");
            m.find();
            return "end=" + m.end(-1);
        });
        row("too_big_before_find", () -> "group=" + m(HEX, "#ff0000").group(2));
        row("unset_group", () -> {
            Matcher m = m("(a)|(b)", "b");
            boolean found = m.find();
            return "find=" + found + " group0=" + m.group(0) + " group1=" + m.group(1)
                    + " start1=" + m.start(1) + " end1=" + m.end(1)
                    + " group2=" + m.group(2) + " start2=" + m.start(2) + " end2=" + m.end(2)
                    + " count=" + m.groupCount();
        });
        row("empty_group", () -> {
            Matcher m = m("a()b", "xab");
            boolean found = m.find();
            return "find=" + found + " group1=[" + m.group(1) + "] start1=" + m.start(1)
                    + " end1=" + m.end(1);
        });
        row("group_count_none", () -> "count=" + m("abc", "abc").groupCount());
        row("group_count_many", () -> "count=" + m("(a)(?:b)(c(d))(?<n>e)", "abcde").groupCount());
        row("dynamic_groups", () -> {
            String pattern = "(\\w+)" + "@" + "(\\w+)";
            Matcher m = m(pattern, "mail bob@example now");
            boolean found = m.find();
            return "find=" + found + " count=" + m.groupCount() + " group=" + m.group()
                    + " user=" + m.group(1) + " host=" + m.group(2) + " start2=" + m.start(2);
        });

        // Where the next search starts, empty matches included.
        row("trace_ab", () -> trace("ab", "ababab", 4));
        row("trace_overlap", () -> trace("aa", "aaaaa", 3));
        row("trace_empty_star", () -> trace("x*", "ab", 4));
        row("trace_empty_a", () -> trace("a*", "baac", 5));
        row("trace_lookahead_loop", () -> trace("(?=a)", "ab", 6));
        row("trace_lookahead_two", () -> trace("(?=a)", "aab", 5));
        row("trace_anchor_start", () -> trace("^a", "aa", 3));
        row("trace_anchor_end", () -> trace("a$", "aa", 3));
        row("trace_word_boundary", () -> trace("\\b", "ab cd", 5));
        row("trace_lookbehind", () -> trace("(?<=a)b", "ab ab", 4));
        row("trace_nbt_path", () -> trace("\\[[^\\[]*]$", "foo[bar", 1));
        row("trace_nbt_closed", () -> trace("\\[[^\\[]*]$", "foo[bar]", 2));
        row("trace_lazy", () -> trace("a.*?b", "axbxab", 3));
        row("trace_alternation", () -> trace("a|ab|b", "abb", 4));
        row("nbt_group", () -> {
            Matcher m = m("\\[[^\\[]*]$", "list[1].name[2]");
            boolean found = m.find();
            return "find=" + found + " group=" + m.group();
        });
        row("loop_count", () -> {
            Matcher m = m("ab", "ababab");
            int n = 0;
            while (m.find()) {
                n++;
            }
            return "count=" + n;
        });
        row("loop_groups", () -> {
            Matcher m = m("(\\d+)-(\\d+)", "1-2, 33-44; x-5; 6-7");
            StringBuilder b = new StringBuilder();
            while (m.find()) {
                b.append(m.group(1)).append(':').append(m.group(2)).append('@')
                        .append(m.start()).append(' ');
            }
            return b.toString();
        });

        // find(int) resets the matcher and searches from an index.
        row("find_from_zero", () -> {
            Matcher m = m(HEX, "#ff0000 #00ff00");
            boolean a = m.find(0);
            return "find=" + a + " start=" + m.start() + " group1=" + m.group(1);
        });
        row("find_from_middle", () -> {
            Matcher m = m(HEX, "#ff0000 #00ff00");
            boolean a = m.find(1);
            return "find=" + a + " start=" + m.start() + " group1=" + m.group(1);
        });
        row("find_from_end", () -> "find=" + m(HEX, "#ff0000").find(7));
        row("find_from_end_empty", () -> {
            Matcher m = m("x*", "ab");
            boolean a = m.find(2);
            return "find=" + a + " start=" + m.start() + " end=" + m.end();
        });
        row("find_from_past_end", () -> "find=" + m(HEX, "#ff0000").find(8));
        row("find_from_negative", () -> "find=" + m(HEX, "#ff0000").find(-1));
        row("find_from_then_find", () -> {
            Matcher m = m(HEX, "#111111 #222222 #333333");
            boolean a = m.find(1);
            boolean b = m.find();
            return "a=" + a + " b=" + b + " group1=" + m.group(1);
        });
        row("find_from_resets", () -> {
            Matcher m = m(HEX, "#111111 #222222");
            m.find();
            m.find();
            boolean a = m.find(0);
            return "a=" + a + " group1=" + m.group(1);
        });
        row("find_from_miss_group", () -> {
            Matcher m = m(HEX, "#111111");
            m.find();
            boolean a = m.find(3);
            return "a=" + a + " group=" + m.group();
        });
        row("find_from_bad_keeps_match", () -> {
            Matcher m = m(HEX, "#111111");
            m.find();
            String error;
            try {
                m.find(99);
                error = "none";
            } catch (IndexOutOfBoundsException e) {
                error = e.getMessage();
            }
            return "error=" + error + " group=" + m.group();
        });

        // What matches() leaves for the next call.
        row("matches_then_group", () -> {
            Matcher m = m("(\\w+)-(\\d+)", "abc-42");
            boolean a = m.matches();
            return "matches=" + a + " group=" + m.group() + " g1=" + m.group(1)
                    + " g2=" + m.group(2) + " start=" + m.start() + " end=" + m.end();
        });
        row("failed_matches_then_group", () -> {
            Matcher m = m("(\\w+)-(\\d+)", "abc-42 x");
            boolean a = m.matches();
            return "matches=" + a + " group=" + m.group();
        });
        row("matches_then_find", () -> {
            Matcher m = m("a*", "aa");
            boolean a = m.matches();
            boolean b = m.find();
            return "matches=" + a + " find=" + b + " start=" + m.start() + " end=" + m.end();
        });
        row("matches_then_find_none", () -> {
            Matcher m = m("a", "a");
            boolean a = m.matches();
            boolean b = m.find();
            return "matches=" + a + " find=" + b;
        });
        row("failed_matches_then_find", () -> {
            Matcher m = m("a", "ab");
            boolean a = m.matches();
            boolean b = m.find();
            return "matches=" + a + " find=" + b + " start=" + m.start();
        });
        row("find_then_failed_matches_then_find", () -> {
            Matcher m = m("a", "aab");
            boolean a = m.find();
            boolean b = m.matches();
            boolean c = m.find();
            return "find=" + a + " matches=" + b + " find2=" + c + " start=" + m.start();
        });
        row("find_then_matches", () -> {
            Matcher m = m("a+", "aaa");
            boolean a = m.find();
            boolean b = m.matches();
            return "find=" + a + " matches=" + b + " start=" + m.start() + " end=" + m.end();
        });
        row("matches_twice", () -> {
            Matcher m = m("a", "a");
            return "first=" + m.matches() + " second=" + m.matches();
        });
        row("matches_then_reset_find", () -> {
            Matcher m = m("a", "a");
            boolean a = m.matches();
            m.reset();
            boolean b = m.find();
            return "matches=" + a + " find=" + b + " start=" + m.start();
        });
        row("failed_matches_keeps_last", () -> {
            Matcher m = m("(?=a)", "ab");
            boolean a = m.find();
            boolean b = m.matches();
            boolean c = m.find();
            return "find=" + a + " matches=" + b + " find2=" + c;
        });

        // A miss leaves `last` where a lookahead's end set it.
        row("failed_matches_lookahead_moves_last", () -> {
            Matcher m = m("(?=a)", "aab");
            boolean a = m.find();
            boolean b = m.matches();
            boolean c = m.find();
            return "find=" + a + " matches=" + b + " find2=" + c + " start=" + m.start()
                    + " end=" + m.end();
        });
        row("failed_matches_first_matches", () -> {
            Matcher m = m("(?=a)", "ab");
            boolean b = m.matches();
            boolean c = m.find();
            return "matches=" + b + " find=" + c;
        });
        row("failed_matches_lookahead_twice", () -> {
            Matcher m = m("(?=b)", "abb");
            boolean a = m.find();
            boolean b = m.find();
            boolean c = m.matches();
            boolean d = m.find();
            return "find=" + a + b + " matches=" + c + " find3=" + d + " start=" + m.start();
        });

        // Two matchers on one pattern are independent; a method reference
        // drives the same state.
        row("two_matchers", () -> {
            Pattern p = Pattern.compile(HEX);
            Matcher a = p.matcher("#111111 #222222");
            Matcher b = p.matcher("#111111 #222222");
            a.find();
            a.find();
            b.find();
            return "a=" + a.group(1) + " b=" + b.group(1);
        });
        row("method_reference", () -> {
            Matcher m = m(HEX, "#111111 #222222");
            BooleanSupplier finder = m::find;
            boolean a = finder.getAsBoolean();
            boolean b = finder.getAsBoolean();
            boolean c = finder.getAsBoolean();
            return "a=" + a + " b=" + b + " c=" + c;
        });
        row("matcher_kept_in_static", () -> {
            keep = m(HEX, "#111111 #222222");
            keep.find();
            return "first=" + keep.group(1);
        });
        row("matcher_kept_in_static_next", () -> {
            boolean b = keep.find();
            return "second=" + b + " " + keep.group(1);
        });

        // Answered: after a failed attempt over a quantified pattern, what does
        // not read `last` (group, start, end, matches, find(int), reset).
        for (String[] c : FAILED_MATCHES) {
            for (String ops : AFTER_FAILED_MATCHES) {
                sequence(c[0], c[1], ops);
            }
        }
        for (String[] c : FAILED_FIND) {
            for (String ops : AFTER_FAILED_FIND) {
                sequence(c[0], c[1], ops);
            }
        }
        for (String[] c : FAILED_LOOK) {
            for (String ops : AFTER_FAILED_FIND) {
                sequence(c[0], c[1], ops);
            }
        }
        for (String[] c : FAILED_BEHIND) {
            for (String ops : AFTER_FAILED_FIND) {
                sequence(c[0], c[1], ops);
            }
            sequence(c[0], c[1], "ffg");
        }
        return out.toString();
    }

    /**
     * Counted: the `find()` after a failed attempt over a quantified pattern,
     * and the `group()` after it. The rows hold Java's answer; the runtime does
     * not give it. Each row ends in exactly one counted `find()` and one
     * counted `group()`.
     */
    static String counted() {
        out = new StringBuilder();
        for (String[] c : FAILED_MATCHES) {
            sequence(c[0], c[1], "mfg");
        }
        for (String[] c : FAILED_FIND) {
            sequence(c[0], c[1], "ffg");
        }
        for (String[] c : FAILED_LOOK) {
            sequence(c[0], c[1], "ffg");
        }
        // A `find(int)` that fails over a quantified pattern, then `find()`
        // (Java: `last` is 0 after the reset and no atom ran, so it is `true`),
        // and a `find()` after an empty match and a failed `matches()`.
        sequence(HEX, "#111111", "f7fg");
        sequence("x*", "ab", "fmfg");
        // A `find()` after a miss that ended a run of matches.
        sequence(HEX, "#ff0000", "fffg");
        sequence("a.*?b", "axbxab", "ffffg");
        sequence("\\[[^\\[]*]$", "foo[bar]", "fffg");
        return out.toString();
    }

    static Matcher keep;

    public static void main(String[] args) {
        System.out.print(rows());
        System.out.print("--counted--\n");
        System.out.print(counted());
    }
}
