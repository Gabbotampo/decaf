package demo;

import java.util.ArrayDeque;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.HashSet;
import java.util.LinkedHashMap;
import java.util.Vector;

public final class Probe {
    static final int B = 1 << 24;

    static String error(Throwable error) {
        return error.getClass().getName() + ":" + error.getMessage();
    }

    public static String zero() {
        new ArrayList<Object>(0);
        new HashMap<Object, Object>(0);
        new HashMap<Object, Object>(0, .75f);
        new HashSet<Object>(0);
        new LinkedHashMap<Object, Object>(0);
        new LinkedHashMap<Object, Object>(0, .75f);
        new LinkedHashMap<Object, Object>(0, .75f, true);
        new ArrayDeque<Object>(0);
        new StringBuilder(0);
        new Vector<Object>(0);
        new ArrayList<Object>().ensureCapacity(0);
        new Vector<Object>().ensureCapacity(0);
        new StringBuilder().ensureCapacity(0);
        return "ok";
    }

    public static String negatives() {
        StringBuilder out = new StringBuilder();
        try { new ArrayList<Object>(-1); out.append("ok"); }
        catch (RuntimeException e) { out.append(error(e)); }
        out.append("|");
        try { new HashMap<Object, Object>(-1); out.append("ok"); }
        catch (RuntimeException e) { out.append(error(e)); }
        out.append("|");
        try { new HashMap<Object, Object>(-1, .75f); out.append("ok"); }
        catch (RuntimeException e) { out.append(error(e)); }
        out.append("|");
        try { new HashSet<Object>(-1); out.append("ok"); }
        catch (RuntimeException e) { out.append(error(e)); }
        out.append("|");
        try { new LinkedHashMap<Object, Object>(-1); out.append("ok"); }
        catch (RuntimeException e) { out.append(error(e)); }
        out.append("|");
        try { new LinkedHashMap<Object, Object>(-1, .75f); out.append("ok"); }
        catch (RuntimeException e) { out.append(error(e)); }
        out.append("|");
        try { new LinkedHashMap<Object, Object>(-1, .75f, true); out.append("ok"); }
        catch (RuntimeException e) { out.append(error(e)); }
        out.append("|");
        try { new ArrayDeque<Object>(-1); out.append("ok"); }
        catch (RuntimeException e) { out.append(error(e)); }
        out.append("|");
        try { new StringBuilder(-1); out.append("ok"); }
        catch (RuntimeException e) { out.append(error(e)); }
        out.append("|");
        try { new Vector<Object>(-1); out.append("ok"); }
        catch (RuntimeException e) { out.append(error(e)); }
        out.append("|");
        try { new ArrayList<Object>().ensureCapacity(-1); out.append("ok"); }
        catch (RuntimeException e) { out.append(error(e)); }
        out.append("|");
        try { new Vector<Object>().ensureCapacity(-1); out.append("ok"); }
        catch (RuntimeException e) { out.append(error(e)); }
        out.append("|");
        try { new StringBuilder().ensureCapacity(-1); out.append("ok"); }
        catch (RuntimeException e) { out.append(error(e)); }
        return out.toString();
    }

    public static String arrayListBound() { new ArrayList<Object>(B); return "ok"; }
    public static String arrayListOver() { new ArrayList<Object>(B + 1); return "ok"; }
    public static String arrayListMax() { new ArrayList<Object>(Integer.MAX_VALUE); return "ok"; }

    public static String hashMapBound() { new HashMap<Object, Object>(B); return "ok"; }
    public static String hashMapOver() { new HashMap<Object, Object>(B + 1); return "ok"; }
    public static String hashMapMax() { new HashMap<Object, Object>(Integer.MAX_VALUE); return "ok"; }
    public static String hashMapFloatBound() { new HashMap<Object, Object>(B, .75f); return "ok"; }
    public static String hashMapFloatOver() { new HashMap<Object, Object>(B + 1, .75f); return "ok"; }
    public static String hashMapFloatMax() { new HashMap<Object, Object>(Integer.MAX_VALUE, .75f); return "ok"; }
    public static String hashMapFirstPut() { HashMap<Object, Object> map = new HashMap<Object, Object>(Integer.MAX_VALUE); map.put("k", "v"); return "ok"; }

    public static String hashSetBound() { new HashSet<Object>(B); return "ok"; }
    public static String hashSetOver() { new HashSet<Object>(B + 1); return "ok"; }
    public static String hashSetMax() { new HashSet<Object>(Integer.MAX_VALUE); return "ok"; }
    public static String hashSetFirstPut() { HashSet<Object> set = new HashSet<Object>(Integer.MAX_VALUE); set.add("v"); return "ok"; }

    public static String linkedHashMapBound() { new LinkedHashMap<Object, Object>(B); return "ok"; }
    public static String linkedHashMapOver() { new LinkedHashMap<Object, Object>(B + 1); return "ok"; }
    public static String linkedHashMapMax() { new LinkedHashMap<Object, Object>(Integer.MAX_VALUE); return "ok"; }
    public static String linkedHashMapFloatBound() { new LinkedHashMap<Object, Object>(B, .75f); return "ok"; }
    public static String linkedHashMapFloatOver() { new LinkedHashMap<Object, Object>(B + 1, .75f); return "ok"; }
    public static String linkedHashMapFloatMax() { new LinkedHashMap<Object, Object>(Integer.MAX_VALUE, .75f); return "ok"; }
    public static String linkedHashMapOrderBound() { new LinkedHashMap<Object, Object>(B, .75f, true); return "ok"; }
    public static String linkedHashMapOrderOver() { new LinkedHashMap<Object, Object>(B + 1, .75f, true); return "ok"; }
    public static String linkedHashMapOrderMax() { new LinkedHashMap<Object, Object>(Integer.MAX_VALUE, .75f, true); return "ok"; }
    public static String linkedHashMapFirstPut() { LinkedHashMap<Object, Object> map = new LinkedHashMap<Object, Object>(Integer.MAX_VALUE); map.put("k", "v"); return "ok"; }

    public static String dequeUnderBound() { new ArrayDeque<Object>(B - 1); return "ok"; }
    public static String dequeBound() { new ArrayDeque<Object>(B); return "ok"; }
    public static String dequeOver() { new ArrayDeque<Object>(B + 1); return "ok"; }
    public static String dequeMax() { new ArrayDeque<Object>(Integer.MAX_VALUE); return "ok"; }

    public static String stringBuilderBound() { new StringBuilder(B); return "ok"; }
    public static String stringBuilderOver() { new StringBuilder(B + 1); return "ok"; }
    public static String stringBuilderMax() { new StringBuilder(Integer.MAX_VALUE); return "ok"; }

    public static String vectorBound() { new Vector<Object>(B); return "ok"; }
    public static String vectorOver() { new Vector<Object>(B + 1); return "ok"; }
    public static String vectorMax() { new Vector<Object>(Integer.MAX_VALUE); return "ok"; }

    public static String ensureArrayListBound() { new ArrayList<Object>().ensureCapacity(B); return "ok"; }
    public static String ensureArrayListOver() { new ArrayList<Object>().ensureCapacity(B + 1); return "ok"; }
    public static String ensureArrayListMax() { new ArrayList<Object>().ensureCapacity(Integer.MAX_VALUE); return "ok"; }
    public static String ensureVectorBound() { new Vector<Object>().ensureCapacity(B); return "ok"; }
    public static String ensureVectorOver() { new Vector<Object>().ensureCapacity(B + 1); return "ok"; }
    public static String ensureVectorMax() { new Vector<Object>().ensureCapacity(Integer.MAX_VALUE); return "ok"; }
    public static String ensureBuilderBound() { new StringBuilder().ensureCapacity(B); return "ok"; }
    public static String ensureBuilderOver() { new StringBuilder().ensureCapacity(B + 1); return "ok"; }
    public static String ensureBuilderMax() { new StringBuilder().ensureCapacity(Integer.MAX_VALUE); return "ok"; }

    public static String ensureArrayListGrowth() { new ArrayList<Object>(B - 1).ensureCapacity(B); return "ok"; }
    public static String ensureVectorGrowth() { new Vector<Object>(B - 1).ensureCapacity(B); return "ok"; }
    public static String ensureBuilderGrowth() { new StringBuilder(B - 1).ensureCapacity(B); return "ok"; }
}
