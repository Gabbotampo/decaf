import java.lang.reflect.*;
import java.io.*;
import java.nio.file.*;
import java.util.*;
import net.fabricmc.loader.api.Version;

/** Captures Fabric Loader 0.19.5's complete candidate resolver on Java 25. */
public final class ModPrioSorterOracle {
  record Spec(String label, String id, String version, boolean root, List<String> provides, String parent) {}

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
              case "getDependencies" -> List.of();
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
          fields.length < 6 || fields[5].isBlank() || fields[5].equals("-") ? null : fields[5]));
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
              case "getEnvironmentType" -> Enum.valueOf((Class) Class.forName("net.fabricmc.api.EnvType"), "SERVER");
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

  public static void main(String[] args) throws Exception {
    Class<?> type = Class.forName("net.fabricmc.loader.impl.discovery.ModCandidateImpl");
    Method addParent = type.getDeclaredMethod("addParent", type);
    addParent.setAccessible(true);
    Method id = type.getMethod("getId");
    Method isRoot = type.getMethod("isRoot");
    Method level = type.getMethod("getMinNestLevel");
    Method nestedMods = type.getMethod("getNestedMods");
    Method resolve = Class.forName("net.fabricmc.loader.impl.discovery.ModResolver")
        .getMethod("resolve", Collection.class, Class.forName("net.fabricmc.api.EnvType"), Map.class);
    Class<?> envType = Class.forName("net.fabricmc.api.EnvType");
    Object server = Enum.valueOf((Class) envType, "SERVER");
    // Fabric's resolver logs diagnostics for an unsatisfied SAT result. This
    // inert handler keeps those diagnostics off the oracle data stream.
    Class<?> log = Class.forName("net.fabricmc.loader.impl.util.log.Log");
    Class<?> handlerType = Class.forName("net.fabricmc.loader.impl.util.log.LogHandler");
    Object silentHandler = Proxy.newProxyInstance(
        ModPrioSorterOracle.class.getClassLoader(), new Class<?>[] {handlerType},
        (proxy, method, args1) -> method.getName().equals("shouldLog") ? false : null);
    log.getMethod("init", handlerType).invoke(null, silentHandler);

    for (String line : Files.readAllLines(Path.of(args[0]))) {
      if (line.isBlank() || line.startsWith("#")) continue;
      String[] split = line.split("\\|", 2);
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
          List<?> selected = (List<?>) resolve.invoke(null, discovered, server, new HashMap<>());
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
        List<?> selected = (List<?>) resolve.invoke(null, candidates, server, new HashMap<>());
        StringJoiner row = new StringJoiner(",");
        for (Object candidate : selected) {
          row.add(id.invoke(candidate) + "@" + (isRoot.invoke(candidate).equals(Boolean.TRUE) ? "r" : "n") + "@" + level.invoke(candidate));
        }
        System.out.println(split[0] + "|OK|" + row);
      } catch (InvocationTargetException error) {
        Throwable cause = error.getCause();
        System.out.println(split[0] + "|ERROR|" + cause.getClass().getName() + "|" + escaped(cause.getMessage()));
      }
    }
  }
}
