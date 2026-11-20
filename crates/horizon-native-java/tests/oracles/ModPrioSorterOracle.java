import java.lang.reflect.*;
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
      if (line.isBlank()) continue;
      String[] split = line.split("\\|", 2);
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
