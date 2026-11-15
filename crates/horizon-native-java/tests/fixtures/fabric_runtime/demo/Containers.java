package demo;

import java.nio.file.Path;
import java.util.Collection;
import java.util.List;
import net.fabricmc.loader.api.FabricLoader;
import net.fabricmc.loader.api.ModContainer;

/**
 * Two mods' containers keep their own collections (#233 review B1): Fabric Loader 0.19.5's
 * `ModContainerImpl.getRootPaths` returns the container's own `roots` field, and `V1ModMetadata`'s
 * getters return its own fields, so each is the same object on every call, never another mod's.
 * Those collections are unmodifiable in Fabric (`unmodifiableCollection`, a `singletonList`), which
 * the host doesn't model yet (#238), so no probe writes to them.
 */
public final class Containers {
    static String b(boolean v) {
        return v ? "T" : "F";
    }

    static ModContainer mod(String id) {
        return FabricLoader.getInstance().getModContainer(id).get();
    }

    /** `a`'s root paths: one list, still `a`'s after `c`'s are read. */
    public static String roots() {
        ModContainer a = mod("moda");
        ModContainer c = mod("modb");
        List<Path> ra = a.getRootPaths();
        String before = ra.get(0).toString();
        List<Path> rb = c.getRootPaths();
        return b(before.equals(ra.get(0).toString())) + b(ra.get(0).equals(rb.get(0))) + b(ra == rb)
            + b(ra == a.getRootPaths()) + b(rb == c.getRootPaths()) + ra.size() + rb.size();
    }

    /** Metadata collections: one per mod per getter, the same on every call. */
    public static String metadata() {
        ModContainer a = mod("moda");
        ModContainer c = mod("modb");
        Collection<String> license = a.getMetadata().getLicense();
        return b((Object) a.getMetadata().getAuthors() == c.getMetadata().getAuthors())
            + b(a.getMetadata().getAuthors() == a.getMetadata().getAuthors())
            + b(license == c.getMetadata().getLicense()) + b(license == a.getMetadata().getLicense())
            + a.getMetadata().getLicense() + c.getMetadata().getLicense();
    }

    /** Fabric Loader 0.19.5 wraps these fields; every mutator throws UOE. */
    @SuppressWarnings({"rawtypes", "unchecked"})
    public static String mutators() {
        ModContainer a = mod("moda");
        StringBuilder out = new StringBuilder();
        try { ((Collection) a.getMetadata().getLicense()).add("MIT"); out.append('N'); }
        catch (UnsupportedOperationException e) { out.append('U'); }
        try { ((Collection) a.getMetadata().getAuthors()).add(null); out.append('N'); }
        catch (UnsupportedOperationException e) { out.append('U'); }
        try { ((Collection) a.getMetadata().getContributors()).add(null); out.append('N'); }
        catch (UnsupportedOperationException e) { out.append('U'); }
        try { ((Collection) a.getMetadata().getProvides()).add("alias"); out.append('N'); }
        catch (UnsupportedOperationException e) { out.append('U'); }
        try { a.getMetadata().getDependencies().add((net.fabricmc.loader.api.metadata.ModDependency) null); out.append('N'); }
        catch (UnsupportedOperationException e) { out.append('U'); }
        try { a.getRootPaths().add(Path.of("x")); out.append('N'); }
        catch (UnsupportedOperationException e) { out.append('U'); }
        return out.toString();
    }

    /** `UnmodifiableCollection` keeps Object identity equals/hashCode and delegates toString. */
    public static String wrapperSemantics() {
        ModContainer a = mod("moda");
        Collection<String> first = a.getMetadata().getLicense();
        Collection<String> same = a.getMetadata().getLicense();
        Collection<String> other = mod("modb").getMetadata().getLicense();
        return b(first.equals(same)) + b(first.equals(other))
            + b(first.hashCode() == same.hashCode()) + first.toString();
    }
    /** Parsed V1 metadata keeps populated values in the same wrappers. */
    @SuppressWarnings({"rawtypes", "unchecked"})
    public static String populated() {
        ModContainer c = mod("probe-fixture");
        var m = c.getMetadata();
        Collection authors = m.getAuthors();
        Collection contributors = m.getContributors();
        Collection license = m.getLicense();
        Collection provides = m.getProvides();
        Collection deps = m.getDependencies();
        StringBuilder out = new StringBuilder();
        out.append(authors.size()).append(':').append(contributors.size()).append(':')
            .append(license.size()).append(':').append(provides.size()).append(':').append(deps.size());
        out.append(':').append(((net.fabricmc.loader.api.metadata.Person) authors.iterator().next()).getName());
        for (Collection values : new Collection[]{authors, contributors, license, provides, deps}) {
            try { values.add("x"); out.append('N'); } catch (UnsupportedOperationException e) { out.append('U'); }
            var it = values.iterator();
            if (it.hasNext()) { it.next(); try { it.remove(); out.append('N'); } catch (UnsupportedOperationException e) { out.append('U'); } }
        }
        return out.toString();
    }

    public static String originPaths() {
        ModContainer c = mod("probe-fixture");
        var a = c.getOrigin().getPaths();
        var b = c.getOrigin().getPaths();
        return a.getClass().getName() + ":" + a.equals(b) + ":" + (a == b) + ":" + a;
    }

    public static String dependencyKinds() {
        var m = mod("probe-fixture").getMetadata();
        StringBuilder out = new StringBuilder();
        for (var d : m.getDependencies()) out.append(d.getKind()).append(':').append(d.getModId()).append(';');
        return out.toString();
    }

}
