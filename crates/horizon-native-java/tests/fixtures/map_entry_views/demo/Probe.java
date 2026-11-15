package demo;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import com.google.gson.JsonPrimitive;
import java.util.*;
import java.util.concurrent.*;

/**
 * Entries read through {@code Collections.unmodifiableMap} (#221 review 6, B10). Each is an
 * {@code UnmodifiableEntry} wrapping the entry the source map hands out: a Gson node keeps its
 * own value once removed, a {@code ConcurrentHashMap.MapEntry} keeps the value it was made with,
 * a {@code ConcurrentSkipListMap} entry is a snapshot.
 */
public final class Probe {
    static <K, V> Map.Entry<K, V> ent(Map<K, V> m, K k) {
        for (var e : m.entrySet()) if (e.getKey().equals(k)) return e;
        return null;
    }

    static JsonPrimitive p(Number n) {
        return new JsonPrimitive(n);
    }

    static JsonObject obj() {
        JsonObject o = new JsonObject();
        o.addProperty("a", 1);
        o.addProperty("b", 2);
        return o;
    }

    public static String v01UnmodView() {
        JsonObject o = obj();
        Map<String, JsonElement> v = Collections.unmodifiableMap(o.asMap());
        var e = ent(v, "a");
        o.remove("a");
        o.addProperty("a", 5);
        return e.getValue() + "|" + e;
    }

    public static String v09UnmodOfUnmod() {
        JsonObject o = obj();
        var v = Collections.unmodifiableMap(Collections.unmodifiableMap(o.asMap()));
        var e = ent(v, "a");
        o.remove("a");
        o.addProperty("a", 5);
        return e.getValue() + "|" + e;
    }

    public static String v10NestedObjView() {
        JsonObject o = new JsonObject();
        o.add("x", obj());
        var v = Collections.unmodifiableMap(o.getAsJsonObject("x").asMap());
        var e = ent(v, "a");
        o.getAsJsonObject("x").remove("a");
        o.getAsJsonObject("x").addProperty("a", 5);
        return e.getValue() + "|" + e;
    }

    public static String v11ArrayObjView() {
        JsonArray arr = new JsonArray();
        arr.add(obj());
        var v = Collections.unmodifiableMap(arr.get(0).getAsJsonObject().asMap());
        var e = ent(v, "a");
        arr.get(0).getAsJsonObject().remove("a");
        arr.get(0).getAsJsonObject().addProperty("a", 5);
        return e.getValue() + "|" + e;
    }

    public static String v12ViewEntrySetHeld() {
        JsonObject o = obj();
        Set<Map.Entry<String, JsonElement>> es = Collections.unmodifiableMap(o.asMap()).entrySet();
        var e = es.iterator().next();
        o.remove("a");
        o.addProperty("a", 5);
        return e.getValue() + "|" + e + "|" + es.size();
    }

    public static String v13ViewEqualsHashSet() {
        JsonObject o = obj();
        var v = Collections.unmodifiableMap(o.asMap());
        var e1 = ent(v, "a");
        o.remove("a");
        o.addProperty("a", 5);
        var e2 = ent(v, "a");
        Set<Object> s = new HashSet<>();
        s.add(e1);
        s.add(e2);
        return e1.equals(e2) + "|" + s.size() + "|" + e1.hashCode();
    }

    public static String v14ViewAsMapWrites() {
        JsonObject o = obj();
        var v = Collections.unmodifiableMap(o.asMap());
        var e = ent(v, "a");
        o.asMap().remove("a");
        o.asMap().put("a", p(5));
        return e.getValue() + "|" + e;
    }

    public static String v16ViewClear() {
        JsonObject o = obj();
        var v = Collections.unmodifiableMap(o.asMap());
        var e = ent(v, "a");
        o.asMap().clear();
        o.addProperty("a", 5);
        return e.getValue() + "|" + e;
    }

    public static String v17ViewOfParsed() {
        JsonObject o = JsonParser.parseString("{\"a\":1,\"b\":2}").getAsJsonObject();
        var v = Collections.unmodifiableMap(o.asMap());
        var e = ent(v, "a");
        o.remove("a");
        o.addProperty("a", 5);
        return e.getValue() + "|" + e;
    }

    public static String g01ViewToArray() {
        JsonObject o = obj();
        Object[] a = Collections.unmodifiableMap(o.asMap()).entrySet().toArray();
        o.remove("a");
        o.addProperty("a", 5);
        return a[0] + "";
    }

    public static String g04SyncOfUnmodGson() {
        JsonObject o = obj();
        var e = ent(Collections.synchronizedMap(Collections.unmodifiableMap(o.asMap())), "a");
        o.remove("a");
        o.addProperty("a", 5);
        return e.getValue() + "|" + e;
    }

    public static String v18ChmView() {
        var m = new ConcurrentHashMap<String, Integer>();
        m.put("a", 1);
        var e = ent(Collections.unmodifiableMap(m), "a");
        m.put("a", 5);
        return e.getValue() + "|" + e;
    }

    public static String v19ChmViewRemovePut() {
        var m = new ConcurrentHashMap<String, Integer>();
        m.put("a", 1);
        var e = ent(Collections.unmodifiableMap(m), "a");
        m.remove("a");
        m.put("a", 5);
        return e.getValue() + "|" + e;
    }

    public static String c02ChmViewOfView() {
        var m = new ConcurrentHashMap<String, Integer>();
        m.put("a", 1);
        var e = ent(Collections.unmodifiableMap(Collections.unmodifiableMap(m)), "a");
        m.put("a", 5);
        return e.getValue() + "|" + e;
    }

    public static String a02CslmView() {
        var m = new ConcurrentSkipListMap<String, Integer>();
        m.put("a", 1);
        var e = ent(Collections.unmodifiableMap(m), "a");
        m.put("a", 5);
        return e.getValue() + "|" + e;
    }

    /** A `HashMap` node behind a view reads live, as the node does; `setValue` throws. */
    public static String hashMapView() {
        var m = new HashMap<String, Integer>();
        m.put("a", 1);
        var e = ent(Collections.unmodifiableMap(m), "a");
        m.put("a", 5);
        String thrown;
        try {
            e.setValue(9);
            thrown = "no";
        } catch (UnsupportedOperationException x) {
            thrown = "UOE";
        }
        return e.getValue() + "|" + e + "|" + thrown + "|" + m;
    }

    public static void main(String[] args) throws Exception {
        for (var m : Probe.class.getDeclaredMethods()) {
            if (m.getParameterCount() == 0 && m.getReturnType() == String.class
                    && java.lang.reflect.Modifier.isPublic(m.getModifiers())) {
                System.out.println(m.getName() + "=" + m.invoke(null));
            }
        }
    }
}
