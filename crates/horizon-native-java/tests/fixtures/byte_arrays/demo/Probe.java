package demo;

import java.io.ByteArrayInputStream;
import java.nio.charset.StandardCharsets;
import java.util.Arrays;

/**
 * {@code byte[]} end to end (#274): the runtime keeps one byte per element. Loads sign-extend,
 * stores keep the low 8 bits, and every path a mod uses on its byte arrays answers as Java 25.
 */
public final class Probe {
    static String t(java.util.function.Supplier<Object> s) {
        try {
            return String.valueOf(s.get());
        } catch (RuntimeException e) {
            return e.getClass().getSimpleName() + ":" + e.getMessage();
        }
    }

    /** `baload` sign-extends; `arraylength`; `Arrays.toString`. */
    public static String basics() {
        byte[] b = new byte[5];
        b[0] = (byte) 200;
        b[1] = (byte) -1;
        b[2] = 127;
        b[3] += 300;
        int x = 0x1234;
        b[4] = (byte) x;
        return Arrays.toString(b) + "|" + b.length + "|" + (b[0] & 0xff) + "|" + t(() -> b[5]);
    }

    /** `System.arraycopy` within one array, both directions (as if through a temporary). */
    public static String overlap() {
        byte[] a = {0, 1, 2, 3, 4, 5, 6, 7, 8, 9};
        System.arraycopy(a, 0, a, 2, 5);
        String forward = Arrays.toString(a);
        byte[] c = {0, 1, 2, 3, 4, 5, 6, 7, 8, 9};
        System.arraycopy(c, 3, c, 1, 4);
        return forward + "|" + Arrays.toString(c);
    }

    /** `System.arraycopy` refusals: kinds, `null`, bounds. */
    public static String copyErrors() {
        byte[] b = {1, 2, 3};
        int[] i = new int[3];
        Object[] o = {1};
        return t(() -> { System.arraycopy(b, 0, i, 0, 1); return "ok"; })
            + "|" + t(() -> { System.arraycopy(o, 0, b, 0, 1); return "ok"; })
            + "|" + t(() -> { System.arraycopy(null, 0, b, 0, 1); return "ok"; })
            + "|" + t(() -> { System.arraycopy(b, 2, b, 0, 2); return "ok"; })
            + "|" + t(() -> { System.arraycopy(b, 0, b, -1, 1); return "ok"; })
            + "|" + t(() -> { System.arraycopy(b, 0, new byte[2], 0, 2); return "ok"; });
    }

    /** `Arrays.copyOf`/`copyOfRange`, `fill`, `equals`, `hashCode`, `clone`. */
    public static String arraysOps() {
        byte[] b = {(byte) 200, 2, 3};
        byte[] padded = Arrays.copyOf(b, 5);
        byte[] range = Arrays.copyOfRange(b, 1, 3);
        byte[] c = b.clone();
        c[0] = 7;
        byte[] f = new byte[4];
        Arrays.fill(f, (byte) -3);
        return Arrays.toString(padded) + "|" + Arrays.toString(range) + "|" + b[0] + "," + c[0]
            + "|" + Arrays.toString(f) + "|" + Arrays.equals(b, b.clone()) + Arrays.equals(b, c)
            + "|" + Arrays.hashCode(b) + "|" + Arrays.hashCode(f);
    }

    /** EntityCulling's `ArrayOcclusionCache`: a 4 MiB `byte[]`, written and read by index. */
    public static String big() {
        byte[] cache = new byte[4 << 20];
        for (int i = 0; i < cache.length; i = i + 4096) {
            cache[i] = (byte) (i >>> 12);
        }
        long sum = 0;
        for (int i = 0; i < cache.length; i = i + 4096) {
            sum += cache[i];
        }
        byte[] half = new byte[2 << 20];
        System.arraycopy(cache, 0, half, 0, half.length);
        Arrays.fill(cache, (byte) 1);
        return cache.length + "|" + sum + "|" + half[4096 * 200] + "|" + cache[123456];
    }

    /** Text and streams over a `byte[]`. */
    public static String text() throws Exception {
        byte[] utf8 = "héllo".getBytes(StandardCharsets.UTF_8);
        byte[] out = new byte[4];
        int read = new ByteArrayInputStream(utf8).read(out, 1, 3);
        return utf8.length + "|" + new String(utf8, StandardCharsets.UTF_8) + "|" + read + "|"
            + Arrays.toString(out);
    }

    public static void main(String[] args) throws Exception {
        for (String m : new String[] {"basics", "overlap", "copyErrors", "arraysOps", "big", "text"}) {
            System.out.println(m + "=" + Probe.class.getMethod(m).invoke(null));
        }
    }
}
