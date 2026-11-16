//! Java 25/client-26.3 oracle for pure model-layer construction.
#[path = "support/javac_fixture.rs"]
mod javac_fixture;
use horizon_native_java::StdHost;
use javac_fixture::Fixture;

const PROBE: &str = include_str!("fixtures/model_layers/demo/Probe.java");
const ORACLE: &str = include_str!("fixtures/model_layers/java25.txt");
const STUBS: &[(&str, &str)] = &[
    (
        "net/minecraft/client/model/geom/PartPose.java",
        "package net.minecraft.client.model.geom; public final class PartPose { public static final PartPose ZERO = null; public static PartPose offset(float a,float b,float c){return null;} public static PartPose offsetAndRotation(float a,float b,float c,float d,float e,float f){return null;} public PartPose translated(float a,float b,float c){return null;} public float x(){return 0;} public float yRot(){return 0;} public String toString(){return null;} }",
    ),
    (
        "net/minecraft/client/model/geom/builders/CubeDeformation.java",
        "package net.minecraft.client.model.geom.builders; public class CubeDeformation { public static final CubeDeformation NONE = null; public CubeDeformation(float x){} public CubeDeformation extend(float x){return null;} public CubeDeformation extend(float x,float y,float z){return null;} public String toString(){return null;} }",
    ),
    (
        "net/minecraft/client/model/geom/builders/CubeListBuilder.java",
        "package net.minecraft.client.model.geom.builders; public class CubeListBuilder { public static CubeListBuilder create(){return null;} public CubeListBuilder texOffs(int x,int y){return null;} public CubeListBuilder mirror(){return null;} public CubeListBuilder mirror(boolean b){return null;} public CubeListBuilder addBox(float a,float b,float c,float d,float e,float f){return null;} public CubeListBuilder addBox(float a,float b,float c,float d,float e,float f,CubeDeformation g,float h,float i){return null;} public CubeListBuilder addBox(float a,float b,float c,float d,float e,float f,java.util.Set<?> faces){return null;} public CubeListBuilder addBox(float a,float b,float c,float d,float e,float f,boolean mirror){return null;} public java.util.List<Object> getCubes(){return null;} }",
    ),
    (
        "net/minecraft/client/model/geom/builders/MeshDefinition.java",
        "package net.minecraft.client.model.geom.builders; public class MeshDefinition { public MeshDefinition(){} public PartDefinition getRoot(){return null;} }",
    ),
    (
        "net/minecraft/client/model/geom/builders/PartDefinition.java",
        "package net.minecraft.client.model.geom.builders; import net.minecraft.client.model.geom.PartPose; public class PartDefinition { public PartDefinition addOrReplaceChild(String n,CubeListBuilder b,PartPose p){return null;} public PartDefinition addOrReplaceChild(String n,PartDefinition p){return null;} public PartDefinition getChild(String n){return null;} }",
    ),
    (
        "net/minecraft/client/model/geom/builders/LayerDefinition.java",
        "package net.minecraft.client.model.geom.builders; public class LayerDefinition { public static LayerDefinition create(MeshDefinition m,int w,int h){return null;} }",
    ),
];

fn fixture() -> Fixture {
    let mut sources = STUBS.to_vec();
    sources.push(("demo/Probe.java", PROBE));
    Fixture::build("model-layers", &sources)
}

#[test]
fn model_layer_construction_matches_java25_oracle() {
    let fixture = fixture();
    let mut host = fixture
        .jar
        .seed_host(StdHost::new().with_environment_type("CLIENT"));
    assert_eq!(fixture.text_on(&mut host, "run"), ORACLE.trim_end());
    assert_eq!(host.unknown_host_calls(), Vec::<(String, usize)>::new());
}
