package demo;
import org.lwjgl.system.MathUtil;
import org.apache.logging.log4j.util.Strings;
import org.apache.commons.compress.utils.Lists;
public final class Probe {
 public static String bits(){ return MathUtil.mathIsPoT(0)+"|"+MathUtil.mathIsPoT(1)+"|"+MathUtil.mathIsPoT(Integer.MIN_VALUE)+"|"+MathUtil.mathIsPoT(-1)+"|"+MathUtil.mathRoundPoT(0)+"|"+MathUtil.mathRoundPoT(1)+"|"+MathUtil.mathRoundPoT(2)+"|"+MathUtil.mathRoundPoT(3)+"|"+MathUtil.mathRoundPoT(5)+"|"+MathUtil.mathRoundPoT(Integer.MAX_VALUE); }
 public static String strings(){ return Strings.isEmpty(null)+"|"+Strings.isEmpty("")+"|"+Strings.isEmpty("x")+"|"+Strings.isNotEmpty(null)+"|"+Strings.isNotEmpty("x")+"|"+Strings.isBlank(null)+"|"+Strings.isBlank(" \t")+"|"+Strings.isNotBlank(" "); }
 public static String list(){ java.util.ArrayList<Object> x=Lists.newArrayList(); x.add("x"); return (x!=null)+"|"+x.size(); }
}
