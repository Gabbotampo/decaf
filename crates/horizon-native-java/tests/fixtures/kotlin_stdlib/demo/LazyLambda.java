package demo;

import java.util.*;
import java.util.function.Supplier;
import kotlin.Lazy;
import kotlin.LazyKt;

/**
 * A Kotlin lazy holder built from a lambda (a callback handle since #261) that
 * memoizes another lambda: what the memoized lambda captured must stay
 * reachable through the holder across a collection.
 */
public final class LazyLambda {
    private static Lazy<Supplier<String>> kept;

    public static String lazyLambdaHold() {
        // The list is made inside the initializer, so only the memoized
        // lambda holds it; the 5,000-char capture before it puts it past the
        // bound of the lambda's key text, so only the lambda's row names it.
        kept = LazyKt.lazy(() -> {
            String pad = "p".repeat(5000);
            List<String> names = new ArrayList<>(List.of("held"));
            return (Supplier<String>) () -> pad.length() + names.get(0) + names.size();
        });
        return kept.getValue().get() + "|" + kept.isInitialized();
    }

    public static String lazyLambdaUse() {
        for (int i = 0; i < 70000; i++) { Object[] trash = new Object[1]; }
        return kept.getValue().get() + "|" + kept.isInitialized();
    }
}
