import java.lang.reflect.*;
import java.io.*;
import java.nio.file.*;
import java.util.*;
import java.util.zip.*;
import net.fabricmc.loader.api.Version;

/** Captures Fabric Loader 0.19.5's complete candidate resolver on Java 25. */
public final class ModPrioSorterOracle {
  static String environment = "SERVER";
  record Dep(String id, String range) {}
  record Spec(String label, String id, String version, boolean root, List<String> provides, String parent,
              List<Dep> dependencies) {}

  /*
   * These are inert LoaderModMetadata proxies. The resolver only needs the
   * metadata's identity, version, provides, schema version, environment and
   * dependency collection for these candidate shapes; every other method is
   * an empty metadata surface. They contain no candidate-selection logic.
   */
  static Object metadata(Spec spec) {
    try {
      Class<?> type = Class.forName("net.fabricmc.loader.impl.metadata.LoaderModMetadata");
      return Proxy.newProxyInstance(
          ModPrioSorterOracle.class.getClassLoader(), new Class<?>[] {type}, (proxy, method, args) -> {
            return switch (method.getName()) {
              case "getId" -> spec.id();
              case "getVersion" -> Version.parse(spec.version());
              case "getProvides" -> spec.provides();
              case "getType" -> "normal";
              case "getDependencies" -> spec.dependencies().stream().map(ModPrioSorterOracle::dependency).toList();
              case "getEnvironment" -> Enum.valueOf((Class) Class.forName("net.fabricmc.loader.api.metadata.ModEnvironment"), "UNIVERSAL");
              case "getSchemaVersion" -> 1;
              case "getMixinConfigs", "getOldInitializers", "getEntrypointKeys", "getJars", "getEntrypoints" -> List.of();
              case "loadsInEnvironment" -> true;
              case "getLanguageAdapterDefinitions", "getCustomValues" -> Map.of();
              case "getOldStyleLanguageAdapter", "getClassTweaker", "getName", "getDescription", "getContact" -> null;
              case "getAuthors", "getContributors", "getLicense" -> List.of();
              case "getIconPath" -> Optional.empty();
              case "containsCustomValue", "containsCustomElement" -> false;
              case "getCustomValue" -> null;
              case "emitFormatWarnings", "setVersion", "setDependencies" -> null;
              default -> throw new UnsupportedOperationException(method.toString());
            };
          });
    } catch (ReflectiveOperationException e) {
      throw new RuntimeException(e);
    }
  }

  static Object dependency(Dep dep) {
    try {
      Class<?> type = Class.forName("net.fabricmc.loader.api.metadata.ModDependency");
      Class<?> kind = Class.forName("net.fabricmc.loader.api.metadata.ModDependency$Kind");
      Object depends = Enum.valueOf((Class) kind, "DEPENDS");
      return Proxy.newProxyInstance(ModPrioSorterOracle.class.getClassLoader(), new Class<?>[] {type},
          (proxy, method, args) -> switch (method.getName()) {
            case "getModId" -> dep.id();
            case "getKind" -> depends;
            case "matches" -> matches(dep.range(), (Version) args[0]);
            case "toString" -> dep.id() + " " + dep.range();
            default -> method.getReturnType().isPrimitive() ? false : null;
          });
    } catch (ReflectiveOperationException e) {
      throw new RuntimeException(e);
    }
  }

  static boolean matches(String range, Version version) throws Exception {
    String value = version.getFriendlyString();
    if (range.startsWith("<")) return Version.parse(value).compareTo(Version.parse(range.substring(1))) < 0;
    if (range.startsWith(">=")) return Version.parse(value).compareTo(Version.parse(range.substring(2))) >= 0;
    return Version.parse(value).compareTo(Version.parse(range)) == 0;
  }

  static Object candidate(Spec spec, Map<String, Object> byLabel, Class<?> type) throws Exception {
    Object metadata = metadata(spec);
    Object candidate;
    // ModCandidateImpl owns these mutable collections and ModResolver removes
    // rejected candidates from them while it cleans up a successful result.
    Collection<Object> nested = new ArrayList<>();
    if (spec.root()) {
      Method method = type.getDeclaredMethod(
          "createPlain", List.class, Class.forName("net.fabricmc.loader.impl.metadata.LoaderModMetadata"), boolean.class, Collection.class);
      method.setAccessible(true);
      candidate = method.invoke(null, List.of(Path.of(spec.label())), metadata, false, nested);
    } else {
      Method method = type.getDeclaredMethod(
          "createNested", String.class, long.class, Class.forName("net.fabricmc.loader.impl.metadata.LoaderModMetadata"), boolean.class, Collection.class);
      method.setAccessible(true);
      candidate = method.invoke(null, spec.label(), 1L, metadata, false, nested);
    }
    byLabel.put(spec.label(), candidate);
    return candidate;
  }

  static List<Spec> parse(String line) {
    List<Spec> out = new ArrayList<>();
    for (String raw : line.split(";")) {
      if (raw.isBlank()) continue;
      String[] fields = raw.split(",", -1);
      out.add(new Spec(
          fields[0], fields[1], fields[2], fields[3].equals("1"),
          fields[4].equals("-") ? List.of() : List.of(fields[4].split("/")),
          fields.length < 6 || fields[5].isBlank() || fields[5].equals("-") ? null : fields[5],
          fields.length < 7 || fields[6].isBlank() || fields[6].equals("-") ? List.of()
              : Arrays.stream(fields[6].split("/")).map(value -> {
                  String[] dep = value.split(":", 2);
                  return new Dep(dep[0], dep.length == 1 ? ">=0" : dep[1]);
                }).toList()));
    }
    return out;
  }

  /** Run Fabric's own candidate finders and discovery graph. */
  static final class RealDiscovery {
    static boolean launcherReady;
    final Class<?> candidateType = Class.forName("net.fabricmc.loader.impl.discovery.ModCandidateImpl");
    final Method id = candidateType.getMethod("getId");
    final Method version = candidateType.getMethod("getVersion");
    final Method root = candidateType.getMethod("isRoot");
    final Method level = candidateType.getMethod("getMinNestLevel");
    final Method discover;
    final Object loader;
    final Object versionOverrides = Class.forName("net.fabricmc.loader.impl.metadata.VersionOverrides")
        .getConstructor().newInstance();
    final Object dependencyOverrides = Class.forName("net.fabricmc.loader.impl.metadata.DependencyOverrides")
        .getConstructor(Path.class).newInstance(Path.of("oracle-no-overrides"));
    final Method createBuiltin = candidateType.getDeclaredMethod("createBuiltin",
        Class.forName("net.fabricmc.loader.impl.game.GameProvider$BuiltinMod"),
        Class.forName("net.fabricmc.loader.impl.metadata.VersionOverrides"),
        Class.forName("net.fabricmc.loader.impl.metadata.DependencyOverrides"));
    final Constructor<?> builtinConstructor = Class.forName("net.fabricmc.loader.impl.game.GameProvider$BuiltinMod")
        .getConstructor(List.class, Class.forName("net.fabricmc.loader.api.metadata.ModMetadata"));
    final Constructor<?> metadataBuilder = Class.forName("net.fabricmc.loader.impl.metadata.BuiltinModMetadata$Builder")
        .getConstructor(String.class, String.class);
    final Method metadataBuild = metadataBuilder.getDeclaringClass().getMethod("build");

    RealDiscovery() throws Exception {
      Class<?> launcher = Class.forName("net.fabricmc.loader.impl.launch.FabricLauncher");
      if (!launcherReady) {
        Object launcherProxy = Proxy.newProxyInstance(getClass().getClassLoader(), new Class<?>[] {launcher},
            (proxy, method, args) -> switch (method.getName()) {
              case "getEnvironmentType" -> Enum.valueOf((Class) Class.forName("net.fabricmc.api.EnvType"), environment);
              case "isDevelopment" -> false;
              case "getClassPath" -> List.of();
              default -> method.getReturnType().isPrimitive() ? false : null;
            });
        Class.forName("net.fabricmc.loader.impl.launch.FabricLauncherBase")
            .getMethod("setLauncher", launcher).invoke(null, launcherProxy);
        launcherReady = true;
      }
      loader = Class.forName("net.fabricmc.loader.impl.FabricLoaderImpl")
          .getField("INSTANCE").get(null);
      Class<?> providerType = Class.forName("net.fabricmc.loader.impl.game.GameProvider");
      Object provider = Proxy.newProxyInstance(getClass().getClassLoader(), new Class<?>[] {providerType},
          (proxy, method, args) -> switch (method.getName()) {
            case "getLaunchDirectory" -> Path.of("oracle-real-scratch").toAbsolutePath();
            case "getRawGameVersion", "getNormalizedGameVersion" -> "26.3";
            case "getArguments" -> new net.fabricmc.loader.impl.util.Arguments();
            case "isDevelopmentEnvironment" -> false;
            case "getBuiltinMods" -> List.of(builtinMod("minecraft", "26.3"), builtinMod("fabricloader", "0.19.5"));
            default -> method.getReturnType().isPrimitive() ? false : null;
          });
      loader.getClass().getMethod("setGameProvider", providerType).invoke(loader, provider);
      Class<?> discovererType = Class.forName("net.fabricmc.loader.impl.discovery.ModDiscoverer");
      Object discoverer = discovererType.getConstructor(
          Class.forName("net.fabricmc.loader.impl.metadata.VersionOverrides"),
          Class.forName("net.fabricmc.loader.impl.metadata.DependencyOverrides"))
          .newInstance(versionOverrides, dependencyOverrides);
      discovererType.getMethod("addCandidateFinder", Class.forName("net.fabricmc.loader.impl.discovery.ModCandidateFinder"))
          .invoke(discoverer, newFinder("net.fabricmc.loader.impl.discovery.ArgumentModCandidateFinder"));
      discover = discovererType.getMethod("discoverMods", loader.getClass(), Map.class);
      this.discoverer = discoverer;
    }

    final Object discoverer;

    Object builtinMod(String id, String value) throws Exception {
      Object metadata = metadataBuild.invoke(metadataBuilder.newInstance(id, value));
      return builtinConstructor.newInstance(List.of(), metadata);
    }

    Object builtin(String id, String value) throws Exception {
      Object gameMod = builtinMod(id, value);
      createBuiltin.setAccessible(true);
      return createBuiltin.invoke(null, gameMod, versionOverrides, dependencyOverrides);
    }

    Object newFinder(String name) throws Exception {
      Class<?> type = Class.forName(name);
      Object finder = type.getConstructor(boolean.class).newInstance(false);
      if (name.endsWith("ArgumentModCandidateFinder")) {
        System.setProperty("fabric.addMods", "");
      }
      return finder;
    }

    List<?> find(Path mods) throws Exception {
      Class<?> directory = Class.forName("net.fabricmc.loader.impl.discovery.DirectoryModCandidateFinder");
      Object finder = directory.getConstructor(Path.class, boolean.class).newInstance(mods, false);
      Class<?> finderType = Class.forName("net.fabricmc.loader.impl.discovery.ModCandidateFinder");
      Class<?> discovererType = discoverer.getClass();
      discovererType.getMethod("addCandidateFinder", finderType).invoke(discoverer, finder);
      return (List<?>) discover.invoke(discoverer, loader, new HashMap<>());
    }

    String row(List<?> selected) throws Exception {
      StringJoiner row = new StringJoiner(",");
      for (Object candidate : selected) {
        row.add(id.invoke(candidate) + "@" + version.invoke(candidate) + "@"
            + (root.invoke(candidate).equals(Boolean.TRUE) ? "root" : "nested") + "@" + level.invoke(candidate));
      }
      return row.toString();
    }
  }

  static String escaped(String value) {
    return value.replace("\\", "\\\\").replace("\r", "\\r").replace("\n", "\\n");
  }

  static void zip(Path path, Map<String, byte[]> entries) throws IOException {
    try (ZipOutputStream out = new ZipOutputStream(Files.newOutputStream(path))) {
      for (var entry : entries.entrySet()) {
        out.putNextEntry(new ZipEntry(entry.getKey()));
        out.write(entry.getValue());
        out.closeEntry();
      }
    }
  }

  static List<Path> syntheticGenerated(String name, Path root) throws IOException {
    Files.createDirectories(root);
    if (name.equals("root-constraint") || name.equals("root-breaks")
        || name.equals("nested-sibling-constraint") || name.equals("two-top-level-same-path")
        || name.equals("alias-constraint") || name.equals("caret-breaks")
        || name.equals("tilde-breaks")) {
      boolean alias = name.equals("alias-constraint") || name.equals("caret-breaks") || name.equals("tilde-breaks");
      String oldVersion = name.equals("caret-breaks") ? "1.2" : name.equals("tilde-breaks") ? "1.1.9" : "1.0";
      String newVersion = name.equals("caret-breaks") ? "1.3" : name.equals("tilde-breaks") ? "1.2.5" : "2.0";
      String oldMetadata = "{\"schemaVersion\":1,\"id\":\"shared-lib\",\"version\":\"" + oldVersion
          + (alias ? "\",\"provides\":[\"shared-api\"]}" : "\"}");
      String newMetadata = "{\"schemaVersion\":1,\"id\":\"shared-lib\",\"version\":\"" + newVersion
          + (alias ? "\",\"provides\":[\"shared-api\"]}" : "\"}");
      byte[] oldBytes = oldMetadata.getBytes();
      byte[] newBytes = newMetadata.getBytes();
      byte[] pinnerBytes = ("{\"schemaVersion\":1,\"id\":\"pinner\",\"version\":\"1.0\","
          + "\"depends\":{\"shared-lib\":\"<2.0.0\"}}").getBytes();
      Path oldJar = root.resolve("shared-old.jar");
      Path newJar = root.resolve("shared-new.jar");
      Path pinnerJar = root.resolve("pinner.jar");
      zip(oldJar, Map.of("fabric.mod.json", oldBytes));
      zip(newJar, Map.of("fabric.mod.json", newBytes));
      zip(pinnerJar, Map.of("fabric.mod.json", pinnerBytes));
      if (name.equals("two-top-level-same-path")) {
        Path first = root.resolve("carrier-first.jar");
        Path second = root.resolve("carrier-second.jar");
        zip(first, Map.of(
            "fabric.mod.json", "{\"schemaVersion\":1,\"id\":\"carrier-first\",\"version\":\"1\",\"jars\":[{\"file\":\"META-INF/jars/shared.jar\"}]}".getBytes(),
            "META-INF/jars/shared.jar", Files.readAllBytes(oldJar)));
        zip(second, Map.of(
            "fabric.mod.json", "{\"schemaVersion\":1,\"id\":\"carrier-second\",\"version\":\"1\",\"jars\":[{\"file\":\"META-INF/jars/shared.jar\"}]}".getBytes(),
            "META-INF/jars/shared.jar", Files.readAllBytes(newJar)));
        return List.of(first, second);
      }
      String constraint = name.equals("root-breaks")
          ? "\"breaks\":{\"shared-lib\":\">=2.0.0\"},"
          : name.equals("alias-constraint")
              ? "\"depends\":{\"shared-api\":\"<2.0.0\"},"
              : name.equals("caret-breaks")
                  ? "\"breaks\":{\"shared-api\":\"^1.2\"},"
                  : name.equals("tilde-breaks")
                      ? "\"breaks\":{\"shared-api\":\"~1.2\"},"
          : name.equals("root-constraint")
              ? "\"depends\":{\"shared-lib\":\"<2.0.0\"},"
              : "";
      String members = name.equals("nested-sibling-constraint")
          ? "META-INF/jars/pinner.jar\"},{\"file\":\"META-INF/jars/shared-old.jar\"},{\"file\":\"META-INF/jars/shared-new.jar"
          : "shared-old.jar\"},{\"file\":\"shared-new.jar";
      String metadata = "{\"schemaVersion\":1,\"id\":\"carrier\",\"version\":\"1\","
          + constraint + "\"jars\":[{\"file\":\"" + members + "\"}]}";
      Path carrier = root.resolve("carrier.jar");
      Map<String, byte[]> entries = new HashMap<>();
      entries.put("fabric.mod.json", metadata.getBytes());
      entries.put(name.equals("nested-sibling-constraint") ? "META-INF/jars/shared-old.jar" : "shared-old.jar", Files.readAllBytes(oldJar));
      entries.put(name.equals("nested-sibling-constraint") ? "META-INF/jars/shared-new.jar" : "shared-new.jar", Files.readAllBytes(newJar));
      if (name.equals("nested-sibling-constraint")) entries.put("META-INF/jars/pinner.jar", Files.readAllBytes(pinnerJar));
      zip(carrier, entries);
      return List.of(carrier);
    }
    if (name.equals("plain-library")) {
      Path plain = root.resolve("plain.jar");
      zip(plain, Map.of("plain/Library.class", new byte[] {0}));
      Path carrier = root.resolve("plain-carrier.jar");
      String metadata = "{\"schemaVersion\":1,\"id\":\"plain-carrier\",\"version\":\"1\","
          + "\"environment\":\"client\",\"jars\":[{\"file\":\"META-INF/jars/plain.jar\"}]}";
      zip(carrier, Map.of("fabric.mod.json", metadata.getBytes(),
          "META-INF/jars/plain.jar", Files.readAllBytes(plain)));
      return List.of(carrier);
    }
    byte[] childOne = ("{\"schemaVersion\":1,\"id\":\"generated-lib\","
        + "\"version\":\"1.0\",\"custom\":{\"fabric-loom:generated\":true}}")
        .getBytes(java.nio.charset.StandardCharsets.UTF_8);
    byte[] childTwo = ("{\"schemaVersion\":1,\"id\":\"generated-lib\","
        + "\"version\":\"2.0\",\"custom\":{\"fabric-loom:generated\":true}}")
        .getBytes(java.nio.charset.StandardCharsets.UTF_8);
    Path childOnePath = root.resolve("lib-one.jar");
    Path childTwoPath = root.resolve("lib-two.jar");
    zip(childOnePath, Map.of("fabric.mod.json", childOne, "marker.txt", "one".getBytes()));
    zip(childTwoPath, Map.of("fabric.mod.json", childTwo, "marker.txt", "two".getBytes()));
    String firstId = name.equals("generated-single") ? "carrier" : "carrier_a";
    String firstMetadata = "{\"schemaVersion\":1,\"id\":\"carrier_a\",\"version\":\"1\","
        + "\"jars\":[{\"file\":\"META-INF/jars/lib.jar\"}]}";
    String secondMetadata = "{\"schemaVersion\":1,\"id\":\"carrier_b\",\"version\":\"1\","
        + "\"jars\":[{\"file\":\"META-INF/jars/lib.jar\"}]}";
    String singleMetadata = name.equals("generated-single")
        ? ("{\"schemaVersion\":1,\"id\":\"" + firstId + "\",\"version\":\"1\","
            + "\"jars\":[{\"file\":\"META-INF/jars/lib.jar\"}]}")
        : firstMetadata;
    Path first = root.resolve("carrier-a.jar");
    zip(first, Map.of("fabric.mod.json", singleMetadata.getBytes(),
        "META-INF/jars/lib.jar", Files.readAllBytes(childOnePath)));
    if (name.equals("generated-single")) return List.of(first);
    Path second = root.resolve("carrier-b.jar");
    zip(second, Map.of("fabric.mod.json", secondMetadata.getBytes(),
        "META-INF/jars/lib.jar", Files.readAllBytes(childTwoPath)));
    return List.of(first, second);
  }

  public static void main(String[] args) throws Exception {
    Class<?> type = Class.forName("net.fabricmc.loader.impl.discovery.ModCandidateImpl");
    Method addParent = type.getDeclaredMethod("addParent", type);
    addParent.setAccessible(true);
    Method id = type.getMethod("getId");
    Method version = type.getMethod("getVersion");
    Method isRoot = type.getMethod("isRoot");
    Method level = type.getMethod("getMinNestLevel");
    Method nestedMods = type.getMethod("getNestedMods");
    Method resolve = Class.forName("net.fabricmc.loader.impl.discovery.ModResolver")
        .getMethod("resolve", Collection.class, Class.forName("net.fabricmc.api.EnvType"), Map.class);
    Class<?> envType = Class.forName("net.fabricmc.api.EnvType");
    // Fabric's resolver logs diagnostics for an unsatisfied SAT result. This
    // inert handler keeps those diagnostics off the oracle data stream.
    Class<?> log = Class.forName("net.fabricmc.loader.impl.util.log.Log");
    Class<?> handlerType = Class.forName("net.fabricmc.loader.impl.util.log.LogHandler");
    Object silentHandler = Proxy.newProxyInstance(
        ModPrioSorterOracle.class.getClassLoader(), new Class<?>[] {handlerType},
        (proxy, method, args1) -> method.getName().equals("shouldLog") ? false : null);
    log.getMethod("init", handlerType).invoke(null, silentHandler);

    if (args.length > 1) environment = args[1].toUpperCase(Locale.ROOT);
    Object activeEnvironment = Enum.valueOf((Class) envType, environment);
    for (String line : Files.readAllLines(Path.of(args[0]))) {
      if (line.isBlank() || line.startsWith("#")) continue;
      String[] split = line.split("\\|", 2);
      if (split[1].startsWith("synthetic=")) {
        try {
          String name = split[1].substring("synthetic=".length());
          Path mods = Path.of("oracle-real-scratch").resolve(split[0]).resolve("mods");
          for (Path source : syntheticGenerated(name, mods.getParent().resolve("inputs"))) {
            Files.createDirectories(mods);
            Files.copy(source, mods.resolve(source.getFileName()), StandardCopyOption.REPLACE_EXISTING);
          }
          RealDiscovery real = new RealDiscovery();
          List<?> discovered = real.find(mods);
          List<?> selected = (List<?>) resolve.invoke(null, discovered, activeEnvironment, new HashMap<>());
          System.out.println(split[0] + "|OK|" + real.row(selected));
        } catch (InvocationTargetException error) {
          Throwable cause = error.getCause();
          System.out.println(split[0] + "|ERROR|" + cause.getClass().getName() + "|" + escaped(cause.getMessage()));
        }
        continue;
      }
      if (split[1].startsWith("jar=")) {
        try {
          Path mods = Path.of("oracle-real-scratch").resolve(split[0]).resolve("mods");
          Files.createDirectories(mods);
          for (String raw : split[1].substring(4).split("\\|")) {
            Path source = Path.of(raw);
            if (!source.isAbsolute() && !Files.isRegularFile(source)) {
              String corpus = System.getenv("HORIZON_CORPUS");
              if (corpus != null) source = Path.of(corpus).resolve(source);
            }
            Files.copy(source, mods.resolve(source.getFileName()), StandardCopyOption.REPLACE_EXISTING);
          }
          RealDiscovery real = new RealDiscovery();
          List<?> discovered = real.find(mods);
          List<?> selected = (List<?>) resolve.invoke(null, discovered, activeEnvironment, new HashMap<>());
          System.out.println(split[0] + "|OK|" + real.row(selected));
        } catch (InvocationTargetException error) {
          Throwable cause = error.getCause();
          System.out.println(split[0] + "|ERROR|" + cause.getClass().getName() + "|" + escaped(cause.getMessage()));
        }
        continue;
      }
      List<Spec> specs = parse(split[1]);
      Map<String, Object> byLabel = new HashMap<>();
      List<Object> candidates = new ArrayList<>();
      for (Spec spec : specs) candidates.add(candidate(spec, byLabel, type));
      for (int i = 0; i < specs.size(); i++) {
        String parent = specs.get(i).parent();
        if (parent != null) {
          ((Collection<Object>) nestedMods.invoke(byLabel.get(parent))).add(candidates.get(i));
          addParent.invoke(candidates.get(i), byLabel.get(parent));
        }
      }
      try {
        List<?> selected = (List<?>) resolve.invoke(null, candidates, activeEnvironment, new HashMap<>());
        StringJoiner row = new StringJoiner(",");
        for (Object candidate : selected) {
          row.add(id.invoke(candidate) + "@" + version.invoke(candidate) + "@"
              + (isRoot.invoke(candidate).equals(Boolean.TRUE) ? "root" : "nested") + "@" + level.invoke(candidate));
        }
        System.out.println(split[0] + "|OK|" + row);
      } catch (InvocationTargetException error) {
        Throwable cause = error.getCause();
        System.out.println(split[0] + "|ERROR|" + cause.getClass().getName() + "|" + escaped(cause.getMessage()));
      }
    }
  }
}
