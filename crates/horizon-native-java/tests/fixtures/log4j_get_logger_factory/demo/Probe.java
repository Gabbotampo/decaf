package demo;
import org.apache.logging.log4j.*; import org.apache.logging.log4j.message.*;
public class Probe {
 static final class A extends AbstractMessageFactory { public Message newMessage(String s){return new SimpleMessage(s);} public Message newMessage(String s,Object... a){return new ParameterizedMessage(s,a);} }
 static final class B extends FormattedMessageFactory { public Message newMessage(String s){return new SimpleMessage(s);} }
 static String row(MessageFactory f1, MessageFactory f2){ Logger a=LogManager.getLogger("x",f1), b=LogManager.getLogger("x",f1), c=LogManager.getLogger("x",f2); return (a.getMessageFactory()==f1)+":"+(a==b)+":"+(a!=c); }
 public static String rows(){ A a1=new A(),a2=new A(); B b1=new B(),b2=new B(); return row(a1,a2)+"|"+row(b1,b2); }
 public static void main(String[] x){System.out.print(rows());}
}
