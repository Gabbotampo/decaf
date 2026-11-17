package demo;
import java.util.*;
public class Probe {
  public static final class Named { public String toString(){ return "named"; } }
  public static final class Nully { public String toString(){ return null; } }
  public static final class Boom { public String toString(){ throw new IllegalStateException("boom"); } }
  public static final class Fmt implements Formattable { public void formatTo(Formatter f,int flags,int width,int precision){ f.format("fmt"); } }
  public static String rows() {
    Named n = new Named();
    return String.join("\n",
      "[".concat("%s".formatted(n)).concat("]"),
      String.format("%-8s|%.3s|%S", n, n, n),
      "%s %s".formatted("host", n),
      "[%s]".formatted((Object)null),
      "[%s]".formatted(new Nully()));
  }
  public static String boom() { try { "[%s]".formatted(new Boom()); return "ok"; } catch (Throwable t) { return t.getClass().getName()+":"+t.getMessage(); } }
  public static String fmt() { try { return "[%s]".formatted(new Fmt()); } catch (Throwable t) { return t.getClass().getName()+":"+t.getMessage(); } }
  public static String mixed() { try { return String.format("%d %s", new Named(), new Named()); } catch (Throwable t) { return t.getClass().getName()+":"+t.getMessage(); } }
  public static void main(String[] a) { System.out.print(rows()); }
}
