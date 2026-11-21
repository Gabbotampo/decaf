import java.util.List;
import org.apache.maven.artifact.versioning.ComparableVersion;
import org.apache.maven.artifact.versioning.VersionRange;

/** Captures JarJar 0.5.3's Maven version and range operations on JDK 25. */
public final class JarJarVersionOracle {
    private static final String[][] COMPARES = {
        {"1.0-ga", "1.0"}, {"1.0-final", "1.0"}, {"1.0-cr", "1.0-rc"},
        {"1.0a", "1.0"}, {"1.0a1", "1.0-alpha-1"}, {"1.0-alpha", "1.0"},
        {"1.0+build", "1.0"}, {"1.0+build1", "1.0+build2"},
        {"1.0+foo", "1.0.foo"}, {"1.0+build", "1.0+foo"},
        {"1.0.0", "1.0"}, {"1.0-sp", "1.0"}, {"1.0-rc1", "1.0-rc2"},
        {"2.0", "10.0"}, {"1.0.0.0", "1.0"}, {"1.0-SNAPSHOT", "1.0"},
        {"1-ga-1", "1+foo"}, {"1+foo", "1-ga-1"}, {"1+foo", "1.foo"},
        {"1-alpha-1", "1a1"}, {"1.0+foo-bar", "1.0+foo.bar"},
        {"1.0.0+foo", "1.0+foo"}, {"1.0.0-foo", "1.0-foo"},
        {"1.0-rc.1", "1.0-rc1"}, {"1.0-sp.1", "1.0-sp"},
        {"1.0-unknown", "1.0-sp"}, {"01.002", "1.2"},
        {"1.0000000000", "1.000000000"}, {"1.0000000000000000000", "1.0000000000000000001"},
        {"1..0", "1.0"}, {"1--0", "1-0"}, {"1+", "1"}
    };
    private static final String[] RANGES = {
        "1.5", "[1,2)", "[1, 2)", "(,1]", "[1,2), [3,4)", "[1.0, 2.0]",
        "2", "[1,2", "[2,1]", "[1,2]x", "1.5, [1,2)",
        "[1,2), [1.5,3)", "[1,2), [2,3)", "[1,]", "(,]", "[]"
    };
    private static final String[] EXTRA_VERSIONS = {
        "", "0", "00", "1", "1.", "1-", "1+", "1.0", "1.0.0", "1..0", "1--0",
        "1a", "1a1", "1b", "1m2", "1alpha", "1-alpha", "1-alpha-1", "1.alpha.1",
        "1+foo", "1+foo.1", "1+foo-1", "1.foo", "1.foo.bar", "1.foo.1", "1-foo.bar",
        "1-ga-1", "1-final-1", "1-cr-1", "1-rc-1", "1-snapshot", "1-sp", "1-unknown",
        "1.0.0+build", "1.0.0-rc.1", "1.0-rc1", "1.0-rc01", "1.0-rc.01", "1.0+foo-bar",
        "01.002", "1.0000000000", "1.0000000000000000000", "2147483648", "9223372036854775808"
    };

    public static void main(String[] args) throws Exception {
        for (String[] pair : COMPARES) {
            int sign = Integer.signum(new ComparableVersion(pair[0]).compareTo(new ComparableVersion(pair[1])));
            System.out.println("cmp|" + pair[0] + "|" + pair[1] + "|" + sign);
        }
        for (int i = 0; i < EXTRA_VERSIONS.length; i++) {
            for (int j = i + 1; j < EXTRA_VERSIONS.length; j++) {
                int sign = Integer.signum(new ComparableVersion(EXTRA_VERSIONS[i]).compareTo(new ComparableVersion(EXTRA_VERSIONS[j])));
                System.out.println("cmp|" + EXTRA_VERSIONS[i] + "|" + EXTRA_VERSIONS[j] + "|" + sign);
            }
        }
        for (String spec : RANGES) {
            try {
                VersionRange range = VersionRange.createFromVersionSpec(spec);
                System.out.println("range|" + spec + "|" + range + "|" + range.getRecommendedVersion());
            } catch (Exception error) {
                System.out.println("range-error|" + spec + "|" + error.getClass().getName() + "|" + error.getMessage());
            }
        }
        VersionRange left = VersionRange.createFromVersionSpec("[1,4)");
        for (String spec : new String[] {"[2,5)", "[4,6)", "1.5", "2"}) {
            VersionRange right = VersionRange.createFromVersionSpec(spec);
            VersionRange restricted = left.restrict(right);
            System.out.println("restrict|[1,4)|" + spec + "|" + restricted + "|" + restricted.getRecommendedVersion());
        }
        VersionRange restricted = VersionRange.createFromVersionSpec("2")
            .restrict(VersionRange.createFromVersionSpec("[1,2)"));
        System.out.println("restrict|2|[1,2)|" + restricted + "|" + restricted.getRecommendedVersion()
            + "|" + restricted.containsVersion(new org.apache.maven.artifact.versioning.DefaultArtifactVersion("2"))
            + "|" + restricted.containsVersion(new org.apache.maven.artifact.versioning.DefaultArtifactVersion("1.5")));
    }
}
