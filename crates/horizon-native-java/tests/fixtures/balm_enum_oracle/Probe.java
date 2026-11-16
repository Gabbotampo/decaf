public class Probe {
 enum Plain { A, B }
 enum Body { A { }, B }
 static void p(Enum<?> e) { Class<?> c=e.getClass(), d=e.getDeclaringClass(); System.out.println(e+"|"+c.getName()+"|"+d.getName()+"|"+(c==d)+"|"+c.isEnum()+"|"+c.getEnclosingClass().getName()); }
 public static void main(String[] x){ p(Plain.A); p(Body.A); p(Body.B); System.out.println(java.util.Arrays.toString(Body.class.getEnumConstants())); }
}
