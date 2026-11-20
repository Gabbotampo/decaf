import java.util.List;
import org.apache.maven.artifact.versioning.ComparableVersion;
import org.apache.maven.artifact.versioning.VersionRange;

/** Captures JarJar 0.5.3's Maven version and range operations on JDK 25. */
public final class JarJarVersionOracle {
    private static final String[][] COMPARES = {
        {"1.0-ga", "1.0"}, {"1.0-final", "1.0"}, {"1.0-cr", "1.0-rc"},
        {"1.0a", "1.0"}, {"1.0-alpha", "1.0"}, {"1.0+build", "1.0"},
        {"1.0.0", "1.0"}, {"1.0-sp", "1.0"}, {"1.0-rc1", "1.0-rc2"},
        {"2.0", "10.0"}, {"1.0.0.0", "1.0"}, {"1.0-SNAPSHOT", "1.0"}
    };
    private static final String[] RANGES = {
        "1.5", "[1,2)", "(,1]", "[1,2),[3,4)", "[1.0,2.0]"
    };

    public static void main(String[] args) throws Exception {
        for (String[] pair : COMPARES) {
            int sign = Integer.signum(new ComparableVersion(pair[0]).compareTo(new ComparableVersion(pair[1])));
            System.out.println("cmp|" + pair[0] + "|" + pair[1] + "|" + sign);
        }
        for (String spec : RANGES) {
            VersionRange range = VersionRange.createFromVersionSpec(spec);
            System.out.println("range|" + spec + "|" + range + "|" + range.getRecommendedVersion());
        }
        VersionRange left = VersionRange.createFromVersionSpec("[1,4)");
        for (String spec : new String[] {"[2,5)", "[4,6)", "1.5"}) {
            VersionRange right = VersionRange.createFromVersionSpec(spec);
            VersionRange restricted = left.restrict(right);
            System.out.println("restrict|[1,4)|" + spec + "|" + restricted + "|" + restricted.getRecommendedVersion());
        }
    }
}
