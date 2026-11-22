package oracle;

import net.fabricmc.loader.api.SemanticVersion;
import net.fabricmc.loader.api.Version;
import net.fabricmc.loader.api.VersionParsingException;

/**
 * Java side only (never in the Decaf jar): for each version text, `Version.parse` and
 * `SemanticVersion.parse` on the real Fabric Loader, then `demo.Probe.describe` on what
 * `Version.parse` returned.
 */
public final class OracleMain {
    private static final String[] CASES = {
        "26.3+v260915", "1.0.0", "1.0.0-beta.2+build.5", "1.2", "1.0.0-SNAPSHOT", "1.0", "1",
        "v1.0.0", "1.0.0.0", "26.3", "1.20.1-rc.1", "abc", "1.21.1", "25", "",
    };

    public static void main(String[] args) {
        for (String text : CASES) {
            String shown = "\"" + text + "\"";
            try {
                System.out.println("Version.parse(" + shown + ") -> " + demo.Probe.describe(Version.parse(text)));
            } catch (VersionParsingException e) {
                System.out.println("Version.parse(" + shown + ") -> throws " + e.getClass().getName() + ": " + e.getMessage());
            }
            try {
                SemanticVersion sv = SemanticVersion.parse(text);
                System.out.println("SemanticVersion.parse(" + shown + ") -> " + demo.Probe.describe(sv));
            } catch (VersionParsingException e) {
                System.out.println("SemanticVersion.parse(" + shown + ") -> throws " + e.getClass().getName() + ": " + e.getMessage());
            }
        }
    }
}
