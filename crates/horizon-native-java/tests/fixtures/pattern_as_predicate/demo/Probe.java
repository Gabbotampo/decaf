package demo;

import java.util.ArrayList;
import java.util.List;
import java.util.function.Predicate;
import java.util.regex.Pattern;
import java.util.stream.Collectors;
import java.util.stream.Stream;

public final class Probe {
    private static void row(StringBuilder b, String name, boolean value) {
        b.append(name).append(" = ").append(value).append('\n');
    }

    /** A `Predicate<String>` passed in as an argument: the call site is not the one that made it. */
    private static boolean apply(Predicate<String> predicate, String input) {
        return predicate.test(input);
    }

    /** A jar class implementing Predicate itself. */
    private static final class StartsWithA implements Predicate<String> {
        @Override
        public boolean test(String s) {
            return s.startsWith("a");
        }
    }

    public static String rows() {
        StringBuilder b = new StringBuilder();

        // Anchored pattern (the shape Jankson uses for unquoted keys).
        Predicate<String> key = Pattern.compile("^[a-zA-Z0-9]+$").asPredicate();
        row(b, "key validKey", key.test("validKey"));
        row(b, "key abc123", key.test("abc123"));
        row(b, "key invalid-key", key.test("invalid-key"));
        row(b, "key abc_def", key.test("abc_def"));
        row(b, "key empty", key.test(""));
        row(b, "key underscore", key.test("_"));
        row(b, "key dollar", key.test("$"));
        // `$` also matches before a final line terminator.
        row(b, "key abc LF", key.test("abc\n"));
        row(b, "key abc CRLF", key.test("abc\r\n"));
        row(b, "key abc LF def", key.test("abc\ndef"));
        row(b, "key LF abc", key.test("\nabc"));

        // Unanchored: find semantics, not matches.
        Predicate<String> digits = Pattern.compile("[0-9]+").asPredicate();
        row(b, "digits abc123def", digits.test("abc123def"));
        row(b, "digits abc", digits.test("abc"));
        row(b, "digits empty", digits.test(""));
        row(b, "digits 123", digits.test("123"));
        Predicate<String> a = Pattern.compile("a").asPredicate();
        row(b, "a bab", a.test("bab"));
        row(b, "a xyz", a.test("xyz"));

        // The empty pattern occurs everywhere, including in the empty string.
        Predicate<String> empty = Pattern.compile("").asPredicate();
        row(b, "empty-pattern empty", empty.test(""));
        row(b, "empty-pattern x", empty.test("x"));
        Predicate<String> bothAnchors = Pattern.compile("^$").asPredicate();
        row(b, "anchors empty", bothAnchors.test(""));
        row(b, "anchors LF", bothAnchors.test("\n"));
        row(b, "anchors x", bothAnchors.test("x"));

        // Word boundary, lookbehind, backtracking alternation.
        Predicate<String> word = Pattern.compile("\\bfoo\\b").asPredicate();
        row(b, "word a foo b", word.test("a foo b"));
        row(b, "word afoo", word.test("afoo"));
        row(b, "word foo", word.test("foo"));
        Predicate<String> behind = Pattern.compile("(?<=x)y").asPredicate();
        row(b, "behind xy", behind.test("xy"));
        row(b, "behind y", behind.test("y"));
        Predicate<String> alt = Pattern.compile("(a|ab)(c|bcd)").asPredicate();
        row(b, "alt abcd", alt.test("abcd"));
        row(b, "alt abd", alt.test("abd"));

        // Non-ASCII text in the Basic Multilingual Plane.
        Predicate<String> cafe = Pattern.compile("caf\u00e9").asPredicate();
        row(b, "cafe cafe-acute", cafe.test("un caf\u00e9 noir"));
        row(b, "cafe CAFE-acute", cafe.test("CAF\u00c9"));

        // A predicate keeps no state between calls, and each call to asPredicate is a new object.
        row(b, "key again validKey", key.test("validKey"));
        row(b, "key again invalid-key", key.test("invalid-key"));
        Pattern shared = Pattern.compile("[0-9]");
        Predicate<String> first = shared.asPredicate();
        Predicate<String> second = shared.asPredicate();
        row(b, "first-test-1", first.test("1"));
        row(b, "second-test-x", second.test("x"));
        row(b, "first-test-1-again", first.test("1"));
        row(b, "same-object", first == second);
        row(b, "equals-self", first.equals(first));
        row(b, "equals-other", first.equals(second));

        // The call site that holds it as an `Object`, and as the interface it is.
        Object asObject = first;
        row(b, "instanceof-Predicate", asObject instanceof Predicate);
        row(b, "instanceof-Runnable", asObject instanceof Runnable);

        // Reached through a helper that takes any Predicate<String>: a predicate
        // the pattern made, a lambda, a method reference and a jar class all take
        // the same call.
        row(b, "apply pattern yes", apply(digits, "x9"));
        row(b, "apply pattern no", apply(digits, "x"));
        row(b, "apply lambda yes", apply(s -> s.length() == 3, "abc"));
        row(b, "apply lambda no", apply(s -> s.length() == 3, "ab"));
        row(b, "apply methodref yes", apply(String::isEmpty, ""));
        row(b, "apply methodref no", apply(String::isEmpty, "x"));
        row(b, "apply jar-class yes", apply(new StartsWithA(), "abc"));
        row(b, "apply jar-class no", apply(new StartsWithA(), "bca"));
        Predicate<String> anonymous = new Predicate<String>() {
            @Override
            public boolean test(String s) {
                return s.equals("anon");
            }
        };
        row(b, "apply anonymous yes", apply(anonymous, "anon"));
        row(b, "apply anonymous no", apply(anonymous, "other"));

        // The predicate handed to the JDK's own consumers of one, and combined.
        b.append("stream filter = ")
            .append(Stream.of("a1", "b", "c2").filter(digits).collect(Collectors.toList()))
            .append('\n');
        List<String> removable = new ArrayList<>(List.of("a1", "b", "c2"));
        row(b, "removeIf changed", removable.removeIf(digits));
        b.append("removeIf left = ").append(removable).append('\n');
        List<String> untouched = new ArrayList<>(List.of("b", "c"));
        row(b, "removeIf none changed", untouched.removeIf(digits));
        b.append("removeIf none left = ").append(untouched).append('\n');
        Predicate<String> letters = Pattern.compile("[a-z]").asPredicate();
        row(b, "negate 1", digits.negate().test("1"));
        row(b, "negate x", digits.negate().test("x"));
        row(b, "and 1a", digits.and(letters).test("1a"));
        row(b, "and 1", digits.and(letters).test("1"));
        row(b, "or x", digits.or(letters).test("x"));
        row(b, "or ?", digits.or(letters).test("?"));
        return b.toString();
    }

    /** Pattern flags other than 0: the runtime does not model them, so these are counted. */
    public static String flagged() {
        Predicate<String> p = Pattern.compile("abc", Pattern.CASE_INSENSITIVE).asPredicate();
        return "flagged ABC = " + p.test("ABC");
    }

    /**
     * An input with a supplementary character (`.` is one code point there): the
     * engine reads the input as UTF-16 units it cannot build, so the runtime counts it.
     */
    public static String supplementary() {
        String face = new String(Character.toChars(0x1F600));
        StringBuilder b = new StringBuilder();
        b.append("one face = ").append(Pattern.compile("^.$").asPredicate().test(face)).append('\n');
        b.append("two face = ").append(Pattern.compile("^..$").asPredicate().test(face)).append('\n');
        List<String> faces = new ArrayList<>(List.of(face, "b"));
        faces.removeIf(Pattern.compile("^.$").asPredicate());
        b.append("removeIf left = ").append(faces.size()).append('\n');
        return b.toString();
    }

    /** `test` on an argument that is not a String: the JDK throws, the runtime counts it. */
    @SuppressWarnings({"unchecked", "rawtypes"})
    public static String nonString() {
        Predicate raw = Pattern.compile("a").asPredicate();
        StringBuilder b = new StringBuilder();
        String builder;
        try {
            builder = String.valueOf(raw.test(new StringBuilder("a")));
        } catch (ClassCastException e) {
            builder = "ClassCastException";
        }
        b.append("builder = ").append(builder).append('\n');
        String none;
        try {
            none = String.valueOf(raw.test(null));
        } catch (NullPointerException e) {
            none = "NullPointerException";
        }
        b.append("null = ").append(none).append('\n');
        String integer;
        try {
            integer = String.valueOf(raw.test(Integer.valueOf(1)));
        } catch (ClassCastException e) {
            integer = "ClassCastException";
        }
        b.append("integer = ").append(integer).append('\n');
        return b.toString();
    }

    /** `java demo.Probe` prints the oracle rows; `java demo.Probe counted` the cases the runtime counts. */
    public static void main(String[] args) {
        if (args.length > 0 && args[0].equals("counted")) {
            System.out.println(flagged());
            System.out.print(supplementary());
            System.out.print(nonString());
        } else {
            System.out.print(rows());
        }
    }
}
