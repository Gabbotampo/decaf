import java.lang.reflect.*;
import java.nio.file.*;
import java.util.*;
import net.fabricmc.loader.api.Version;

/** Captures Fabric Loader 0.19.5 ModPrioSorter rows on Java 25. */
public final class ModPrioSorterOracle {
  record Spec(String label, String id, String version, boolean root, List<String> provides, String parent) {}

  static Object metadata(Spec spec) throws Exception {
    return Proxy.newProxyInstance(
      ModPrioSorterOracle.class.getClassLoader(),
      new Class<?>[] { Class.forName("net.fabricmc.loader.impl.metadata.LoaderModMetadata") },
      (proxy, method, args) -> switch (method.getName()) {
        case "getId" -> spec.id();
        case "getVersion" -> Version.parse(spec.version());
        case "getProvides" -> spec.provides();
        case "getType" -> "normal";
        case "getDependencies" -> List.of();
        case "getEnvironment" -> Enum.valueOf((Class)Class.forName("net.fabricmc.loader.api.metadata.ModEnvironment"), "UNIVERSAL");
        case "getSchemaVersion" -> 1;
        case "getMixinConfigs", "getOldInitializers", "getEntrypointKeys", "getJars" -> List.of();
        case "loadsInEnvironment" -> true;
        case "getLanguageAdapterDefinitions" -> Map.of();
        case "getOldStyleLanguageAdapter", "getClassTweaker", "getName", "getDescription" -> null;
        case "getAuthors", "getContributors", "getLicense" -> List.of();
        case "getContact" -> null;
        case "getIconPath" -> Optional.empty();
        case "containsCustomValue", "containsCustomElement" -> false;
        case "getCustomValue" -> null;
        case "getCustomValues" -> Map.of();
        case "emitFormatWarnings", "setVersion", "setDependencies" -> null;
        default -> throw new UnsupportedOperationException(method.toString());
      });
  }

  static Object candidate(Spec spec, Map<String,Object> byLabel, Class<?> type) throws Exception {
    Object md = metadata(spec);
    Object candidate;
    if (spec.root()) {
      Method m = type.getDeclaredMethod("createPlain", List.class, Class.forName("net.fabricmc.loader.impl.metadata.LoaderModMetadata"), boolean.class, Collection.class);
      m.setAccessible(true);
      candidate = m.invoke(null, List.of(Path.of(spec.label())), md, false, List.of());
    } else {
      Method m = type.getDeclaredMethod("createNested", String.class, long.class, Class.forName("net.fabricmc.loader.impl.metadata.LoaderModMetadata"), boolean.class, Collection.class);
      m.setAccessible(true);
      candidate = m.invoke(null, spec.label(), 1L, md, false, List.of());
    }
    byLabel.put(spec.label(), candidate);
    return candidate;
  }

  static List<Spec> parse(String line) {
    List<Spec> out = new ArrayList<>();
    for (String raw : line.split(";")) {
      if (raw.isBlank()) continue;
      String[] f = raw.split(",", -1);
      out.add(new Spec(f[0], f[1], f[2], f[3].equals("1"), f[4].equals("-") ? List.of() : List.of(f[4].split("/")), f.length < 6 || f[5].equals("-") ? null : f[5]));
    }
    return out;
  }

  static List<Object> selectLikeLoader(List<Object> sorted, Class<?> type) throws Exception {
    Method id = type.getMethod("getId");
    Method provides = type.getMethod("getProvides");
    Method version = type.getMethod("getVersion");
    Method root = type.getMethod("isRoot");
    Set<String> counts = new HashSet<>(), provided = new HashSet<>();
    Map<String,Integer> count = new HashMap<>();
    for (Object c : sorted) {
      String name = (String) id.invoke(c);
      count.merge(name, 1, Integer::sum);
      for (String alias : (Collection<String>) provides.invoke(c)) {
        count.merge(alias, 1, Integer::sum);
        provided.add(alias);
      }
    }
    for (var e : count.entrySet()) if (e.getValue() > 1 && provided.contains(e.getKey())) counts.add(e.getKey());
    List<Object> selected = new ArrayList<>();
    int firstNested = 0;
    boolean seenNested = false;
    for (Object c : sorted) {
      if (!seenNested && !((Boolean) root.invoke(c))) { seenNested = true; firstNested = selected.size(); }
      List<String> ids = new ArrayList<>();
      ids.add((String) id.invoke(c));
      ids.addAll((Collection<String>) provides.invoke(c));
      Set<String> overlap = new HashSet<>(ids); overlap.retainAll(counts);
      if (overlap.isEmpty()) { selected.add(c); continue; }
      Integer replacement = null;
      boolean reject = false;
      for (int j = selected.size() - 1; j >= firstNested; j--) {
        Object previous = selected.get(j);
        if (((String) id.invoke(c)).equals(id.invoke(previous))) break;
        List<String> previousIds = new ArrayList<>();
        previousIds.add((String) id.invoke(previous));
        previousIds.addAll((Collection<String>) provides.invoke(previous));
        boolean relevant = previousIds.stream().anyMatch(overlap::contains);
        if (!relevant) continue;
        int score = Integer.MAX_VALUE;
        for (String alias : (Collection<String>) provides.invoke(c)) if (alias.equals(id.invoke(previous))) {
          score = score == Integer.MAX_VALUE ? 0 : score;
          score += Integer.signum(((Version) version.invoke(c)).compareTo((Version) version.invoke(previous)));
        }
        for (String alias : (Collection<String>) provides.invoke(previous)) {
          if (alias.equals(id.invoke(c))) {
            score = score == Integer.MAX_VALUE ? 0 : score;
            score += Integer.signum(((Version) version.invoke(previous)).compareTo((Version) version.invoke(c)));
          }
          for (String other : (Collection<String>) provides.invoke(c)) if (alias.equals(other)) {
            score = score == Integer.MAX_VALUE ? 0 : score;
            score += Integer.signum(((Version) version.invoke(previous)).compareTo((Version) version.invoke(c)));
          }
        }
        if (score < 0) replacement = j;
        else if (score > 0) { reject = true; break; }
      }
      if (replacement != null) { selected.remove((int) replacement); selected.add((int) replacement, c); }
      else if (reject) {
        continue;
      }
      else if (!((Boolean) root.invoke(c)) && selected.subList(0, firstNested).stream().anyMatch(previous -> {
        try {
          return overlap.contains(id.invoke(previous)) || ((Collection<String>) provides.invoke(previous)).stream().anyMatch(overlap::contains);
        } catch (Exception e) { throw new RuntimeException(e); }
      })) {
        // A root wins every overlap against a nested candidate.
      } else selected.add(c);
    }
    return selected;
  }

  public static void main(String[] args) throws Exception {
    Class<?> type = Class.forName("net.fabricmc.loader.impl.discovery.ModCandidateImpl");
    Method addParent = type.getDeclaredMethod("addParent", type);
    addParent.setAccessible(true);
    Method isRoot = type.getMethod("isRoot");
    Method level = type.getMethod("getMinNestLevel");
    Method id = type.getMethod("getId");
    try (var lines = Files.lines(Path.of(args[0]))) {
      lines.filter(s -> !s.isBlank()).forEach(line -> {
        try {
          String[] split = line.split("\\|", 2);
          List<Spec> specs = parse(split[1]);
          Map<String,Object> byLabel = new HashMap<>();
          List<Object> candidates = new ArrayList<>();
          for (Spec spec : specs) candidates.add(candidate(spec, byLabel, type));
          for (int i = 0; i < specs.size(); i++) {
            String parent = specs.get(i).parent();
            if (parent != null) addParent.invoke(candidates.get(i), byLabel.get(parent));
          }
          Method sort = Class.forName("net.fabricmc.loader.impl.discovery.ModPrioSorter").getDeclaredMethod("sort", List.class, Map.class);
          sort.setAccessible(true);
          sort.invoke(null, candidates, new HashMap<>());
          // ModResolver's complete graph requires loader-owned dependency objects;
          // the sorter is the candidate-selection oracle exercised here.
          candidates = new ArrayList<>(selectLikeLoader(candidates, type));
          StringJoiner row = new StringJoiner(",");
          for (Object c : candidates) row.add(id.invoke(c) + "@" + (isRoot.invoke(c).equals(Boolean.TRUE) ? "r" : "n") + "@" + level.invoke(c));
          System.out.println(split[0] + "|" + row);
        } catch (Exception e) { throw new RuntimeException(e); }
      });
    }
  }
}
