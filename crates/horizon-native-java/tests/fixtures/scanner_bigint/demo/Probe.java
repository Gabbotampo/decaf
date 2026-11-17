package demo;
import java.util.*; import java.math.*;
public final class Probe { public static String rows(){
StringBuilder s=new StringBuilder(); for(String x:new String[]{"","a","a\n","a\r\n","a\r","a\n\n"}){Scanner z=new Scanner(x); s.append(z.hasNextLine()).append(':'); while(z.hasNextLine())s.append('[').append(z.nextLine()).append(']'); s.append('|');}
Scanner z=new Scanner("a b\n"); s.append(z.hasNextLine()).append(':').append(z.next()).append(':').append(z.hasNextLine()).append(':').append(z.nextLine()).append('|');
for(String x:new String[]{"-1","2147483648","2147483647","9223372036854775808","-9223372036854775809"})s.append(new BigInteger(x).intValue()).append('|'); return s.toString(); }}
