use horizon_native_java::{Host, MethodRef, StdHost, Value};
#[test]
fn tool_material_getstatic_and_accessors_match_capture() {
    let mut h = StdHost::new();
    let field = MethodRef {
        owner: "net/minecraft/world/item/ToolMaterial".into(),
        name: "WOOD".into(),
        descriptor: "Lnet/minecraft/world/item/ToolMaterial;".into(),
    };
    let material = h.get_static(&field).unwrap();
    for (name, desc, expected) in [
        ("durability", "()I", Value::Int(59)),
        ("speed", "()F", Value::Float(2.0)),
        ("attackDamageBonus", "()F", Value::Float(0.0)),
        ("enchantmentValue", "()I", Value::Int(15)),
    ] {
        let call = MethodRef {
            owner: field.owner.clone(),
            name: name.into(),
            descriptor: desc.into(),
        };
        assert_eq!(h.invoke_instance(&call, &material, &[]).unwrap(), expected);
    }
}
