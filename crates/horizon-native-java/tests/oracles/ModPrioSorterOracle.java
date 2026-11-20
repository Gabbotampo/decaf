import java.lang.reflect.*;
import java.io.*;
import java.nio.file.*;
import java.util.*;
import java.util.zip.*;
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

  /**
   * Read real Fabric metadata and nested entries. The paths used by the
   * captured rows below are third-party corpus jars and are deliberately not
   * copied into this repository.
   */
  static final class RealCandidates {
    final Class<?> type;
    final Method createBuiltin, createPlain, createNested, addParent, nestedMods;
    final Constructor<?> builtinConstructor, metadataBuilderConstructor;
    final Method metadataBuild;
    final Method parseMetadata;
    final Object versionOverrides, dependencyOverrides;
    final Map<Long, Object> byHash = new HashMap<>();

    RealCandidates(Class<?> type) throws Exception {
      this.type = type;
      createBuiltin = type.getDeclaredMethod("createBuiltin",
          Class.forName("net.fabricmc.loader.impl.game.GameProvider$BuiltinMod"),
          Class.forName("net.fabricmc.loader.impl.metadata.VersionOverrides"),
          Class.forName("net.fabricmc.loader.impl.metadata.DependencyOverrides"));
      createPlain = type.getDeclaredMethod("createPlain", List.class,
          Class.forName("net.fabricmc.loader.impl.metadata.LoaderModMetadata"),
          boolean.class, Collection.class);
      createNested = type.getDeclaredMethod("createNested", String.class, long.class,
          Class.forName("net.fabricmc.loader.impl.metadata.LoaderModMetadata"),
          boolean.class, Collection.class);
      addParent = type.getDeclaredMethod("addParent", type);
      nestedMods = type.getMethod("getNestedMods");
      createPlain.setAccessible(true);
      createNested.setAccessible(true);
      createBuiltin.setAccessible(true);
      addParent.setAccessible(true);
      parseMetadata = Class.forName("net.fabricmc.loader.impl.metadata.ModMetadataParser")
          .getMethod("parseMetadata", InputStream.class, String.class, List.class,
              Class.forName("net.fabricmc.loader.impl.metadata.VersionOverrides"),
              Class.forName("net.fabricmc.loader.impl.metadata.DependencyOverrides"),
              boolean.class);
      versionOverrides = Class.forName("net.fabricmc.loader.impl.metadata.VersionOverrides")
          .getConstructor().newInstance();
      dependencyOverrides = Class.forName("net.fabricmc.loader.impl.metadata.DependencyOverrides")
          .getConstructor(Path.class).newInstance(Path.of("oracle-no-overrides"));
      Class<?> builder = Class.forName("net.fabricmc.loader.impl.metadata.BuiltinModMetadata$Builder");
      metadataBuilderConstructor = builder.getConstructor(String.class, String.class);
      metadataBuild = builder.getMethod("build");
      builtinConstructor = Class.forName("net.fabricmc.loader.impl.game.GameProvider$BuiltinMod")
          .getConstructor(List.class, Class.forName("net.fabricmc.loader.api.metadata.ModMetadata"));
    }

    Object builtin(String id, String version) throws Exception {
      Object builder = metadataBuilderConstructor.newInstance(id, version);
      Object metadata = metadataBuild.invoke(builder);
      Object gameMod = builtinConstructor.newInstance(List.of(), metadata);
      return createBuiltin.invoke(null, gameMod, versionOverrides, dependencyOverrides);
    }

    // `javap -p` on Fabric Loader 0.19.5 shows this package-private factory:
    // ModCandidateImpl.createBuiltin(GameProvider.BuiltinMod, VersionOverrides,
    // DependencyOverrides). Use that same factory, rather than a plain
    // candidate, so the resolver sees the candidates as real built-ins.
    List<Object> builtinMods() throws Exception {
      return List.of(builtin("minecraft", "26.3"), builtin("java", "25"),
          builtin("fabricloader", "0.19.5"));
    }

    Object metadata(InputStream stream, String path) throws Exception {
      return parseMetadata.invoke(null, stream, path, List.of(), versionOverrides,
          dependencyOverrides, false);
    }

    static Object call(Object receiver, String name, Class<?>... parameters) throws Exception {
      Method method = receiver.getClass().getMethod(name, parameters);
      method.setAccessible(true);
      return method.invoke(receiver);
    }

    Object nested(Path owner, ZipFile zip, ZipEntry entry, Object parent) throws Exception {
      byte[] bytes;
      try (InputStream in = zip.getInputStream(entry)) {
        bytes = in.readAllBytes();
      }
      long hash = (entry.getCrc() << 32) | (entry.getSize() & 0xffffffffL);
      Object candidate = byHash.get(hash);
      if (candidate == null) {
        try (InputStream in = new ByteArrayInputStream(bytes);
             ZipInputStream nestedZip = new ZipInputStream(in)) {
          ZipEntry metadataEntry;
          while ((metadataEntry = nestedZip.getNextEntry()) != null
              && !metadataEntry.getName().equals("fabric.mod.json")) {}
          if (metadataEntry == null) return null;
          Object metadata = metadata(nestedZip, owner + "!" + entry.getName());
          Collection<Object> children = new ArrayList<>();
          candidate = createNested.invoke(null, entry.getName(), hash, metadata, false, children);
          byHash.put(hash, candidate);
          scanNested(owner + "!" + entry.getName(), bytes, candidate, children);
        }
      }
      ((Collection<Object>) nestedMods.invoke(parent)).add(candidate);
      addParent.invoke(candidate, parent);
      return candidate;
    }

    void scanNested(String label, byte[] bytes, Object parent, Collection<Object> children)
        throws Exception {
      try (ZipFile zip = new ZipFile(writeScratchJar(label, bytes).toFile())) {
        Object metadata = call(parent, "getMetadata");
        Collection<?> jars = (Collection<?>) call(metadata, "getJars");
        for (Object nestedEntry : jars) {
          String file = (String) call(nestedEntry, "getFile");
          ZipEntry entry = zip.getEntry(file);
          if (entry != null && !entry.isDirectory() && file.endsWith(".jar")) {
            nested(Path.of(label), zip, entry, parent);
          }
        }
      }
    }

    // ZipFile is used so CRC and size are the same fields ModDiscoverer hashes.
    // Keep temporary copies inside the oracle's scratch directory only.
    Path writeScratchJar(String label, byte[] bytes) throws IOException {
      Path dir = Path.of("oracle-real-scratch");
      Files.createDirectories(dir);
      Path path = dir.resolve(Integer.toUnsignedString(label.hashCode()) + ".jar");
      Files.write(path, bytes);
      return path;
    }

    Object root(Path path) throws Exception {
      try (ZipFile zip = new ZipFile(path.toFile())) {
        ZipEntry metadataEntry = zip.getEntry("fabric.mod.json");
        if (metadataEntry == null) return null;
        Object metadata = metadata(zip.getInputStream(metadataEntry), path.toString());
        Collection<Object> children = new ArrayList<>();
        Object candidate = createPlain.invoke(null, List.of(path), metadata, false, children);
        Collection<?> jars = (Collection<?>) call(metadata, "getJars");
        for (Object nestedEntry : jars) {
          String file = (String) call(nestedEntry, "getFile");
          ZipEntry entry = zip.getEntry(file);
          if (entry != null && !entry.isDirectory() && file.endsWith(".jar")) {
            nested(path, zip, entry, candidate);
          }
        }
        return candidate;
      }
    }

    void addAll(Object candidate, Collection<Object> out, Set<Object> seen) throws Exception {
      if (!seen.add(candidate)) return;
      out.add(candidate);
      for (Object nested : (Collection<?>) nestedMods.invoke(candidate)) {
        addAll(nested, out, seen);
      }
    }
  }

  static String escaped(String value) {
    return value.replace("\\", "\\\\").replace("\r", "\\r").replace("\n", "\\n");
  }

  // Real-jar capture inputs (read-only third-party corpus paths):
  // /home/solenopsisbot/horizon-corpus/corpus-xl/DayCount-1.6.0-Fabric-mc26.3.jar
  // /home/solenopsisbot/horizon-corpus/mega/b1b136006d-clientcommands-2.16.3.jar
  // /home/solenopsisbot/horizon-corpus/deps-pool/fabric-api-0.166.2+26.3.jar
  // /home/solenopsisbot/horizon-corpus/corpus-xl/1283899-8898481.jar
  // /home/solenopsisbot/horizon-corpus/mega/5c80b582b9-WorldEditCUI-26.3+01.jar
  // /home/solenopsisbot/horizon-corpus/mega/8f7962199e-worldedit-mod-7.4.6.jar
  // /home/solenopsisbot/horizon-corpus/mega/2363766a77-tweakeroo-fabric-26.3-0.30.1.jar
  // /home/solenopsisbot/horizon-corpus/mega/832a00c0d6-malilib-fabric-26.3-0.30.2.jar
  // /home/solenopsisbot/horizon-corpus/neoforge/DistantHorizons-3.3.4-26.3-fabric-neoforge.jar

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
      if (split[1].startsWith("jar=")) {
        List<Object> candidates = new ArrayList<>();
        RealCandidates real = new RealCandidates(type);
        candidates.addAll(real.builtinMods());
        Set<Object> seen = Collections.newSetFromMap(new IdentityHashMap<>());
        for (String raw : split[1].substring(4).split("\\|")) {
          Object candidate = real.root(Path.of(raw));
          if (candidate != null) real.addAll(candidate, candidates, seen);
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
