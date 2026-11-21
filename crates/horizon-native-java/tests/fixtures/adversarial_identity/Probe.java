package demo;
import java.util.*;
import java.util.function.*;
public class Probe {
  static final IdentityHashMap<String,Integer> shared = new IdentityHashMap<>();
  static String result(String s) { return s; }
  static void helper(IdentityHashMap<String,Integer> m, int v) { m.put(new String("x"), v); }
  static void computeHelper(IdentityHashMap<String,Integer> m, int v) { m.computeIfAbsent(new String("x"), k -> v); }
  static class A { static void add(IdentityHashMap<String,Integer> m) { m.put(new String("x"), 1); } }
  static class B { static void add(IdentityHashMap<String,Integer> m) { m.put(new String("x"), 2); } }
  static void rec(IdentityHashMap<String,Integer> m, int n) { m.put(new String("x"), n); if (n > 0) rec(m,n-1); }
  static void overloaded(IdentityHashMap<String,Integer> m, int v) { m.put(new String("x"), v); }
  static void overloaded(IdentityHashMap<String,Integer> m, long v) { m.put(new String("x"), (int)v); }
  static String fieldCase() { IdentityHashMap<String,Integer> m = new IdentityHashMap<>(); String[] a = new String[1]; String x = new String("x"); m.put(x,1); a[0]=x; m.put(a[0],2); return m.size()+":"+m.get(x)+":"+m.get(a[0]); }
  static String listCase() { IdentityHashMap<String,Integer> m = new IdentityHashMap<>(); List<String> l = new ArrayList<>(); String x = new String("x"); m.put(x,1); l.add(x); m.put(l.get(0),2); return m.size()+":"+m.get(x)+":"+m.get(l.get(0)); }
  static String identityCmpHelper() { String a = helperString(); String b = helperString(); return String.valueOf(a == b); }
  static String helperString() { return new String("x"); }
  static String identityCmpRec(int n) { String a = new String("x"); if (n == 0) return String.valueOf(a == identityCmpRecString()); return identityCmpRec(n-1); }
  static String identityCmpRecString() { return new String("x"); }
  static String lambdaCase() { IdentityHashMap<String,Integer> m = new IdentityHashMap<>(); Consumer<Integer> c = v -> m.put(new String("x"),v); c.accept(1); c.accept(2); return m.size()+":"+m.getOrDefault("x",-1); }
  static IdentityHashMap<String,Integer> initMap = init();
  static IdentityHashMap<String,Integer> init() { IdentityHashMap<String,Integer> m = new IdentityHashMap<>(); helper(m,1); helper(m,2); return m; }
  static String sameClassCalls() { IdentityHashMap<String,Integer> m = new IdentityHashMap<>(); helper(m,1); helper(m,2); return m.size()+":"+m.getOrDefault("x",-1); }
  static String computeCalls() { IdentityHashMap<String,Integer> m = new IdentityHashMap<>(); computeHelper(m,1); computeHelper(m,2); return m.size()+":"+m.getOrDefault("x",-1); }
  static String classCalls() { IdentityHashMap<String,Integer> m = new IdentityHashMap<>(); A.add(m); B.add(m); return String.valueOf(m.size()); }
  static String recursion() { IdentityHashMap<String,Integer> m = new IdentityHashMap<>(); rec(m,1); return m.size()+":"+m.getOrDefault("x",-1); }
  static String overloadCalls() { IdentityHashMap<String,Integer> m = new IdentityHashMap<>(); overloaded(m,1); overloaded(m,2L); return String.valueOf(m.size()); }
  static String staticInit() { return initMap.size()+":"+initMap.getOrDefault("x",-1); }
  static String recCmp(int n) { String a = new String("x"); if (n > 0) return recCmp(n-1); String b = new String("x"); return String.valueOf(a == b); }
  static String lambdaIdentityCmp() { Supplier<String> s = () -> new String("x"); String a=s.get(), b=s.get(); return String.valueOf(a==b); }
  static String loopAlloc() {
    IdentityHashMap<String,Integer> m = new IdentityHashMap<>();
    for (int i = 0; i < 2; i++) {
      m.put(new String("x"), i);
    }
    return m.size() + ":" + m.getOrDefault("x", -1);
  }
  static String stringValueOf() {
    String a = new String("x");
    IdentityHashMap<String,Integer> m = new IdentityHashMap<>();
    m.put(a, 1);
    return String.valueOf(m.get(String.valueOf(a)));
  }
  static String wholeSubstring() {
    String a = new String("x");
    String b = a.substring(0);
    IdentityHashMap<String,Integer> m = new IdentityHashMap<>();
    m.put(a, 1);
    return (a == b) + ":" + m.get(b);
  }
  static String callbackMutation() {
    String a = new String("x");
    String b = new String("x");
    IdentityHashMap<String,Integer> m = new IdentityHashMap<>();
    m.put("z", 1);
    m.put(a, 2);
    Integer n = m.computeIfAbsent(b, k -> {
      m.remove("z");
      return 3;
    });
    return n + ":" + m.get(a) + ":" + m.get(b) + ":" + m.size();
  }
  static String computeValueOverwrite() {
    String v1 = new String("v");
    IdentityHashMap<String,String> m = new IdentityHashMap<>();
    m.put("k", v1);
    m.compute("k", (k, old) -> new String("v"));
    return String.valueOf(m.remove("k", v1));
  }
  static class InternA {
    static final IdentityHashMap<String,Integer> M = new IdentityHashMap<>();
    static String S;
    static String make() {
      String s = new StringBuilder("xy").append("zq").toString();
      boolean self = (s.intern() == s);
      S = s;
      M.put(s, 1);
      return "A intern self=" + self;
    }
  }
  static class InternB {
    static String lookup() {
      Integer v = InternA.M.get("xyzq");
      boolean same = (InternA.S == "xyzq");
      return "B lookup=" + v + " same=" + same;
    }
  }
  static String dynamicIntern() {
    return InternA.make() + "|" + InternB.lookup();
  }
  public static String sameClassCallsOut(){return sameClassCalls();}
  public static String computeCallsOut(){return computeCalls();}
  public static String classCallsOut(){return classCalls();}
  public static String recursionOut(){return recursion();}
  public static String overloadCallsOut(){return overloadCalls();}
  public static String staticInitOut(){return staticInit();}
  public static String fieldCaseOut(){return fieldCase();}
  public static String listCaseOut(){return listCase();}
  public static String lambdaCaseOut(){return lambdaCase();}
  public static String identityCmpHelperOut(){return identityCmpHelper();}
  public static String lambdaIdentityCmpOut(){return lambdaIdentityCmp();}
  public static String recCmpOut(){return recCmp(1);}
  public static String loopAllocOut(){return loopAlloc();}
  public static String stringValueOfOut(){return stringValueOf();}
  public static String wholeSubstringOut(){return wholeSubstring();}
  public static String callbackMutationOut(){return callbackMutation();}
  public static String computeValueOverwriteOut(){return computeValueOverwrite();}
  public static String dynamicInternOut(){return dynamicIntern();}
}
