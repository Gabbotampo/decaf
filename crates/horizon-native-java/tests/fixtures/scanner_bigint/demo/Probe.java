package demo;
import java.util.*; import java.math.*; import java.io.*; import java.nio.file.*;
public final class Probe { public static String rows(){StringBuilder s=new StringBuilder();
try{Path p=Path.of("sc470.txt"); Files.writeString(p,"a\r\nb\r\nc",java.nio.charset.StandardCharsets.UTF_8); Scanner z=new Scanner(p.toFile()); s.append(z.hasNextLine()).append(':').append(z.nextLine()).append('|').append(z.nextLine()).append('|').append(z.nextLine()).append('|').append(z.hasNextLine()); try{z.nextLine();}catch(Exception e){s.append('|').append(e.getClass().getSimpleName()).append(':').append(e.getMessage());} z.close(); try{z.hasNextLine();}catch(Exception e){s.append('|').append(e.getClass().getSimpleName()).append(':').append(e.getMessage());} Files.delete(p);}catch(Exception e){s.append(e);}
try{new Scanner(new File("/definitely/missing/scanner470"));}catch(Exception e){s.append('|').append(e.getClass().getSimpleName()).append(':').append(e.getMessage());}
try{Path p=Path.of("sc470e.txt"); Files.write(p,new byte[0]); s.append('|').append(new Scanner(p.toFile()).hasNextLine()); Files.delete(p);}catch(Exception e){s.append('|').append(e);}
return s.toString();}}
