// Oracle probe for java.util.Objects null checks and the checkIndex family on
// JDK 25. `main` prints one `<label>=<value>` line per `case_*` method; a case
// that throws prints `<simple class name>:<message>` (`null` message as such).
package demo;

import java.util.Objects;
import java.util.function.Supplier;

public final class Probe {
    static final String KEPT = "kept";
    static final String MSG = "msg";

    public static String case_rn_kept() {
        return Objects.requireNonNull(KEPT);
    }

    public static String case_rn_null() {
        return thrown(() -> Objects.requireNonNull(null));
    }

    public static String case_rn_msg_kept() {
        return Objects.requireNonNull(KEPT, MSG);
    }

    public static String case_rn_msg_null() {
        return thrown(() -> Objects.requireNonNull(null, MSG));
    }

    public static String case_rn_msg_null_message() {
        return thrown(() -> Objects.requireNonNull(null, (String) null));
    }

    public static String case_rn_supplier_kept() {
        return Objects.requireNonNull(KEPT, () -> MSG);
    }

    public static String case_rn_supplier_kept_null_supplier() {
        return Objects.requireNonNull(KEPT, (Supplier<String>) null);
    }

    public static String case_rn_supplier_null() {
        return thrown(() -> Objects.requireNonNull(null, () -> MSG));
    }

    public static String case_rn_supplier_null_return() {
        return thrown(() -> Objects.requireNonNull(null, () -> (String) null));
    }

    public static String case_rn_supplier_null_supplier() {
        return thrown(() -> Objects.requireNonNull(null, (Supplier<String>) null));
    }

    public static String case_rne_kept_fallback() {
        return Objects.requireNonNullElse(KEPT, MSG);
    }

    public static String case_rne_null_fallback() {
        return Objects.requireNonNullElse(null, MSG);
    }

    public static String case_rne_kept_null_fallback() {
        return Objects.requireNonNullElse(KEPT, null);
    }

    public static String case_rne_null_null() {
        return thrown(() -> Objects.requireNonNullElse(null, null));
    }

    public static String case_rneg_kept_supplier() {
        return Objects.requireNonNullElseGet(KEPT, () -> MSG);
    }

    public static String case_rneg_kept_null_supplier() {
        return Objects.requireNonNullElseGet(KEPT, (Supplier<String>) null);
    }

    public static String case_rneg_null_supplier() {
        return Objects.requireNonNullElseGet(null, () -> MSG);
    }

    public static String case_rneg_null_null_return() {
        return thrown(() -> Objects.requireNonNullElseGet(null, () -> (Object) null));
    }

    public static String case_rneg_null_null_supplier() {
        return thrown(() -> Objects.requireNonNullElseGet(null, (Supplier<Object>) null));
    }

    public static String case_ci_ok() {
        return String.valueOf(Objects.checkIndex(5, 10));
    }

    public static String case_ci_zero_ok() {
        return String.valueOf(Objects.checkIndex(0, 1));
    }

    public static String case_ci_negative() {
        return thrown(() -> Objects.checkIndex(-1, 10));
    }

    public static String case_ci_at_length() {
        return thrown(() -> Objects.checkIndex(10, 10));
    }

    public static String case_ci_zero_length() {
        return thrown(() -> Objects.checkIndex(0, 0));
    }

    public static String case_ci_negative_length() {
        return thrown(() -> Objects.checkIndex(0, -1));
    }

    public static String case_ci_min_value() {
        return thrown(() -> Objects.checkIndex(Integer.MIN_VALUE, Integer.MAX_VALUE));
    }

    public static String case_ci_max_value() {
        return thrown(() -> Objects.checkIndex(Integer.MAX_VALUE, Integer.MAX_VALUE));
    }

    public static String case_ci_long_ok() {
        return String.valueOf(Objects.checkIndex(5L, 10L));
    }

    public static String case_ci_long_negative() {
        return thrown(() -> Objects.checkIndex(-1L, 10L));
    }

    public static String case_ci_long_at_length() {
        return thrown(() -> Objects.checkIndex(10L, 10L));
    }

    public static String case_ci_long_big() {
        return thrown(() -> Objects.checkIndex(Long.MAX_VALUE, Long.MAX_VALUE));
    }

    public static String case_ci_long_min() {
        return thrown(() -> Objects.checkIndex(Long.MIN_VALUE, Long.MAX_VALUE));
    }

    public static String case_cfti_ok() {
        return String.valueOf(Objects.checkFromToIndex(0, 5, 10));
    }

    public static String case_cfti_empty_ok() {
        return String.valueOf(Objects.checkFromToIndex(3, 3, 10));
    }

    public static String case_cfti_full_ok() {
        return String.valueOf(Objects.checkFromToIndex(0, 10, 10));
    }

    public static String case_cfti_negative_from() {
        return thrown(() -> Objects.checkFromToIndex(-1, 5, 10));
    }

    public static String case_cfti_too_long() {
        return thrown(() -> Objects.checkFromToIndex(0, 11, 10));
    }

    public static String case_cfti_reversed() {
        return thrown(() -> Objects.checkFromToIndex(6, 5, 10));
    }

    public static String case_cfti_negative_to() {
        return thrown(() -> Objects.checkFromToIndex(0, -1, 10));
    }

    public static String case_cfti_negative_length() {
        return thrown(() -> Objects.checkFromToIndex(0, 0, -1));
    }

    public static String case_cfti_min_value() {
        return thrown(() -> Objects.checkFromToIndex(Integer.MIN_VALUE, Integer.MIN_VALUE, Integer.MAX_VALUE));
    }

    public static String case_cfti_long_ok() {
        return String.valueOf(Objects.checkFromToIndex(2L, 5L, 10L));
    }

    public static String case_cfti_long_too_long() {
        return thrown(() -> Objects.checkFromToIndex(0L, 11L, 10L));
    }

    public static String case_cfti_long_negative_from() {
        return thrown(() -> Objects.checkFromToIndex(-1L, 5L, 10L));
    }

    public static String case_cfti_long_reversed() {
        return thrown(() -> Objects.checkFromToIndex(6L, 5L, 10L));
    }

    public static String case_cfis_ok() {
        return String.valueOf(Objects.checkFromIndexSize(5, 5, 10));
    }

    public static String case_cfis_empty_at_end_ok() {
        return String.valueOf(Objects.checkFromIndexSize(10, 0, 10));
    }

    public static String case_cfis_past_end() {
        return thrown(() -> Objects.checkFromIndexSize(5, 6, 10));
    }

    public static String case_cfis_negative_from() {
        return thrown(() -> Objects.checkFromIndexSize(-1, 5, 10));
    }

    public static String case_cfis_negative_size() {
        return thrown(() -> Objects.checkFromIndexSize(2, -1, 10));
    }

    public static String case_cfis_negative_length() {
        return thrown(() -> Objects.checkFromIndexSize(0, 0, -1));
    }

    public static String case_cfis_overflowing_sum() {
        return thrown(() -> Objects.checkFromIndexSize(2, Integer.MAX_VALUE, 10));
    }

    public static String case_cfis_max_from() {
        return thrown(() -> Objects.checkFromIndexSize(Integer.MAX_VALUE, 1, Integer.MAX_VALUE));
    }

    public static String case_cfis_long_ok() {
        return String.valueOf(Objects.checkFromIndexSize(5L, 5L, 10L));
    }

    public static String case_cfis_long_past_end() {
        return thrown(() -> Objects.checkFromIndexSize(5L, 6L, 10L));
    }

    public static String case_cfis_long_negative_size() {
        return thrown(() -> Objects.checkFromIndexSize(2L, -1L, 10L));
    }

    public static String case_cfis_long_overflowing_sum() {
        return thrown(() -> Objects.checkFromIndexSize(2L, Long.MAX_VALUE, 10L));
    }

    private static String thrown(Throwing action) {
        try {
            action.run();
            return "no-throw";
        } catch (Throwable e) {
            return e.getClass().getSimpleName() + ":" + String.valueOf(e.getMessage());
        }
    }

    @FunctionalInterface
    interface Throwing {
        void run();
    }

    public static void main(String[] args) throws Exception {
        println("case_rn_kept");
        println("case_rn_null");
        println("case_rn_msg_kept");
        println("case_rn_msg_null");
        println("case_rn_msg_null_message");
        println("case_rn_supplier_kept");
        println("case_rn_supplier_kept_null_supplier");
        println("case_rn_supplier_null");
        println("case_rn_supplier_null_return");
        println("case_rn_supplier_null_supplier");
        println("case_rne_kept_fallback");
        println("case_rne_null_fallback");
        println("case_rne_kept_null_fallback");
        println("case_rne_null_null");
        println("case_rneg_kept_supplier");
        println("case_rneg_kept_null_supplier");
        println("case_rneg_null_supplier");
        println("case_rneg_null_null_return");
        println("case_rneg_null_null_supplier");
        println("case_ci_ok");
        println("case_ci_zero_ok");
        println("case_ci_negative");
        println("case_ci_at_length");
        println("case_ci_zero_length");
        println("case_ci_negative_length");
        println("case_ci_min_value");
        println("case_ci_max_value");
        println("case_ci_long_ok");
        println("case_ci_long_negative");
        println("case_ci_long_at_length");
        println("case_ci_long_big");
        println("case_ci_long_min");
        println("case_cfti_ok");
        println("case_cfti_empty_ok");
        println("case_cfti_full_ok");
        println("case_cfti_negative_from");
        println("case_cfti_too_long");
        println("case_cfti_reversed");
        println("case_cfti_negative_to");
        println("case_cfti_negative_length");
        println("case_cfti_min_value");
        println("case_cfti_long_ok");
        println("case_cfti_long_too_long");
        println("case_cfti_long_negative_from");
        println("case_cfti_long_reversed");
        println("case_cfis_ok");
        println("case_cfis_empty_at_end_ok");
        println("case_cfis_past_end");
        println("case_cfis_negative_from");
        println("case_cfis_negative_size");
        println("case_cfis_negative_length");
        println("case_cfis_overflowing_sum");
        println("case_cfis_max_from");
        println("case_cfis_long_ok");
        println("case_cfis_long_past_end");
        println("case_cfis_long_negative_size");
        println("case_cfis_long_overflowing_sum");
    }

    static void println(String label) throws Exception {
        System.out.println(label + "=" + Probe.class.getMethod(label).invoke(null));
    }
}
