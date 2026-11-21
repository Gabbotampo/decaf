import java.io.IOException;
import java.nio.file.FileSystem;
import java.nio.file.FileSystems;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.Collections;
import java.util.Comparator;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.regex.Pattern;
import java.util.stream.Stream;
import java.util.zip.ZipEntry;
import java.util.zip.ZipOutputStream;

/**
 * Pure JDK 25 ZipFileSystem probe matching Fabric's {@code ModNioPackResources.readNamespaces}
 * ({@code FileSystems.newFileSystem(zip)} + {@code Files.isDirectory(root.resolve("data"))} +
 * {@code Files.list(dir)} + {@code Files.isDirectory(child)} + {@code [a-z0-9-_.]+}).
 *
 * <p>Captures {@code tests/fixtures/recipes/jdk25_zipfs_namespaces.txt} without any game files.
 */
public final class ZipFsNamespacesOracle {
    private static final Pattern RESOURCE_PACK_PATH = Pattern.compile("[a-z0-9-_.]+");

    private static Path writeZip(Path dir, String name, String... entries) throws IOException {
        Path zipPath = dir.resolve(name + ".zip");
        try (ZipOutputStream zos = new ZipOutputStream(Files.newOutputStream(zipPath))) {
            for (String entry : entries) {
                zos.putNextEntry(new ZipEntry(entry));
                if (!entry.endsWith("/")) {
                    zos.write("x".getBytes());
                }
                zos.closeEntry();
            }
        }
        return zipPath;
    }

    private static String probe(Path zipPath) {
        try (FileSystem fs = FileSystems.newFileSystem(zipPath, (ClassLoader) null)) {
            Path dir = fs.getPath("/").resolve("data");
            boolean dataIsDir = Files.isDirectory(dir);
            if (!dataIsDir) {
                return "open=OK|dataIsDir=false|namespaces=[]";
            }
            List<String> valid = new ArrayList<>();
            try (Stream<Path> stream = Files.list(dir)) {
                for (Path child : stream.toList()) {
                    if (!Files.isDirectory(child)) {
                        continue;
                    }
                    String raw = child.getFileName().toString();
                    String ns = raw.endsWith("/") ? raw.substring(0, raw.length() - 1) : raw;
                    if (RESOURCE_PACK_PATH.matcher(ns).matches()) {
                        valid.add(ns);
                    }
                }
            }
            Collections.sort(valid);
            return "open=OK|dataIsDir=true|namespaces=" + valid;
        } catch (Throwable t) {
            return "open=THROW|" + t.getClass().getSimpleName();
        }
    }

    public static void main(String[] args) throws Exception {
        Path tmp = Files.createTempDirectory("jdk25-zipfs-namespaces-");
        try {
            Map<String, String[]> cases = new LinkedHashMap<>();
            cases.put("normal_file", new String[] {"data/ns/recipe/a.json"});
            cases.put("normal_empty_dir", new String[] {"data/", "data/empty_ns/"});
            cases.put("normal_empty_dir_no_data", new String[] {"data/empty_ns/"});
            cases.put("valid_dotted_ns", new String[] {"data/a.b/recipe/a.json", "data/c..d/"});
            cases.put("bare_data_file_only", new String[] {"data"});
            cases.put("bare_file_under_data", new String[] {"data/", "data/plain_file.txt"});
            cases.put("invalid_uppercase_dir", new String[] {"data/", "data/Bad_NS/"});
            cases.put("data_file_then_child", new String[] {"data", "data/ns/recipe/a.json"});
            cases.put("child_then_data_file", new String[] {"data/ns/recipe/a.json", "data"});
            cases.put("ns_file_then_child", new String[] {"data/ns", "data/ns/recipe/a.json"});
            cases.put("child_then_ns_file", new String[] {"data/ns/recipe/a.json", "data/ns"});
            cases.put(
                    "ns_file_plus_other_ns",
                    new String[] {"data/ns", "data/ns/recipe/a.json", "data/other/recipe/b.json"});
            cases.put("dot_in_data_empty_dir", new String[] {"data/./empty_ns/"});
            cases.put("dot_in_data_file", new String[] {"data/./ns/recipe/a.json"});
            cases.put("dot_outside_data", new String[] {"foo/./bar.txt", "data/ns/recipe/a.json"});
            cases.put("dotdot_in_data", new String[] {"data/ns/../other/recipe/a.json"});
            cases.put("dotdot_outside_data", new String[] {"foo/../bar.txt", "data/ns/recipe/a.json"});
            cases.put("dot_literal_dir", new String[] {"data/./"});
            cases.put("dotdot_literal_dir", new String[] {"data/../"});

            StringBuilder out = new StringBuilder();
            for (var entry : cases.entrySet()) {
                Path zip = writeZip(tmp, entry.getKey(), entry.getValue());
                out.append(entry.getKey()).append('=').append(probe(zip)).append('\n');
            }
            if (args.length > 0) {
                Files.writeString(Path.of(args[0]), out.toString());
            } else {
                System.out.print(out);
            }
        } finally {
            try (Stream<Path> walk = Files.walk(tmp)) {
                walk.sorted(Comparator.reverseOrder()).forEach(path -> {
                    try {
                        Files.deleteIfExists(path);
                    } catch (IOException ignored) {
                    }
                });
            }
        }
    }
}
