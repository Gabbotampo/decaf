package demo;

import com.google.gson.FieldNamingPolicy;
import com.google.gson.Gson;
import com.google.gson.GsonBuilder;
import com.google.gson.annotations.SerializedName;
import java.lang.reflect.Modifier;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

/**
 * Deep Java recursion and Gson's enum writer (the client census's Sodium abort).
 *
 * <p>Sodium's {@code SodiumOptions.writeToDisk} writes its options through a {@code Gson} built
 * with {@code excludeFieldsWithModifiers(PRIVATE)}, which writes static fields. Reflecting over an
 * enum constant wrote its constants' {@code public static} fields, each the enum again, until the
 * process aborted; Gson writes an enum by name.
 */
public final class Probe {
    static int depth;

    static int down(int n) {
        depth++;
        return n == 0 ? 0 : 1 + down(n - 1);
    }

    /** A recursion 1,000,000 frames deep, which no default JVM stack holds, caught; then more work. */
    public static String deepRecursion() {
        depth = 0;
        try {
            return "ok " + down(1_000_000);
        } catch (StackOverflowError e) {
            return "SOE " + (depth > 1) + " " + down(2);
        }
    }

    /** A recursion every stack holds answers. */
    public static String shallowRecursion() {
        return String.valueOf(down(3));
    }

    enum DeferMode {
        ALWAYS, ONE_FRAME_AHEAD, ZERO_FRAMES;
        public static DeferMode fallback = ALWAYS;
    }

    enum Split {
        OFF {
            @Override
            public String toString() {
                return "off";
            }
        },
        SAFE,
        UNLIMITED
    }

    enum Tagged {
        @SerializedName("first") A,
        @SerializedName(value = "second", alternate = {"b2"}) B,
        C
    }

    public static class Quality {
        public boolean hiddenFluidCulling = true;
        public Split splitMode = Split.OFF;
        public Tagged tagged = Tagged.B;
    }

    public static class Performance {
        public int chunkBuilderThreads = 0;
        public DeferMode chunkBuildDeferMode = DeferMode.ONE_FRAME_AHEAD;
        public DeferMode unset = null;
    }

    /** Sodium's {@code SodiumOptions}, in shape. */
    public static class Options {
        public final Quality quality = new Quality();
        public final Performance performance = new Performance();
        private boolean readOnly;
        private static final Gson GSON = new GsonBuilder()
            .setFieldNamingPolicy(FieldNamingPolicy.LOWER_CASE_WITH_UNDERSCORES)
            .setPrettyPrinting()
            .excludeFieldsWithModifiers(Modifier.PRIVATE)
            .create();

        static String write() {
            return GSON.toJson(new Options());
        }
    }

    public static String sodiumOptions() {
        return Options.write().replace("\n", "\\n");
    }

    public static String enumValues() {
        Gson gson = new GsonBuilder().create();
        return gson.toJson(DeferMode.ZERO_FRAMES) + "|" + gson.toJson(Split.OFF) + "|"
            + gson.toJson(Tagged.A) + "|" + gson.toJson(Tagged.B) + "|" + gson.toJson(Tagged.C) + "|"
            + gson.toJson(List.of(DeferMode.ALWAYS, Split.SAFE, Tagged.A));
    }

    public static String enumMapKeys() {
        Gson gson = new GsonBuilder().create();
        Map<Tagged, Integer> tagged = new LinkedHashMap<>();
        tagged.put(Tagged.B, 1);
        Map<Split, Integer> split = new LinkedHashMap<>();
        split.put(Split.OFF, 1);
        return gson.toJson(tagged) + "|" + gson.toJson(split);
    }

    public static class Renamed {
        @SerializedName("n")
        public int number = 3;
        @SerializedName(value = "t", alternate = {"text2", "t3"})
        public String text = "x";
        public int plainField = 1;
        public char letter = 'q';
        public boolean off = false;
    }

    public static String annotatedFields() {
        Gson gson = new GsonBuilder()
            .setFieldNamingPolicy(FieldNamingPolicy.LOWER_CASE_WITH_UNDERSCORES)
            .create();
        Renamed read = gson.fromJson("{\"n\":5,\"t3\":\"y\",\"plain_field\":2}", Renamed.class);
        return gson.toJson(new Renamed()) + "|" + read.number + "|" + read.text + "|" + read.plainField;
    }

    /** Two of one field's names in a document: Gson keeps the last. */
    public static String ambiguousAlternates() {
        Renamed read = new GsonBuilder().create().fromJson("{\"t\":\"a\",\"text2\":\"b\"}", Renamed.class);
        return read == null ? "null" : read.text;
    }

    public static void main(String[] args) {
        System.out.println("deepRecursion=" + deepRecursion());
        System.out.println("shallowRecursion=" + shallowRecursion());
        System.out.println("sodiumOptions=" + sodiumOptions());
        System.out.println("enumValues=" + enumValues());
        System.out.println("enumMapKeys=" + enumMapKeys());
        System.out.println("annotatedFields=" + annotatedFields());
        System.out.println("ambiguousAlternates=" + ambiguousAlternates());
    }
}
