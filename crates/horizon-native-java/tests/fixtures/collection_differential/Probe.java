package demo;
import java.util.*;
import java.util.function.*;
public class Probe {
  static List<Integer> make(int r) {
    int family = r / 3, depth = r % 3;
    List<Integer> x;
    switch (family) {
      case 0: x = new ArrayList<>(List.of(1,2,3,4)); break;
      case 1: x = new LinkedList<>(List.of(1,2,3,4)); break;
      case 2: x = Arrays.asList(1,2,3,4); break;
      case 3: x = List.of(); break;
      case 4: x = List.of(1); break;
      case 5: x = List.of(1,2); break;
      case 6: x = List.of(1,2,3,4); break;
      case 7: x = Collections.singletonList(1); break;
      case 8: x = Collections.nCopies(0, 1); break;
      case 9: x = Collections.nCopies(4, 1); break;
      case 10: x = Collections.emptyList(); break;
      case 11: x = Collections.unmodifiableList(new ArrayList<>(List.of(1,2,3,4))); break;
      case 12: x = Collections.synchronizedList(new ArrayList<>(List.of(1,2,3,4))); break;
      default: throw new AssertionError(family);
    }
    for (int d=0; d<depth; d++) {
      int n=x.size();
      x=x.subList(n == 0 ? 0 : 1, n == 0 ? 0 : Math.min(n, 3));
    }
    return x;
  }
  static String safeList(List<Integer> x) {
    try { return x.toString(); }
    catch (Throwable t) { return "<"+t.getClass().getName()+":"+t.getMessage()+">"; }
  }
  static String result(String prefix, List<Integer> x) { return prefix+"|L="+safeList(x); }
  static String run(int r, int op) {
    List<Integer> x=make(r);
    try {
      switch (op) {
        case 0: return result("R="+x.add(9), x);
        case 1: x.add(x.size(),9); return result("R=void", x);
        case 2: return result("R="+x.set(0,9), x);
        case 3: return result("R="+x.remove(0), x);
        case 4: return result("R="+x.remove(Integer.valueOf(99)), x);
        case 5: return result("R="+x.remove(Integer.valueOf(1)), x);
        case 6: x.clear(); return result("R=void", x);
        case 7: x.sort(null); return result("R=void", x);
        case 8: Collections.sort(x); return result("R=void", x);
        case 9: x.replaceAll(v -> v+10); return result("R=void", x);
        case 10: return result("R="+x.removeIf(v -> v%2==0), x);
        case 11: return result("R="+x.addAll(List.of(7,8)), x);
        case 12: return result("R="+x.addAll(x.size(),List.of(7,8)), x);
        case 13: return result("R="+x.retainAll(List.of(1,2)), x);
        case 14: return result("R="+x.removeAll(List.of(2,99)), x);
        case 15: return result("R="+x.contains(null), x);
        case 16: return result("R="+x.indexOf(null), x);
        case 17: return result("R="+x.get(x.size()), x);
        case 18: { Iterator<Integer> i=x.iterator(); i.next(); i.remove(); return result("R=void", x); }
        case 19: { ListIterator<Integer> i=x.listIterator(); i.next(); i.set(9); return result("R=void", x); }
        case 20: { ListIterator<Integer> i=x.listIterator(); i.add(9); return result("R=void", x); }
        case 21: return result("R="+x.isEmpty(), x);
        case 22: return result("R="+x.size(), x);
        case 23: return result("R="+x.contains(1), x);
        case 24: return result("R="+x.indexOf(1), x);
        case 25: return result("R="+x.lastIndexOf(1), x);
        case 26: return result("R="+x.equals(Set.of(1,2)), x);
        case 27: return result("R="+x.hashCode(), x);
        case 28: return result("R="+x.toArray().length, x);
        case 29: return result("R="+x.toString(), x);
        case 30: { List<Integer> n=Collections.nCopies(Integer.MAX_VALUE, 1); return "R="+n.size()+"|L="+n.get(Integer.MAX_VALUE-1); }
        case 31: try { Collections.nCopies(-1, 1); return "R=bad|L="+safeList(x); }
          catch (Throwable t) { return result("E="+t.getClass().getName()+":"+t.getMessage(), x); }
        default: throw new AssertionError(op);
      }
    } catch (Throwable t) { return result("E="+t.getClass().getName()+":"+t.getMessage(), x); }
  }
  public static String probe(int r, int op) { return run(r,op); }
  public static void main(String[] a) {
    for (int r=0;r<39;r++) for (int op=0;op<32;op++) System.out.println(r+","+op+"="+run(r,op));
  }
}
