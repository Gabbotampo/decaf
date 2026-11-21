//! Small contract checks for the Sunrise-facing client adapters.

#[test]
fn creative_features_use_exact_vanilla_masks_and_refuse_unknowns() {
    assert_eq!(
        crate::runtime::creative_feature_mask(&["minecraft:vanilla".into()]),
        Some(1)
    );
    assert_eq!(
        crate::runtime::creative_feature_mask(&[
            "minecraft:vanilla".into(),
            "minecraft:trade_rebalance".into(),
        ]),
        Some(3)
    );
    assert_eq!(
        crate::runtime::creative_feature_mask(&["example:future".into()]),
        None
    );
}
