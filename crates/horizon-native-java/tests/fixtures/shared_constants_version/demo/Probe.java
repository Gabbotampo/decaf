package demo;
import net.minecraft.SharedConstants;
import net.minecraft.WorldVersion;
public final class Probe {
 public static String rows() {
  WorldVersion v=SharedConstants.getCurrentVersion();
  StringBuilder o=new StringBuilder();
  o.append("name|").append(v.name()).append('\n');
  o.append("same|").append(v==SharedConstants.getCurrentVersion()).append('\n');
  return o.toString();
 }
}
