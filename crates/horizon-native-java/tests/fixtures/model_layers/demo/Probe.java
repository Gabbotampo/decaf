package demo;
import java.util.Set;
import net.minecraft.client.model.geom.PartPose;
import net.minecraft.client.model.geom.builders.*;
public final class Probe {
  public static String run() {
    MeshDefinition mesh = new MeshDefinition();
    PartDefinition root = mesh.getRoot();
    CubeDeformation grow = new CubeDeformation(0.25f).extend(0.5f);
    CubeDeformation tri = new CubeDeformation(0.1f).extend(0.2f, 0.3f, 0.4f);
    CubeListBuilder builder = CubeListBuilder.create().texOffs(2, 3).mirror()
        .addBox(-1f, -2f, -3f, 4f, 5f, 6f, grow, 2f, 3f);
    CubeListBuilder visibleSet = CubeListBuilder.create()
        .addBox(0f, 0f, 0f, 1f, 1f, 1f, Set.of());
    CubeListBuilder explicitMirror = CubeListBuilder.create()
        .addBox(0f, 0f, 0f, 1f, 1f, 1f, true);
    PartPose pose = PartPose.offsetAndRotation(1.25f, -2.5f, 3.75f, .1f, .2f, .3f);
    PartDefinition head = root.addOrReplaceChild("head", builder, pose);
    head.addOrReplaceChild("grand", CubeListBuilder.create(), PartPose.ZERO);
    PartDefinition replacement = root.addOrReplaceChild("head", CubeListBuilder.create(), PartPose.ZERO);
    CubeListBuilder live = CubeListBuilder.create();
    root.addOrReplaceChild("live", live, PartPose.ZERO);
    live.addBox(0f, 0f, 0f, 1f, 1f, 1f);
    PartPose translated = PartPose.offset(.1f, .2f, .3f).translated(.2f, .3f, .4f);
    LayerDefinition layer = LayerDefinition.create(mesh, 32, 16);
    return (builder == builder.mirror(false)) + ","
        + (tri != null) + ","
        + (visibleSet == visibleSet.mirror(false)) + ","
        + (explicitMirror == explicitMirror.mirror(false)) + ","
        + "head" + "," + pose.x() + "," + pose.yRot() + ","
        + pose + "," + (head != replacement) + ","
        + (replacement.getChild("grand") != null) + ","
        + (root.getChild("missing") == null) + ","
        + translated + "," + (layer != null);
  }
  public static void main(String[] args) { System.out.print(run()); }
}
