import java.io.BufferedReader;
import java.io.InputStreamReader;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

import net.fabricmc.loader.api.Version;
import net.fabricmc.loader.api.metadata.version.VersionPredicate;

/** Batch oracle for Fabric Loader 0.19.5 dependency predicates. */
public final class FabricPredicateOracle {
    private static final Pattern FIELD = Pattern.compile("\\\"(predicate|version)\\\"\\s*:\\s*\\\"((?:\\\\.|[^\\\"])*)\\\"");

    /** Decodes the JSON string escapes this oracle's input and output use. */
    private static String unescape(String value) {
        StringBuilder out = new StringBuilder();
        for (int i = 0; i < value.length(); i++) {
            char c = value.charAt(i);
            if (c != '\\' || i + 1 == value.length()) {
                out.append(c);
                continue;
            }
            char e = value.charAt(++i);
            switch (e) {
                case 'n' -> out.append('\n');
                case 't' -> out.append('\t');
                case 'r' -> out.append('\r');
                case 'b' -> out.append('\b');
                case 'f' -> out.append('\f');
                case 'u' -> {
                    out.append((char) Integer.parseInt(value.substring(i + 1, i + 5), 16));
                    i += 4;
                }
                default -> out.append(e);
            }
        }
        return out.toString();
    }

    /** JSON-quotes a string, escaping every control character so rows stay valid JSON. */
    private static String quote(String value) {
        StringBuilder out = new StringBuilder("\"");
        for (char c : value.toCharArray()) {
            if (c == '\\' || c == '"') out.append('\\').append(c);
            else if (c < 0x20) out.append(String.format("\\u%04x", (int) c));
            else out.append(c);
        }
        return out.append('"').toString();
    }

    public static void main(String[] args) throws Exception {
        if (args.length != 1) {
            throw new IllegalArgumentException("usage: FabricPredicateOracle <fabric-loader-0.19.5.jar>");
        }
        try (BufferedReader input = new BufferedReader(new InputStreamReader(System.in))) {
            String line;
            while ((line = input.readLine()) != null) {
                Matcher fields = FIELD.matcher(line);
                String predicate = null;
                String version = null;
                while (fields.find()) {
                    if (fields.group(1).equals("predicate")) predicate = unescape(fields.group(2));
                    else version = unescape(fields.group(2));
                }
                if (predicate == null || version == null) {
                    System.out.println("{\"error\":\"invalid input\"}");
                    continue;
                }
                try {
                    boolean result = VersionPredicate.parse(predicate).test(Version.parse(version));
                    System.out.println("{\"predicate\":" + quote(predicate)
                            + ",\"version\":" + quote(version)
                            + ",\"ok\":" + result + "}");
                } catch (Throwable error) {
                    System.out.println("{\"predicate\":" + quote(predicate)
                            + ",\"version\":" + quote(version)
                            + ",\"error\":" + quote(error.toString()) + "}");
                }
            }
        }
    }
}
