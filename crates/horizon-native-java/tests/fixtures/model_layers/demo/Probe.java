package demo;
import net.minecraft.client.model.geom.PartPose;
import net.minecraft.client.model.geom.builders.*;
public final class Probe {
  public static String run() {
    MeshDefinition mesh = new MeshDefinition();
    PartDefinition root = mesh.getRoot();
    CubeDeformation grow = new CubeDeformation(0.25f).extend(0.5f);
    CubeListBuilder builder = CubeListBuilder.create().texOffs(2, 3).mirror()
        .addBox(-1f, -2f, -3f, 4f, 5f, 6f, grow, 2f, 3f);
    PartPose pose = PartPose.offsetAndRotation(1.25f, -2.5f, 3.75f, .1f, .2f, .3f);
    PartDefinition head = root.addOrReplaceChild("head", builder, pose);
    LayerDefinition layer = LayerDefinition.create(mesh, 32, 16);
    return (builder == builder.mirror(false)) + ","
        + "head" + "," + pose.x() + "," + pose.yRot() + ","
        + pose + "," + (layer != null);
  }
}
