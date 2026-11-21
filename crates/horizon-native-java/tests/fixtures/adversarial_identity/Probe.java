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
}
