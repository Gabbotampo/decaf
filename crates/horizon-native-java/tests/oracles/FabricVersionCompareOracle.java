import net.fabricmc.loader.api.Version;

/** Captures Fabric Loader 0.19.5's Version.parse/compareTo contract. */
public final class FabricVersionCompareOracle {
    private static final String[][] CASES = {
        {"1.0-alpha", "1.0"}, {"1.0+build1", "1.0+build2"},
        {"1.0.0", "1.0"}, {"1.0-alpha.1", "1.0-alpha.beta"},
        {"1.0-alpha.2", "1.0-alpha.10"}, {"1.0-rc", "1.0-beta"},
        {"1.0-rc.1", "1.0-rc.1"}, {"1.0.1", "1.0"},
        {"1.0.0.0", "1"}, {"1.0-01", "1.0-1"},
        {"01.0", "1.0"}, {"1.0.foo", "1.0"},
        {"x", "1.0"}, {"1.0-X", "1.0"},
        {"1.0-SNAPSHOT", "1.0-rc"}, {"1.0-a.2", "1.0-a.10"},
        {"2.0", "10.0"}, {"1.0.0+one", "1.0.0+two"},
        {"1.x", "1.1.1"}, {"1.X", "1.1.1"}, {"1.*", "1.1.1"},
        {"1.x", "1.x"}, {"1.0-final", "1.0"}, {"1.0-release", "1.0"},
        {"1.0-ga", "1.0"}, {"1.0-cr", "1.0-rc"}, {"1.0a", "1.0"},
        {"1.0-", "1.0"}, {"1.0..1", "1.0"}, {"1.0+build", "1.0"},
    };

    public static void main(String[] args) throws Exception {
        for (String[] pair : CASES) {
            Version left = Version.parse(pair[0]);
            Version right = Version.parse(pair[1]);
            System.out.println(pair[0] + "|" + pair[1] + "|" + Integer.signum(left.compareTo(right))
                    + "|" + left.getClass().getSimpleName() + "|" + right.getClass().getSimpleName());
        }
    }
}
