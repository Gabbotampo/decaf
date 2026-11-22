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

    static String rows() {
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
            boolean third = m.find();
            return "find1=" + first + " find2=" + second + " find3=" + third;
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
        row("trace_nbt_path", () -> trace("\\[[^\\[]*]$", "foo[bar", 3));
        row("trace_nbt_closed", () -> trace("\\[[^\\[]*]$", "foo[bar]", 3));
        row("trace_lazy", () -> trace("a.*?b", "axbxab", 4));
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
        row("find_from_miss_then_find", () -> {
            Matcher m = m(HEX, "#111111 #222222");
            m.find();
            boolean a = m.find(8 + 7);
            boolean b = m.find();
            return "a=" + a + " b=" + b;
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
        row("failed_matches_empty_star", () -> {
            Matcher m = m("x*", "ab");
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
        return out.toString();
    }

    static Matcher keep;

    public static void main(String[] args) {
        System.out.print(rows());
    }
}
