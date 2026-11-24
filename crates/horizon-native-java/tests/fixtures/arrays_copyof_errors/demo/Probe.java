package demo;

import java.util.Arrays;

/**
 * Probe for the argument errors of {@code Arrays.copyOf} and {@code Arrays.copyOfRange}.
 * Every row prints {@code label=value}: the copy as {@code Arrays.toString} (or
 * {@code deepToString}) shows it, or {@code class:message} of what the call threw.
 * {@code rows()} are the rows the runtime answers, {@code countedRows()} the ones it
 * still counts.
 */
public class Probe {
    interface Body {
        String run() throws Throwable;
    }

    static void row(StringBuilder out, String label, Body body) {
        String value;
        try {
            value = body.run();
        } catch (Throwable t) {
            value = t.getClass().getName() + ":" + t.getMessage();
        }
        out.append(label).append('=').append(value).append('\n');
    }

    /** A {@code char[]} as text, its NUL padding spelled 0. */
    static String chars(char[] a) {
        return Arrays.toString(a).replace('\0', '0');
    }

    public static String rows() {
        StringBuilder out = new StringBuilder();
        row(out, "int.copyOf.negLen", () -> Arrays.toString(Arrays.copyOf(new int[]{1, 2, 3}, -1)));
        row(out, "int.copyOf.minLen", () -> Arrays.toString(Arrays.copyOf(new int[]{1, 2, 3}, Integer.MIN_VALUE)));
        row(out, "int.copyOf.zero", () -> Arrays.toString(Arrays.copyOf(new int[]{1, 2, 3}, 0)));
        row(out, "int.copyOf.short", () -> Arrays.toString(Arrays.copyOf(new int[]{1, 2, 3}, 2)));
        row(out, "int.copyOf.same", () -> Arrays.toString(Arrays.copyOf(new int[]{1, 2, 3}, 3)));
        row(out, "int.copyOf.pad", () -> Arrays.toString(Arrays.copyOf(new int[]{1, 2, 3}, 5)));
        row(out, "int.copyOfRange.fromGtTo", () -> Arrays.toString(Arrays.copyOfRange(new int[]{1, 2, 3}, 3, 1)));
        row(out, "int.copyOfRange.fromEqTo", () -> Arrays.toString(Arrays.copyOfRange(new int[]{1, 2, 3}, 2, 2)));
        row(out, "int.copyOfRange.fromNeg", () -> Arrays.toString(Arrays.copyOfRange(new int[]{1, 2, 3}, -1, 2)));
        row(out, "int.copyOfRange.fromGtLen", () -> Arrays.toString(Arrays.copyOfRange(new int[]{1, 2, 3}, 5, 6)));
        row(out, "int.copyOfRange.fromLenPlus1", () -> Arrays.toString(Arrays.copyOfRange(new int[]{1, 2, 3}, 4, 4)));
        row(out, "int.copyOfRange.fromEqLen", () -> Arrays.toString(Arrays.copyOfRange(new int[]{1, 2, 3}, 3, 5)));
        row(out, "int.copyOfRange.toGtLen", () -> Arrays.toString(Arrays.copyOfRange(new int[]{1, 2, 3}, 0, 5)));
        row(out, "int.copyOfRange.toNeg", () -> Arrays.toString(Arrays.copyOfRange(new int[]{1, 2, 3}, 0, -1)));
        row(out, "int.copyOfRange.bothNeg", () -> Arrays.toString(Arrays.copyOfRange(new int[]{1, 2, 3}, -5, -4)));
        row(out, "int.copyOfRange.negGt", () -> Arrays.toString(Arrays.copyOfRange(new int[]{1, 2, 3}, -1, -2)));
        row(out, "int.copyOfRange.wrapNeg", () -> Arrays.toString(Arrays.copyOfRange(new int[]{1, 2, 3}, -5, Integer.MAX_VALUE)));
        row(out, "int.copyOfRange.wrapMinMax", () -> Arrays.toString(Arrays.copyOfRange(new int[]{1, 2, 3}, Integer.MIN_VALUE, Integer.MAX_VALUE)));
        row(out, "int.copyOfRange.wrapMaxMin", () -> Arrays.toString(Arrays.copyOfRange(new int[]{1, 2, 3}, Integer.MAX_VALUE, Integer.MIN_VALUE)));
        row(out, "int.copyOfRange.full", () -> Arrays.toString(Arrays.copyOfRange(new int[]{1, 2, 3}, 0, 3)));
        row(out, "int.copyOfRange.mid", () -> Arrays.toString(Arrays.copyOfRange(new int[]{1, 2, 3}, 1, 2)));
        row(out, "long.copyOf.negLen", () -> Arrays.toString(Arrays.copyOf(new long[]{1, 2, 3}, -1)));
        row(out, "long.copyOf.minLen", () -> Arrays.toString(Arrays.copyOf(new long[]{1, 2, 3}, Integer.MIN_VALUE)));
        row(out, "long.copyOf.zero", () -> Arrays.toString(Arrays.copyOf(new long[]{1, 2, 3}, 0)));
        row(out, "long.copyOf.short", () -> Arrays.toString(Arrays.copyOf(new long[]{1, 2, 3}, 2)));
        row(out, "long.copyOf.same", () -> Arrays.toString(Arrays.copyOf(new long[]{1, 2, 3}, 3)));
        row(out, "long.copyOf.pad", () -> Arrays.toString(Arrays.copyOf(new long[]{1, 2, 3}, 5)));
        row(out, "long.copyOfRange.fromGtTo", () -> Arrays.toString(Arrays.copyOfRange(new long[]{1, 2, 3}, 3, 1)));
        row(out, "long.copyOfRange.fromEqTo", () -> Arrays.toString(Arrays.copyOfRange(new long[]{1, 2, 3}, 2, 2)));
        row(out, "long.copyOfRange.fromNeg", () -> Arrays.toString(Arrays.copyOfRange(new long[]{1, 2, 3}, -1, 2)));
        row(out, "long.copyOfRange.fromGtLen", () -> Arrays.toString(Arrays.copyOfRange(new long[]{1, 2, 3}, 5, 6)));
        row(out, "long.copyOfRange.fromLenPlus1", () -> Arrays.toString(Arrays.copyOfRange(new long[]{1, 2, 3}, 4, 4)));
        row(out, "long.copyOfRange.fromEqLen", () -> Arrays.toString(Arrays.copyOfRange(new long[]{1, 2, 3}, 3, 5)));
        row(out, "long.copyOfRange.toGtLen", () -> Arrays.toString(Arrays.copyOfRange(new long[]{1, 2, 3}, 0, 5)));
        row(out, "long.copyOfRange.toNeg", () -> Arrays.toString(Arrays.copyOfRange(new long[]{1, 2, 3}, 0, -1)));
        row(out, "long.copyOfRange.bothNeg", () -> Arrays.toString(Arrays.copyOfRange(new long[]{1, 2, 3}, -5, -4)));
        row(out, "long.copyOfRange.negGt", () -> Arrays.toString(Arrays.copyOfRange(new long[]{1, 2, 3}, -1, -2)));
        row(out, "long.copyOfRange.wrapNeg", () -> Arrays.toString(Arrays.copyOfRange(new long[]{1, 2, 3}, -5, Integer.MAX_VALUE)));
        row(out, "long.copyOfRange.wrapMinMax", () -> Arrays.toString(Arrays.copyOfRange(new long[]{1, 2, 3}, Integer.MIN_VALUE, Integer.MAX_VALUE)));
        row(out, "long.copyOfRange.wrapMaxMin", () -> Arrays.toString(Arrays.copyOfRange(new long[]{1, 2, 3}, Integer.MAX_VALUE, Integer.MIN_VALUE)));
        row(out, "long.copyOfRange.full", () -> Arrays.toString(Arrays.copyOfRange(new long[]{1, 2, 3}, 0, 3)));
        row(out, "long.copyOfRange.mid", () -> Arrays.toString(Arrays.copyOfRange(new long[]{1, 2, 3}, 1, 2)));
        row(out, "double.copyOf.negLen", () -> Arrays.toString(Arrays.copyOf(new double[]{1, 2, 3}, -1)));
        row(out, "double.copyOf.minLen", () -> Arrays.toString(Arrays.copyOf(new double[]{1, 2, 3}, Integer.MIN_VALUE)));
        row(out, "double.copyOf.zero", () -> Arrays.toString(Arrays.copyOf(new double[]{1, 2, 3}, 0)));
        row(out, "double.copyOf.short", () -> Arrays.toString(Arrays.copyOf(new double[]{1, 2, 3}, 2)));
        row(out, "double.copyOf.same", () -> Arrays.toString(Arrays.copyOf(new double[]{1, 2, 3}, 3)));
        row(out, "double.copyOf.pad", () -> Arrays.toString(Arrays.copyOf(new double[]{1, 2, 3}, 5)));
        row(out, "double.copyOfRange.fromGtTo", () -> Arrays.toString(Arrays.copyOfRange(new double[]{1, 2, 3}, 3, 1)));
        row(out, "double.copyOfRange.fromEqTo", () -> Arrays.toString(Arrays.copyOfRange(new double[]{1, 2, 3}, 2, 2)));
        row(out, "double.copyOfRange.fromNeg", () -> Arrays.toString(Arrays.copyOfRange(new double[]{1, 2, 3}, -1, 2)));
        row(out, "double.copyOfRange.fromGtLen", () -> Arrays.toString(Arrays.copyOfRange(new double[]{1, 2, 3}, 5, 6)));
        row(out, "double.copyOfRange.fromLenPlus1", () -> Arrays.toString(Arrays.copyOfRange(new double[]{1, 2, 3}, 4, 4)));
        row(out, "double.copyOfRange.fromEqLen", () -> Arrays.toString(Arrays.copyOfRange(new double[]{1, 2, 3}, 3, 5)));
        row(out, "double.copyOfRange.toGtLen", () -> Arrays.toString(Arrays.copyOfRange(new double[]{1, 2, 3}, 0, 5)));
        row(out, "double.copyOfRange.toNeg", () -> Arrays.toString(Arrays.copyOfRange(new double[]{1, 2, 3}, 0, -1)));
        row(out, "double.copyOfRange.bothNeg", () -> Arrays.toString(Arrays.copyOfRange(new double[]{1, 2, 3}, -5, -4)));
        row(out, "double.copyOfRange.negGt", () -> Arrays.toString(Arrays.copyOfRange(new double[]{1, 2, 3}, -1, -2)));
        row(out, "double.copyOfRange.wrapNeg", () -> Arrays.toString(Arrays.copyOfRange(new double[]{1, 2, 3}, -5, Integer.MAX_VALUE)));
        row(out, "double.copyOfRange.wrapMinMax", () -> Arrays.toString(Arrays.copyOfRange(new double[]{1, 2, 3}, Integer.MIN_VALUE, Integer.MAX_VALUE)));
        row(out, "double.copyOfRange.wrapMaxMin", () -> Arrays.toString(Arrays.copyOfRange(new double[]{1, 2, 3}, Integer.MAX_VALUE, Integer.MIN_VALUE)));
        row(out, "double.copyOfRange.full", () -> Arrays.toString(Arrays.copyOfRange(new double[]{1, 2, 3}, 0, 3)));
        row(out, "double.copyOfRange.mid", () -> Arrays.toString(Arrays.copyOfRange(new double[]{1, 2, 3}, 1, 2)));
        row(out, "float.copyOf.negLen", () -> Arrays.toString(Arrays.copyOf(new float[]{1, 2, 3}, -1)));
        row(out, "float.copyOf.minLen", () -> Arrays.toString(Arrays.copyOf(new float[]{1, 2, 3}, Integer.MIN_VALUE)));
        row(out, "float.copyOf.zero", () -> Arrays.toString(Arrays.copyOf(new float[]{1, 2, 3}, 0)));
        row(out, "float.copyOf.short", () -> Arrays.toString(Arrays.copyOf(new float[]{1, 2, 3}, 2)));
        row(out, "float.copyOf.same", () -> Arrays.toString(Arrays.copyOf(new float[]{1, 2, 3}, 3)));
        row(out, "float.copyOf.pad", () -> Arrays.toString(Arrays.copyOf(new float[]{1, 2, 3}, 5)));
        row(out, "float.copyOfRange.fromGtTo", () -> Arrays.toString(Arrays.copyOfRange(new float[]{1, 2, 3}, 3, 1)));
        row(out, "float.copyOfRange.fromEqTo", () -> Arrays.toString(Arrays.copyOfRange(new float[]{1, 2, 3}, 2, 2)));
        row(out, "float.copyOfRange.fromNeg", () -> Arrays.toString(Arrays.copyOfRange(new float[]{1, 2, 3}, -1, 2)));
        row(out, "float.copyOfRange.fromGtLen", () -> Arrays.toString(Arrays.copyOfRange(new float[]{1, 2, 3}, 5, 6)));
        row(out, "float.copyOfRange.fromLenPlus1", () -> Arrays.toString(Arrays.copyOfRange(new float[]{1, 2, 3}, 4, 4)));
        row(out, "float.copyOfRange.fromEqLen", () -> Arrays.toString(Arrays.copyOfRange(new float[]{1, 2, 3}, 3, 5)));
        row(out, "float.copyOfRange.toGtLen", () -> Arrays.toString(Arrays.copyOfRange(new float[]{1, 2, 3}, 0, 5)));
        row(out, "float.copyOfRange.toNeg", () -> Arrays.toString(Arrays.copyOfRange(new float[]{1, 2, 3}, 0, -1)));
        row(out, "float.copyOfRange.bothNeg", () -> Arrays.toString(Arrays.copyOfRange(new float[]{1, 2, 3}, -5, -4)));
        row(out, "float.copyOfRange.negGt", () -> Arrays.toString(Arrays.copyOfRange(new float[]{1, 2, 3}, -1, -2)));
        row(out, "float.copyOfRange.wrapNeg", () -> Arrays.toString(Arrays.copyOfRange(new float[]{1, 2, 3}, -5, Integer.MAX_VALUE)));
        row(out, "float.copyOfRange.wrapMinMax", () -> Arrays.toString(Arrays.copyOfRange(new float[]{1, 2, 3}, Integer.MIN_VALUE, Integer.MAX_VALUE)));
        row(out, "float.copyOfRange.wrapMaxMin", () -> Arrays.toString(Arrays.copyOfRange(new float[]{1, 2, 3}, Integer.MAX_VALUE, Integer.MIN_VALUE)));
        row(out, "float.copyOfRange.full", () -> Arrays.toString(Arrays.copyOfRange(new float[]{1, 2, 3}, 0, 3)));
        row(out, "float.copyOfRange.mid", () -> Arrays.toString(Arrays.copyOfRange(new float[]{1, 2, 3}, 1, 2)));
        row(out, "short.copyOf.negLen", () -> Arrays.toString(Arrays.copyOf(new short[]{1, 2, 3}, -1)));
        row(out, "short.copyOf.minLen", () -> Arrays.toString(Arrays.copyOf(new short[]{1, 2, 3}, Integer.MIN_VALUE)));
        row(out, "short.copyOf.zero", () -> Arrays.toString(Arrays.copyOf(new short[]{1, 2, 3}, 0)));
        row(out, "short.copyOf.short", () -> Arrays.toString(Arrays.copyOf(new short[]{1, 2, 3}, 2)));
        row(out, "short.copyOf.same", () -> Arrays.toString(Arrays.copyOf(new short[]{1, 2, 3}, 3)));
        row(out, "short.copyOf.pad", () -> Arrays.toString(Arrays.copyOf(new short[]{1, 2, 3}, 5)));
        row(out, "short.copyOfRange.fromGtTo", () -> Arrays.toString(Arrays.copyOfRange(new short[]{1, 2, 3}, 3, 1)));
        row(out, "short.copyOfRange.fromEqTo", () -> Arrays.toString(Arrays.copyOfRange(new short[]{1, 2, 3}, 2, 2)));
        row(out, "short.copyOfRange.fromNeg", () -> Arrays.toString(Arrays.copyOfRange(new short[]{1, 2, 3}, -1, 2)));
        row(out, "short.copyOfRange.fromGtLen", () -> Arrays.toString(Arrays.copyOfRange(new short[]{1, 2, 3}, 5, 6)));
        row(out, "short.copyOfRange.fromLenPlus1", () -> Arrays.toString(Arrays.copyOfRange(new short[]{1, 2, 3}, 4, 4)));
        row(out, "short.copyOfRange.fromEqLen", () -> Arrays.toString(Arrays.copyOfRange(new short[]{1, 2, 3}, 3, 5)));
        row(out, "short.copyOfRange.toGtLen", () -> Arrays.toString(Arrays.copyOfRange(new short[]{1, 2, 3}, 0, 5)));
        row(out, "short.copyOfRange.toNeg", () -> Arrays.toString(Arrays.copyOfRange(new short[]{1, 2, 3}, 0, -1)));
        row(out, "short.copyOfRange.bothNeg", () -> Arrays.toString(Arrays.copyOfRange(new short[]{1, 2, 3}, -5, -4)));
        row(out, "short.copyOfRange.negGt", () -> Arrays.toString(Arrays.copyOfRange(new short[]{1, 2, 3}, -1, -2)));
        row(out, "short.copyOfRange.wrapNeg", () -> Arrays.toString(Arrays.copyOfRange(new short[]{1, 2, 3}, -5, Integer.MAX_VALUE)));
        row(out, "short.copyOfRange.wrapMinMax", () -> Arrays.toString(Arrays.copyOfRange(new short[]{1, 2, 3}, Integer.MIN_VALUE, Integer.MAX_VALUE)));
        row(out, "short.copyOfRange.wrapMaxMin", () -> Arrays.toString(Arrays.copyOfRange(new short[]{1, 2, 3}, Integer.MAX_VALUE, Integer.MIN_VALUE)));
        row(out, "short.copyOfRange.full", () -> Arrays.toString(Arrays.copyOfRange(new short[]{1, 2, 3}, 0, 3)));
        row(out, "short.copyOfRange.mid", () -> Arrays.toString(Arrays.copyOfRange(new short[]{1, 2, 3}, 1, 2)));
        row(out, "byte.copyOf.negLen", () -> Arrays.toString(Arrays.copyOf(new byte[]{1, 2, 3}, -1)));
        row(out, "byte.copyOf.minLen", () -> Arrays.toString(Arrays.copyOf(new byte[]{1, 2, 3}, Integer.MIN_VALUE)));
        row(out, "byte.copyOf.zero", () -> Arrays.toString(Arrays.copyOf(new byte[]{1, 2, 3}, 0)));
        row(out, "byte.copyOf.short", () -> Arrays.toString(Arrays.copyOf(new byte[]{1, 2, 3}, 2)));
        row(out, "byte.copyOf.same", () -> Arrays.toString(Arrays.copyOf(new byte[]{1, 2, 3}, 3)));
        row(out, "byte.copyOf.pad", () -> Arrays.toString(Arrays.copyOf(new byte[]{1, 2, 3}, 5)));
        row(out, "byte.copyOfRange.fromGtTo", () -> Arrays.toString(Arrays.copyOfRange(new byte[]{1, 2, 3}, 3, 1)));
        row(out, "byte.copyOfRange.fromEqTo", () -> Arrays.toString(Arrays.copyOfRange(new byte[]{1, 2, 3}, 2, 2)));
        row(out, "byte.copyOfRange.fromNeg", () -> Arrays.toString(Arrays.copyOfRange(new byte[]{1, 2, 3}, -1, 2)));
        row(out, "byte.copyOfRange.fromGtLen", () -> Arrays.toString(Arrays.copyOfRange(new byte[]{1, 2, 3}, 5, 6)));
        row(out, "byte.copyOfRange.fromLenPlus1", () -> Arrays.toString(Arrays.copyOfRange(new byte[]{1, 2, 3}, 4, 4)));
        row(out, "byte.copyOfRange.fromEqLen", () -> Arrays.toString(Arrays.copyOfRange(new byte[]{1, 2, 3}, 3, 5)));
        row(out, "byte.copyOfRange.toGtLen", () -> Arrays.toString(Arrays.copyOfRange(new byte[]{1, 2, 3}, 0, 5)));
        row(out, "byte.copyOfRange.toNeg", () -> Arrays.toString(Arrays.copyOfRange(new byte[]{1, 2, 3}, 0, -1)));
        row(out, "byte.copyOfRange.bothNeg", () -> Arrays.toString(Arrays.copyOfRange(new byte[]{1, 2, 3}, -5, -4)));
        row(out, "byte.copyOfRange.negGt", () -> Arrays.toString(Arrays.copyOfRange(new byte[]{1, 2, 3}, -1, -2)));
        row(out, "byte.copyOfRange.wrapNeg", () -> Arrays.toString(Arrays.copyOfRange(new byte[]{1, 2, 3}, -5, Integer.MAX_VALUE)));
        row(out, "byte.copyOfRange.wrapMinMax", () -> Arrays.toString(Arrays.copyOfRange(new byte[]{1, 2, 3}, Integer.MIN_VALUE, Integer.MAX_VALUE)));
        row(out, "byte.copyOfRange.wrapMaxMin", () -> Arrays.toString(Arrays.copyOfRange(new byte[]{1, 2, 3}, Integer.MAX_VALUE, Integer.MIN_VALUE)));
        row(out, "byte.copyOfRange.full", () -> Arrays.toString(Arrays.copyOfRange(new byte[]{1, 2, 3}, 0, 3)));
        row(out, "byte.copyOfRange.mid", () -> Arrays.toString(Arrays.copyOfRange(new byte[]{1, 2, 3}, 1, 2)));
        row(out, "char.copyOf.negLen", () -> chars(Arrays.copyOf(new char[]{'a', 'b', 'c'}, -1)));
        row(out, "char.copyOf.minLen", () -> chars(Arrays.copyOf(new char[]{'a', 'b', 'c'}, Integer.MIN_VALUE)));
        row(out, "char.copyOf.zero", () -> chars(Arrays.copyOf(new char[]{'a', 'b', 'c'}, 0)));
        row(out, "char.copyOf.short", () -> chars(Arrays.copyOf(new char[]{'a', 'b', 'c'}, 2)));
        row(out, "char.copyOf.same", () -> chars(Arrays.copyOf(new char[]{'a', 'b', 'c'}, 3)));
        row(out, "char.copyOf.pad", () -> chars(Arrays.copyOf(new char[]{'a', 'b', 'c'}, 5)));
        row(out, "char.copyOfRange.fromGtTo", () -> chars(Arrays.copyOfRange(new char[]{'a', 'b', 'c'}, 3, 1)));
        row(out, "char.copyOfRange.fromEqTo", () -> chars(Arrays.copyOfRange(new char[]{'a', 'b', 'c'}, 2, 2)));
        row(out, "char.copyOfRange.fromNeg", () -> chars(Arrays.copyOfRange(new char[]{'a', 'b', 'c'}, -1, 2)));
        row(out, "char.copyOfRange.fromGtLen", () -> chars(Arrays.copyOfRange(new char[]{'a', 'b', 'c'}, 5, 6)));
        row(out, "char.copyOfRange.fromLenPlus1", () -> chars(Arrays.copyOfRange(new char[]{'a', 'b', 'c'}, 4, 4)));
        row(out, "char.copyOfRange.fromEqLen", () -> chars(Arrays.copyOfRange(new char[]{'a', 'b', 'c'}, 3, 5)));
        row(out, "char.copyOfRange.toGtLen", () -> chars(Arrays.copyOfRange(new char[]{'a', 'b', 'c'}, 0, 5)));
        row(out, "char.copyOfRange.toNeg", () -> chars(Arrays.copyOfRange(new char[]{'a', 'b', 'c'}, 0, -1)));
        row(out, "char.copyOfRange.bothNeg", () -> chars(Arrays.copyOfRange(new char[]{'a', 'b', 'c'}, -5, -4)));
        row(out, "char.copyOfRange.negGt", () -> chars(Arrays.copyOfRange(new char[]{'a', 'b', 'c'}, -1, -2)));
        row(out, "char.copyOfRange.wrapNeg", () -> chars(Arrays.copyOfRange(new char[]{'a', 'b', 'c'}, -5, Integer.MAX_VALUE)));
        row(out, "char.copyOfRange.wrapMinMax", () -> chars(Arrays.copyOfRange(new char[]{'a', 'b', 'c'}, Integer.MIN_VALUE, Integer.MAX_VALUE)));
        row(out, "char.copyOfRange.wrapMaxMin", () -> chars(Arrays.copyOfRange(new char[]{'a', 'b', 'c'}, Integer.MAX_VALUE, Integer.MIN_VALUE)));
        row(out, "char.copyOfRange.full", () -> chars(Arrays.copyOfRange(new char[]{'a', 'b', 'c'}, 0, 3)));
        row(out, "char.copyOfRange.mid", () -> chars(Arrays.copyOfRange(new char[]{'a', 'b', 'c'}, 1, 2)));
        row(out, "boolean.copyOf.negLen", () -> Arrays.toString(Arrays.copyOf(new boolean[]{true, false, true}, -1)));
        row(out, "boolean.copyOf.minLen", () -> Arrays.toString(Arrays.copyOf(new boolean[]{true, false, true}, Integer.MIN_VALUE)));
        row(out, "boolean.copyOf.zero", () -> Arrays.toString(Arrays.copyOf(new boolean[]{true, false, true}, 0)));
        row(out, "boolean.copyOf.short", () -> Arrays.toString(Arrays.copyOf(new boolean[]{true, false, true}, 2)));
        row(out, "boolean.copyOf.same", () -> Arrays.toString(Arrays.copyOf(new boolean[]{true, false, true}, 3)));
        row(out, "boolean.copyOf.pad", () -> Arrays.toString(Arrays.copyOf(new boolean[]{true, false, true}, 5)));
        row(out, "boolean.copyOfRange.fromGtTo", () -> Arrays.toString(Arrays.copyOfRange(new boolean[]{true, false, true}, 3, 1)));
        row(out, "boolean.copyOfRange.fromEqTo", () -> Arrays.toString(Arrays.copyOfRange(new boolean[]{true, false, true}, 2, 2)));
        row(out, "boolean.copyOfRange.fromNeg", () -> Arrays.toString(Arrays.copyOfRange(new boolean[]{true, false, true}, -1, 2)));
        row(out, "boolean.copyOfRange.fromGtLen", () -> Arrays.toString(Arrays.copyOfRange(new boolean[]{true, false, true}, 5, 6)));
        row(out, "boolean.copyOfRange.fromLenPlus1", () -> Arrays.toString(Arrays.copyOfRange(new boolean[]{true, false, true}, 4, 4)));
        row(out, "boolean.copyOfRange.fromEqLen", () -> Arrays.toString(Arrays.copyOfRange(new boolean[]{true, false, true}, 3, 5)));
        row(out, "boolean.copyOfRange.toGtLen", () -> Arrays.toString(Arrays.copyOfRange(new boolean[]{true, false, true}, 0, 5)));
        row(out, "boolean.copyOfRange.toNeg", () -> Arrays.toString(Arrays.copyOfRange(new boolean[]{true, false, true}, 0, -1)));
        row(out, "boolean.copyOfRange.bothNeg", () -> Arrays.toString(Arrays.copyOfRange(new boolean[]{true, false, true}, -5, -4)));
        row(out, "boolean.copyOfRange.negGt", () -> Arrays.toString(Arrays.copyOfRange(new boolean[]{true, false, true}, -1, -2)));
        row(out, "boolean.copyOfRange.wrapNeg", () -> Arrays.toString(Arrays.copyOfRange(new boolean[]{true, false, true}, -5, Integer.MAX_VALUE)));
        row(out, "boolean.copyOfRange.wrapMinMax", () -> Arrays.toString(Arrays.copyOfRange(new boolean[]{true, false, true}, Integer.MIN_VALUE, Integer.MAX_VALUE)));
        row(out, "boolean.copyOfRange.wrapMaxMin", () -> Arrays.toString(Arrays.copyOfRange(new boolean[]{true, false, true}, Integer.MAX_VALUE, Integer.MIN_VALUE)));
        row(out, "boolean.copyOfRange.full", () -> Arrays.toString(Arrays.copyOfRange(new boolean[]{true, false, true}, 0, 3)));
        row(out, "boolean.copyOfRange.mid", () -> Arrays.toString(Arrays.copyOfRange(new boolean[]{true, false, true}, 1, 2)));
        row(out, "String.copyOf.negLen", () -> Arrays.toString(Arrays.copyOf(new String[]{"a", "b", "c"}, -1)));
        row(out, "String.copyOf.minLen", () -> Arrays.toString(Arrays.copyOf(new String[]{"a", "b", "c"}, Integer.MIN_VALUE)));
        row(out, "String.copyOf.zero", () -> Arrays.toString(Arrays.copyOf(new String[]{"a", "b", "c"}, 0)));
        row(out, "String.copyOf.short", () -> Arrays.toString(Arrays.copyOf(new String[]{"a", "b", "c"}, 2)));
        row(out, "String.copyOf.same", () -> Arrays.toString(Arrays.copyOf(new String[]{"a", "b", "c"}, 3)));
        row(out, "String.copyOf.pad", () -> Arrays.toString(Arrays.copyOf(new String[]{"a", "b", "c"}, 5)));
        row(out, "String.copyOfRange.fromGtTo", () -> Arrays.toString(Arrays.copyOfRange(new String[]{"a", "b", "c"}, 3, 1)));
        row(out, "String.copyOfRange.fromEqTo", () -> Arrays.toString(Arrays.copyOfRange(new String[]{"a", "b", "c"}, 2, 2)));
        row(out, "String.copyOfRange.fromNeg", () -> Arrays.toString(Arrays.copyOfRange(new String[]{"a", "b", "c"}, -1, 2)));
        row(out, "String.copyOfRange.fromGtLen", () -> Arrays.toString(Arrays.copyOfRange(new String[]{"a", "b", "c"}, 5, 6)));
        row(out, "String.copyOfRange.fromLenPlus1", () -> Arrays.toString(Arrays.copyOfRange(new String[]{"a", "b", "c"}, 4, 4)));
        row(out, "String.copyOfRange.fromEqLen", () -> Arrays.toString(Arrays.copyOfRange(new String[]{"a", "b", "c"}, 3, 5)));
        row(out, "String.copyOfRange.toGtLen", () -> Arrays.toString(Arrays.copyOfRange(new String[]{"a", "b", "c"}, 0, 5)));
        row(out, "String.copyOfRange.toNeg", () -> Arrays.toString(Arrays.copyOfRange(new String[]{"a", "b", "c"}, 0, -1)));
        row(out, "String.copyOfRange.bothNeg", () -> Arrays.toString(Arrays.copyOfRange(new String[]{"a", "b", "c"}, -5, -4)));
        row(out, "String.copyOfRange.negGt", () -> Arrays.toString(Arrays.copyOfRange(new String[]{"a", "b", "c"}, -1, -2)));
        row(out, "String.copyOfRange.wrapNeg", () -> Arrays.toString(Arrays.copyOfRange(new String[]{"a", "b", "c"}, -5, Integer.MAX_VALUE)));
        row(out, "String.copyOfRange.wrapMinMax", () -> Arrays.toString(Arrays.copyOfRange(new String[]{"a", "b", "c"}, Integer.MIN_VALUE, Integer.MAX_VALUE)));
        row(out, "String.copyOfRange.wrapMaxMin", () -> Arrays.toString(Arrays.copyOfRange(new String[]{"a", "b", "c"}, Integer.MAX_VALUE, Integer.MIN_VALUE)));
        row(out, "String.copyOfRange.full", () -> Arrays.toString(Arrays.copyOfRange(new String[]{"a", "b", "c"}, 0, 3)));
        row(out, "String.copyOfRange.mid", () -> Arrays.toString(Arrays.copyOfRange(new String[]{"a", "b", "c"}, 1, 2)));
        row(out, "Integer.copyOf.negLen", () -> Arrays.toString(Arrays.copyOf(new Integer[]{1, 2, 3}, -1)));
        row(out, "Integer.copyOf.minLen", () -> Arrays.toString(Arrays.copyOf(new Integer[]{1, 2, 3}, Integer.MIN_VALUE)));
        row(out, "Integer.copyOf.zero", () -> Arrays.toString(Arrays.copyOf(new Integer[]{1, 2, 3}, 0)));
        row(out, "Integer.copyOf.short", () -> Arrays.toString(Arrays.copyOf(new Integer[]{1, 2, 3}, 2)));
        row(out, "Integer.copyOf.same", () -> Arrays.toString(Arrays.copyOf(new Integer[]{1, 2, 3}, 3)));
        row(out, "Integer.copyOf.pad", () -> Arrays.toString(Arrays.copyOf(new Integer[]{1, 2, 3}, 5)));
        row(out, "Integer.copyOfRange.fromGtTo", () -> Arrays.toString(Arrays.copyOfRange(new Integer[]{1, 2, 3}, 3, 1)));
        row(out, "Integer.copyOfRange.fromEqTo", () -> Arrays.toString(Arrays.copyOfRange(new Integer[]{1, 2, 3}, 2, 2)));
        row(out, "Integer.copyOfRange.fromNeg", () -> Arrays.toString(Arrays.copyOfRange(new Integer[]{1, 2, 3}, -1, 2)));
        row(out, "Integer.copyOfRange.fromGtLen", () -> Arrays.toString(Arrays.copyOfRange(new Integer[]{1, 2, 3}, 5, 6)));
        row(out, "Integer.copyOfRange.fromLenPlus1", () -> Arrays.toString(Arrays.copyOfRange(new Integer[]{1, 2, 3}, 4, 4)));
        row(out, "Integer.copyOfRange.fromEqLen", () -> Arrays.toString(Arrays.copyOfRange(new Integer[]{1, 2, 3}, 3, 5)));
        row(out, "Integer.copyOfRange.toGtLen", () -> Arrays.toString(Arrays.copyOfRange(new Integer[]{1, 2, 3}, 0, 5)));
        row(out, "Integer.copyOfRange.toNeg", () -> Arrays.toString(Arrays.copyOfRange(new Integer[]{1, 2, 3}, 0, -1)));
        row(out, "Integer.copyOfRange.bothNeg", () -> Arrays.toString(Arrays.copyOfRange(new Integer[]{1, 2, 3}, -5, -4)));
        row(out, "Integer.copyOfRange.negGt", () -> Arrays.toString(Arrays.copyOfRange(new Integer[]{1, 2, 3}, -1, -2)));
        row(out, "Integer.copyOfRange.wrapNeg", () -> Arrays.toString(Arrays.copyOfRange(new Integer[]{1, 2, 3}, -5, Integer.MAX_VALUE)));
        row(out, "Integer.copyOfRange.wrapMinMax", () -> Arrays.toString(Arrays.copyOfRange(new Integer[]{1, 2, 3}, Integer.MIN_VALUE, Integer.MAX_VALUE)));
        row(out, "Integer.copyOfRange.wrapMaxMin", () -> Arrays.toString(Arrays.copyOfRange(new Integer[]{1, 2, 3}, Integer.MAX_VALUE, Integer.MIN_VALUE)));
        row(out, "Integer.copyOfRange.full", () -> Arrays.toString(Arrays.copyOfRange(new Integer[]{1, 2, 3}, 0, 3)));
        row(out, "Integer.copyOfRange.mid", () -> Arrays.toString(Arrays.copyOfRange(new Integer[]{1, 2, 3}, 1, 2)));
        row(out, "int[].copyOf.negLen", () -> Arrays.deepToString(Arrays.copyOf(new int[][]{{1}, {2}, {3}}, -1)));
        row(out, "int[].copyOf.minLen", () -> Arrays.deepToString(Arrays.copyOf(new int[][]{{1}, {2}, {3}}, Integer.MIN_VALUE)));
        row(out, "int[].copyOf.zero", () -> Arrays.deepToString(Arrays.copyOf(new int[][]{{1}, {2}, {3}}, 0)));
        row(out, "int[].copyOf.short", () -> Arrays.deepToString(Arrays.copyOf(new int[][]{{1}, {2}, {3}}, 2)));
        row(out, "int[].copyOf.same", () -> Arrays.deepToString(Arrays.copyOf(new int[][]{{1}, {2}, {3}}, 3)));
        row(out, "int[].copyOf.pad", () -> Arrays.deepToString(Arrays.copyOf(new int[][]{{1}, {2}, {3}}, 5)));
        row(out, "int[].copyOfRange.fromGtTo", () -> Arrays.deepToString(Arrays.copyOfRange(new int[][]{{1}, {2}, {3}}, 3, 1)));
        row(out, "int[].copyOfRange.fromEqTo", () -> Arrays.deepToString(Arrays.copyOfRange(new int[][]{{1}, {2}, {3}}, 2, 2)));
        row(out, "int[].copyOfRange.fromNeg", () -> Arrays.deepToString(Arrays.copyOfRange(new int[][]{{1}, {2}, {3}}, -1, 2)));
        row(out, "int[].copyOfRange.fromGtLen", () -> Arrays.deepToString(Arrays.copyOfRange(new int[][]{{1}, {2}, {3}}, 5, 6)));
        row(out, "int[].copyOfRange.fromLenPlus1", () -> Arrays.deepToString(Arrays.copyOfRange(new int[][]{{1}, {2}, {3}}, 4, 4)));
        row(out, "int[].copyOfRange.fromEqLen", () -> Arrays.deepToString(Arrays.copyOfRange(new int[][]{{1}, {2}, {3}}, 3, 5)));
        row(out, "int[].copyOfRange.toGtLen", () -> Arrays.deepToString(Arrays.copyOfRange(new int[][]{{1}, {2}, {3}}, 0, 5)));
        row(out, "int[].copyOfRange.toNeg", () -> Arrays.deepToString(Arrays.copyOfRange(new int[][]{{1}, {2}, {3}}, 0, -1)));
        row(out, "int[].copyOfRange.bothNeg", () -> Arrays.deepToString(Arrays.copyOfRange(new int[][]{{1}, {2}, {3}}, -5, -4)));
        row(out, "int[].copyOfRange.negGt", () -> Arrays.deepToString(Arrays.copyOfRange(new int[][]{{1}, {2}, {3}}, -1, -2)));
        row(out, "int[].copyOfRange.wrapNeg", () -> Arrays.deepToString(Arrays.copyOfRange(new int[][]{{1}, {2}, {3}}, -5, Integer.MAX_VALUE)));
        row(out, "int[].copyOfRange.wrapMinMax", () -> Arrays.deepToString(Arrays.copyOfRange(new int[][]{{1}, {2}, {3}}, Integer.MIN_VALUE, Integer.MAX_VALUE)));
        row(out, "int[].copyOfRange.wrapMaxMin", () -> Arrays.deepToString(Arrays.copyOfRange(new int[][]{{1}, {2}, {3}}, Integer.MAX_VALUE, Integer.MIN_VALUE)));
        row(out, "int[].copyOfRange.full", () -> Arrays.deepToString(Arrays.copyOfRange(new int[][]{{1}, {2}, {3}}, 0, 3)));
        row(out, "int[].copyOfRange.mid", () -> Arrays.deepToString(Arrays.copyOfRange(new int[][]{{1}, {2}, {3}}, 1, 2)));
        row(out, "class.copyOf.objToObj.negLen", () -> Arrays.toString(Arrays.copyOf(new String[]{"a", "b"}, -1, Object[].class)));
        row(out, "class.copyOf.objToStr.negLen", () -> Arrays.toString(Arrays.copyOf(new Object[]{"a"}, -1, String[].class)));
        row(out, "class.copyOf.objToInt.negLen", () -> Arrays.toString(Arrays.copyOf(new Object[]{"a"}, -1, Integer[].class)));
        row(out, "class.copyOf.objToStr.pad", () -> Arrays.toString(Arrays.copyOf(new Object[]{"a"}, 3, String[].class)));
        row(out, "class.copyOf.strToObj.short", () -> Arrays.toString(Arrays.copyOf(new String[]{"a", "b", "c"}, 2, Object[].class)));
        row(out, "class.copyOfRange.objToStr.fromGtTo", () -> Arrays.toString(Arrays.copyOfRange(new Object[]{"a"}, 3, 1, String[].class)));
        row(out, "class.copyOfRange.objToStr.fromNeg", () -> Arrays.toString(Arrays.copyOfRange(new Object[]{"a"}, -1, 1, String[].class)));
        row(out, "class.copyOfRange.objToInt.fromNeg", () -> Arrays.toString(Arrays.copyOfRange(new Object[]{"a"}, -1, 1, Integer[].class)));
        row(out, "class.copyOfRange.strToObj.fromLenPlus1", () -> Arrays.toString(Arrays.copyOfRange(new String[]{"a", "b"}, 3, 3, Object[].class)));
        row(out, "class.copyOfRange.strToObj.toGtLen", () -> Arrays.toString(Arrays.copyOfRange(new String[]{"a", "b"}, 1, 3, Object[].class)));
        row(out, "class.copyOfRange.objToStr.wrapNeg", () -> Arrays.toString(Arrays.copyOfRange(new Object[]{"a"}, -5, Integer.MAX_VALUE, String[].class)));
        row(out, "empty.int.copyOf.pad", () -> Arrays.toString(Arrays.copyOf(new int[0], 3)));
        row(out, "empty.int.copyOf.negLen", () -> Arrays.toString(Arrays.copyOf(new int[0], -1)));
        row(out, "empty.int.copyOf.zero", () -> Arrays.toString(Arrays.copyOf(new int[0], 0)));
        row(out, "empty.int.copyOfRange.fromNeg", () -> Arrays.toString(Arrays.copyOfRange(new int[0], -1, 2)));
        row(out, "empty.int.copyOfRange.fromGtLen", () -> Arrays.toString(Arrays.copyOfRange(new int[0], 1, 2)));
        row(out, "empty.int.copyOfRange.fromGtTo", () -> Arrays.toString(Arrays.copyOfRange(new int[0], 3, 1)));
        row(out, "empty.int.copyOfRange.toGtLen", () -> Arrays.toString(Arrays.copyOfRange(new int[0], 0, 2)));
        row(out, "empty.String.copyOf.pad", () -> Arrays.toString(Arrays.copyOf(new String[0], 3)));
        row(out, "empty.String.copyOf.negLen", () -> Arrays.toString(Arrays.copyOf(new String[0], -1)));
        row(out, "empty.String.copyOf.zero", () -> Arrays.toString(Arrays.copyOf(new String[0], 0)));
        row(out, "empty.String.copyOfRange.fromNeg", () -> Arrays.toString(Arrays.copyOfRange(new String[0], -1, 2)));
        row(out, "empty.String.copyOfRange.fromGtLen", () -> Arrays.toString(Arrays.copyOfRange(new String[0], 1, 2)));
        row(out, "empty.String.copyOfRange.fromGtTo", () -> Arrays.toString(Arrays.copyOfRange(new String[0], 3, 1)));
        row(out, "empty.String.copyOfRange.toGtLen", () -> Arrays.toString(Arrays.copyOfRange(new String[0], 0, 2)));
        return out.toString();
    }

    /**
     * An array the runtime holds no class for: an unmodelled call's array answer is a shared
     * empty array of its declared type. These calls stay counted.
     */
    public static String countedRows() {
        StringBuilder out = new StringBuilder();
        row(out, "fields.copyOf.negLen", () -> Arrays.toString(Arrays.copyOf(Object.class.getDeclaredFields(), -1)));
        row(out, "fields.copyOf.pad", () -> Arrays.toString(Arrays.copyOf(Object.class.getDeclaredFields(), 2)));
        row(out, "fields.copyOfRange.fromGtTo", () -> Arrays.toString(Arrays.copyOfRange(Object.class.getDeclaredFields(), 3, 1)));
        return out.toString();
    }

    public static void main(String[] args) {
        System.out.print(args.length == 0 ? rows() : countedRows());
    }
}
