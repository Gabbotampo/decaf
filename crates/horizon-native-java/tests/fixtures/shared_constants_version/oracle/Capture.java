import net.minecraft.SharedConstants;
import net.minecraft.WorldVersion;
public final class Capture { public static void main(String[] a) { SharedConstants.tryDetectVersion(); WorldVersion v=SharedConstants.getCurrentVersion(); System.out.print("name|"+v.name()+"\n"); System.out.print("same|"+(v==SharedConstants.getCurrentVersion())+"\n"); } }
