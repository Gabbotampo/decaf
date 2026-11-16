import net.minecraft.SharedConstants;
import net.minecraft.server.Bootstrap;
import demo.Probe;
public final class Oracle {
  public static void main(String[] args) {
    SharedConstants.tryDetectVersion();
    Bootstrap.bootStrap();
    System.out.println("tags=" + Probe.tags());
    System.out.println("categories=" + Probe.categories());
    System.out.println("attrs=" + Probe.attrs());
    System.out.println("invalid=" + Probe.invalid());
    System.out.println("attrsEdges=" + Probe.attrsEdges());
  }
}
