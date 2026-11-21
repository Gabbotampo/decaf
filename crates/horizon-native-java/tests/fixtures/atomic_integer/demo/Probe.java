package demo;
import java.util.concurrent.atomic.AtomicInteger;
public class Probe {
  public static String rows() {
    AtomicInteger a = new AtomicInteger();
    AtomicInteger b = new AtomicInteger(41);
    StringBuilder s = new StringBuilder();
    s.append(a.get()).append('|').append(b.get()).append('|');
    s.append(a.getAndIncrement()).append(':').append(a.get()).append('|');
    s.append(a.incrementAndGet()).append(':').append(a.get()).append('|');
    s.append(b.incrementAndGet()).append('|');
    a.set(-5); s.append(a.get()).append('|');
    AtomicInteger max = new AtomicInteger(Integer.MAX_VALUE);
    s.append(max.getAndIncrement()).append(':').append(max.get()).append('|');
    s.append(max.incrementAndGet()).append('|');
    s.append(b.intValue()).append(':').append(max.longValue());
    return s.toString();
  }
  public static void main(String[] args) { System.out.print(rows()); }
}
