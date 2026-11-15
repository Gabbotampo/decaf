import java.io.BufferedReader;
import java.io.InputStreamReader;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

import net.fabricmc.loader.api.SemanticVersion;
import net.fabricmc.loader.api.metadata.version.VersionPredicate;

/** Batch oracle for Fabric Loader 0.19.5 dependency predicates. */
public final class FabricPredicateOracle {
    private static final Pattern FIELD = Pattern.compile("\\\"(predicate|version)\\\"\\s*:\\s*\\\"((?:\\\\.|[^\\\"])*)\\\"");

    private static String unescape(String value) {
        return value.replace("\\\\", "\\").replace("\\\"", "\"").replace("\\n", "\n");
    }

    private static String quote(String value) {
        return "\\\"" + value.replace("\\", "\\\\").replace("\"", "\\\"").replace("\n", "\\n") + "\\\"";
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
                    boolean result = VersionPredicate.parse(predicate).test(SemanticVersion.parse(version));
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
