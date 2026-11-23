//! Which class of 26.3's block hierarchy declares `useItemOn`, `useWithoutItem` and
//! `getMenuProvider` (#629 review P1 3).
//!
//! A mod block that doesn't override one of them inherits the body of the nearest vanilla
//! class on its superclass chain that declares it. Decaf models only `BlockBehaviour`'s own
//! bodies (`TRY_WITH_EMPTY_HAND`, `PASS`, `null`), so it answers an inherited call only when
//! that nearest declaring class is `BlockBehaviour`. `FurnaceBlock` inherits `useItemOn` from
//! `BlockBehaviour` (answered) but `useWithoutItem` from `AbstractFurnaceBlock`, which opens
//! the furnace's menu (counted).
//!
//! Generated from the 26.3 server jar's class files by
//! `java tools/VanillaBlockUseMethodsExporter.java server-26.3.jar` (JDK 25): every class that
//! extends `BlockBehaviour` (itself included), its superclass, and which of the three methods
//! it declares itself with the exact descriptor (static and private methods excluded). Only
//! names and declaration bits are kept, no code.

/// `useItemOn(ItemStack, BlockState, Level, BlockPos, Player, InteractionHand, BlockHitResult)`.
pub(crate) const USE_ITEM_ON: u8 = 1;
/// `useWithoutItem(BlockState, Level, BlockPos, Player, BlockHitResult)`.
pub(crate) const USE_WITHOUT_ITEM: u8 = 2;
/// `getMenuProvider(BlockState, Level, BlockPos)`.
pub(crate) const GET_MENU_PROVIDER: u8 = 4;

/// The vanilla class whose body a call of `method` (one of the bits above) on an object of
/// vanilla block class `class` runs: the nearest class on `class`'s superclass chain that
/// declares it. `None` when `class` is not one of 26.3's block classes (a library's, or no
/// class of this game), so nothing is known of what it inherits.
pub(crate) fn declaring_class(class: &str, method: u8) -> Option<&'static str> {
    let mut current = class;
    // The deepest chain in 26.3 is a handful of classes; the bound only guards a bad table.
    for _ in 0..CLASSES.len() {
        let index = CLASSES
            .binary_search_by(|(name, _, _)| name.cmp(&current))
            .ok()?;
        let (name, superclass, declared) = CLASSES[index];
        if declared & method != 0 {
            return Some(name);
        }
        if superclass.is_empty() {
            return None;
        }
        current = superclass;
    }
    None
}

/// Whether vanilla block class `class` is `ancestor` or extends it, by the table. `false` for
/// a class the table doesn't have.
pub(crate) fn extends(class: &str, ancestor: &str) -> bool {
    let mut current = class;
    for _ in 0..CLASSES.len() {
        if current == ancestor {
            return true;
        }
        let Ok(index) = CLASSES.binary_search_by(|(name, _, _)| name.cmp(&current)) else {
            return false;
        };
        current = CLASSES[index].1;
        if current.is_empty() {
            return false;
        }
    }
    false
}

/// (class, superclass, declared bits), sorted bytewise by class for the binary search.
#[rustfmt::skip]
const CLASSES: &[(&str, &str, u8)] = &[
    ("net/minecraft/world/level/block/AbstractBannerBlock", "net/minecraft/world/level/block/BaseEntityBlock", 0),
    ("net/minecraft/world/level/block/AbstractBedBlock", "net/minecraft/world/level/block/HorizontalDirectionalBlock", 2),
    ("net/minecraft/world/level/block/AbstractCandleBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/AbstractCauldronBlock", "net/minecraft/world/level/block/Block", 1),
    ("net/minecraft/world/level/block/AbstractChestBlock", "net/minecraft/world/level/block/BaseEntityBlock", 0),
    ("net/minecraft/world/level/block/AbstractFurnaceBlock", "net/minecraft/world/level/block/BaseEntityBlock", 2),
    ("net/minecraft/world/level/block/AbstractSkullBlock", "net/minecraft/world/level/block/BaseEntityBlock", 0),
    ("net/minecraft/world/level/block/AirBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/AmethystBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/AmethystClusterBlock", "net/minecraft/world/level/block/AmethystBlock", 0),
    ("net/minecraft/world/level/block/AnvilBlock", "net/minecraft/world/level/block/FallingBlock", 6),
    ("net/minecraft/world/level/block/AttachedStemBlock", "net/minecraft/world/level/block/VegetationBlock", 0),
    ("net/minecraft/world/level/block/AzaleaBlock", "net/minecraft/world/level/block/VegetationBlock", 0),
    ("net/minecraft/world/level/block/BambooSaplingBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/BambooStalkBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/BannerBlock", "net/minecraft/world/level/block/AbstractBannerBlock", 0),
    ("net/minecraft/world/level/block/BarrelBlock", "net/minecraft/world/level/block/BaseEntityBlock", 2),
    ("net/minecraft/world/level/block/BarrierBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/BaseCoralFanBlock", "net/minecraft/world/level/block/BaseCoralPlantTypeBlock", 0),
    ("net/minecraft/world/level/block/BaseCoralPlantBlock", "net/minecraft/world/level/block/BaseCoralPlantTypeBlock", 0),
    ("net/minecraft/world/level/block/BaseCoralPlantTypeBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/BaseCoralWallFanBlock", "net/minecraft/world/level/block/BaseCoralFanBlock", 0),
    ("net/minecraft/world/level/block/BaseEntityBlock", "net/minecraft/world/level/block/Block", 4),
    ("net/minecraft/world/level/block/BaseFireBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/BasePressurePlateBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/BaseRailBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/BaseTorchBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/BeaconBlock", "net/minecraft/world/level/block/BaseEntityBlock", 2),
    ("net/minecraft/world/level/block/BedBlock", "net/minecraft/world/level/block/AbstractBedBlock", 0),
    ("net/minecraft/world/level/block/BeehiveBlock", "net/minecraft/world/level/block/BaseEntityBlock", 1),
    ("net/minecraft/world/level/block/BeetrootBlock", "net/minecraft/world/level/block/CropBlock", 0),
    ("net/minecraft/world/level/block/BellBlock", "net/minecraft/world/level/block/BaseEntityBlock", 2),
    ("net/minecraft/world/level/block/BigDripleafBlock", "net/minecraft/world/level/block/HorizontalDirectionalBlock", 0),
    ("net/minecraft/world/level/block/BigDripleafStemBlock", "net/minecraft/world/level/block/HorizontalDirectionalBlock", 0),
    ("net/minecraft/world/level/block/BlastFurnaceBlock", "net/minecraft/world/level/block/AbstractFurnaceBlock", 0),
    ("net/minecraft/world/level/block/Block", "net/minecraft/world/level/block/state/BlockBehaviour", 0),
    ("net/minecraft/world/level/block/BonemealableFeaturePlacerBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/BrewingStandBlock", "net/minecraft/world/level/block/BaseEntityBlock", 2),
    ("net/minecraft/world/level/block/BrushableBlock", "net/minecraft/world/level/block/BaseEntityBlock", 0),
    ("net/minecraft/world/level/block/BubbleColumnBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/BuddingAmethystBlock", "net/minecraft/world/level/block/AmethystBlock", 0),
    ("net/minecraft/world/level/block/BushBlock", "net/minecraft/world/level/block/VegetationBlock", 0),
    ("net/minecraft/world/level/block/ButtonBlock", "net/minecraft/world/level/block/FaceAttachedHorizontalDirectionalBlock", 2),
    ("net/minecraft/world/level/block/CactusBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/CactusFlowerBlock", "net/minecraft/world/level/block/VegetationBlock", 0),
    ("net/minecraft/world/level/block/CakeBlock", "net/minecraft/world/level/block/Block", 3),
    ("net/minecraft/world/level/block/CalibratedSculkSensorBlock", "net/minecraft/world/level/block/SculkSensorBlock", 0),
    ("net/minecraft/world/level/block/CampfireBlock", "net/minecraft/world/level/block/BaseEntityBlock", 1),
    ("net/minecraft/world/level/block/CandleBlock", "net/minecraft/world/level/block/AbstractCandleBlock", 1),
    ("net/minecraft/world/level/block/CandleCakeBlock", "net/minecraft/world/level/block/AbstractCandleBlock", 3),
    ("net/minecraft/world/level/block/CarpetBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/CarrotBlock", "net/minecraft/world/level/block/CropBlock", 0),
    ("net/minecraft/world/level/block/CartographyTableBlock", "net/minecraft/world/level/block/Block", 6),
    ("net/minecraft/world/level/block/CarvedPumpkinBlock", "net/minecraft/world/level/block/HorizontalDirectionalBlock", 0),
    ("net/minecraft/world/level/block/CauldronBlock", "net/minecraft/world/level/block/AbstractCauldronBlock", 0),
    ("net/minecraft/world/level/block/CaveVinesBlock", "net/minecraft/world/level/block/GrowingPlantHeadBlock", 2),
    ("net/minecraft/world/level/block/CaveVinesPlantBlock", "net/minecraft/world/level/block/GrowingPlantBodyBlock", 2),
    ("net/minecraft/world/level/block/CeilingHangingSignBlock", "net/minecraft/world/level/block/SignBlock", 1),
    ("net/minecraft/world/level/block/ChainBlock", "net/minecraft/world/level/block/RotatedPillarBlock", 0),
    ("net/minecraft/world/level/block/ChestBlock", "net/minecraft/world/level/block/AbstractChestBlock", 6),
    ("net/minecraft/world/level/block/ChiseledBookShelfBlock", "net/minecraft/world/level/block/BaseEntityBlock", 3),
    ("net/minecraft/world/level/block/ChorusFlowerBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/ChorusPlantBlock", "net/minecraft/world/level/block/PipeBlock", 0),
    ("net/minecraft/world/level/block/CocoaBlock", "net/minecraft/world/level/block/HorizontalDirectionalBlock", 0),
    ("net/minecraft/world/level/block/ColoredFallingBlock", "net/minecraft/world/level/block/FallingBlock", 0),
    ("net/minecraft/world/level/block/CommandBlock", "net/minecraft/world/level/block/BaseEntityBlock", 2),
    ("net/minecraft/world/level/block/ComparatorBlock", "net/minecraft/world/level/block/DiodeBlock", 2),
    ("net/minecraft/world/level/block/ComposterBlock", "net/minecraft/world/level/block/Block", 3),
    ("net/minecraft/world/level/block/ConcretePowderBlock", "net/minecraft/world/level/block/FallingBlock", 0),
    ("net/minecraft/world/level/block/ConduitBlock", "net/minecraft/world/level/block/BaseEntityBlock", 0),
    ("net/minecraft/world/level/block/CopperBulbBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/CopperChestBlock", "net/minecraft/world/level/block/ChestBlock", 0),
    ("net/minecraft/world/level/block/CopperGolemStatueBlock", "net/minecraft/world/level/block/BaseEntityBlock", 1),
    ("net/minecraft/world/level/block/CoralBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/CoralFanBlock", "net/minecraft/world/level/block/BaseCoralFanBlock", 0),
    ("net/minecraft/world/level/block/CoralPlantBlock", "net/minecraft/world/level/block/BaseCoralPlantTypeBlock", 0),
    ("net/minecraft/world/level/block/CoralWallFanBlock", "net/minecraft/world/level/block/BaseCoralWallFanBlock", 0),
    ("net/minecraft/world/level/block/CrafterBlock", "net/minecraft/world/level/block/BaseEntityBlock", 2),
    ("net/minecraft/world/level/block/CraftingTableBlock", "net/minecraft/world/level/block/Block", 6),
    ("net/minecraft/world/level/block/CreakingHeartBlock", "net/minecraft/world/level/block/BaseEntityBlock", 0),
    ("net/minecraft/world/level/block/CropBlock", "net/minecraft/world/level/block/VegetationBlock", 0),
    ("net/minecraft/world/level/block/CrossCollisionBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/CryingObsidianBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/DaylightDetectorBlock", "net/minecraft/world/level/block/BaseEntityBlock", 2),
    ("net/minecraft/world/level/block/DecoratedPotBlock", "net/minecraft/world/level/block/BaseEntityBlock", 3),
    ("net/minecraft/world/level/block/DetectorRailBlock", "net/minecraft/world/level/block/BaseRailBlock", 0),
    ("net/minecraft/world/level/block/DiodeBlock", "net/minecraft/world/level/block/HorizontalDirectionalBlock", 0),
    ("net/minecraft/world/level/block/DirectionalBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/DispenserBlock", "net/minecraft/world/level/block/BaseEntityBlock", 2),
    ("net/minecraft/world/level/block/DoorBlock", "net/minecraft/world/level/block/Block", 2),
    ("net/minecraft/world/level/block/DoublePlantBlock", "net/minecraft/world/level/block/VegetationBlock", 0),
    ("net/minecraft/world/level/block/DragonEggBlock", "net/minecraft/world/level/block/FallingBlock", 2),
    ("net/minecraft/world/level/block/DriedGhastBlock", "net/minecraft/world/level/block/HorizontalDirectionalBlock", 0),
    ("net/minecraft/world/level/block/DropExperienceBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/DropperBlock", "net/minecraft/world/level/block/DispenserBlock", 0),
    ("net/minecraft/world/level/block/DryVegetationBlock", "net/minecraft/world/level/block/VegetationBlock", 0),
    ("net/minecraft/world/level/block/EnchantingTableBlock", "net/minecraft/world/level/block/BaseEntityBlock", 6),
    ("net/minecraft/world/level/block/EndGatewayBlock", "net/minecraft/world/level/block/BaseEntityBlock", 0),
    ("net/minecraft/world/level/block/EndPortalBlock", "net/minecraft/world/level/block/BaseEntityBlock", 0),
    ("net/minecraft/world/level/block/EndPortalFrameBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/EndRodBlock", "net/minecraft/world/level/block/RodBlock", 0),
    ("net/minecraft/world/level/block/EnderChestBlock", "net/minecraft/world/level/block/AbstractChestBlock", 2),
    ("net/minecraft/world/level/block/EyeblossomBlock", "net/minecraft/world/level/block/FlowerBlock", 0),
    ("net/minecraft/world/level/block/FaceAttachedHorizontalDirectionalBlock", "net/minecraft/world/level/block/HorizontalDirectionalBlock", 0),
    ("net/minecraft/world/level/block/FallingBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/FallingParticlesLeavesBlock", "net/minecraft/world/level/block/LeavesBlock", 0),
    ("net/minecraft/world/level/block/FarmlandBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/FenceBlock", "net/minecraft/world/level/block/CrossCollisionBlock", 2),
    ("net/minecraft/world/level/block/FenceGateBlock", "net/minecraft/world/level/block/HorizontalDirectionalBlock", 2),
    ("net/minecraft/world/level/block/FireBlock", "net/minecraft/world/level/block/BaseFireBlock", 0),
    ("net/minecraft/world/level/block/FireflyBushBlock", "net/minecraft/world/level/block/VegetationBlock", 0),
    ("net/minecraft/world/level/block/FlowerBedBlock", "net/minecraft/world/level/block/VegetationBlock", 0),
    ("net/minecraft/world/level/block/FlowerBlock", "net/minecraft/world/level/block/VegetationBlock", 0),
    ("net/minecraft/world/level/block/FlowerPotBlock", "net/minecraft/world/level/block/Block", 3),
    ("net/minecraft/world/level/block/FrogspawnBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/FrostedIceBlock", "net/minecraft/world/level/block/IceBlock", 0),
    ("net/minecraft/world/level/block/FurnaceBlock", "net/minecraft/world/level/block/AbstractFurnaceBlock", 0),
    ("net/minecraft/world/level/block/GlazedTerracottaBlock", "net/minecraft/world/level/block/HorizontalDirectionalBlock", 0),
    ("net/minecraft/world/level/block/GlowLichenBlock", "net/minecraft/world/level/block/MultifaceSpreadeableBlock", 0),
    ("net/minecraft/world/level/block/GrassBlock", "net/minecraft/world/level/block/SpreadingSnowyBlock", 0),
    ("net/minecraft/world/level/block/GrindstoneBlock", "net/minecraft/world/level/block/FaceAttachedHorizontalDirectionalBlock", 6),
    ("net/minecraft/world/level/block/GrowingPlantBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/GrowingPlantBodyBlock", "net/minecraft/world/level/block/GrowingPlantBlock", 0),
    ("net/minecraft/world/level/block/GrowingPlantHeadBlock", "net/minecraft/world/level/block/GrowingPlantBlock", 0),
    ("net/minecraft/world/level/block/HalfTransparentBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/HangingMossBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/HangingRootsBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/HayBlock", "net/minecraft/world/level/block/RotatedPillarBlock", 0),
    ("net/minecraft/world/level/block/HeavyCoreBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/HoneyBlock", "net/minecraft/world/level/block/HalfTransparentBlock", 0),
    ("net/minecraft/world/level/block/HopperBlock", "net/minecraft/world/level/block/BaseEntityBlock", 2),
    ("net/minecraft/world/level/block/HorizontalDirectionalBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/HugeMushroomBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/IceBlock", "net/minecraft/world/level/block/HalfTransparentBlock", 0),
    ("net/minecraft/world/level/block/InfestedBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/InfestedRotatedPillarBlock", "net/minecraft/world/level/block/InfestedBlock", 0),
    ("net/minecraft/world/level/block/IronBarsBlock", "net/minecraft/world/level/block/CrossCollisionBlock", 0),
    ("net/minecraft/world/level/block/JigsawBlock", "net/minecraft/world/level/block/Block", 2),
    ("net/minecraft/world/level/block/JukeboxBlock", "net/minecraft/world/level/block/BaseEntityBlock", 3),
    ("net/minecraft/world/level/block/KelpBlock", "net/minecraft/world/level/block/GrowingPlantHeadBlock", 0),
    ("net/minecraft/world/level/block/KelpPlantBlock", "net/minecraft/world/level/block/GrowingPlantBodyBlock", 0),
    ("net/minecraft/world/level/block/LadderBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/LanternBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/LavaCauldronBlock", "net/minecraft/world/level/block/AbstractCauldronBlock", 0),
    ("net/minecraft/world/level/block/LayeredCauldronBlock", "net/minecraft/world/level/block/AbstractCauldronBlock", 0),
    ("net/minecraft/world/level/block/LeafLitterBlock", "net/minecraft/world/level/block/VegetationBlock", 0),
    ("net/minecraft/world/level/block/LeavesBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/LecternBlock", "net/minecraft/world/level/block/BaseEntityBlock", 7),
    ("net/minecraft/world/level/block/LeverBlock", "net/minecraft/world/level/block/FaceAttachedHorizontalDirectionalBlock", 2),
    ("net/minecraft/world/level/block/LightBlock", "net/minecraft/world/level/block/Block", 2),
    ("net/minecraft/world/level/block/LightningRodBlock", "net/minecraft/world/level/block/RodBlock", 0),
    ("net/minecraft/world/level/block/LilyPadBlock", "net/minecraft/world/level/block/VegetationBlock", 0),
    ("net/minecraft/world/level/block/LiquidBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/LoomBlock", "net/minecraft/world/level/block/HorizontalDirectionalBlock", 6),
    ("net/minecraft/world/level/block/MagmaBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/MangroveLeavesBlock", "net/minecraft/world/level/block/TintedParticleLeavesBlock", 0),
    ("net/minecraft/world/level/block/MangrovePropaguleBlock", "net/minecraft/world/level/block/SaplingBlock", 0),
    ("net/minecraft/world/level/block/MangroveRootsBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/MossyCarpetBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/MudBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/MultifaceBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/MultifaceSpreadeableBlock", "net/minecraft/world/level/block/MultifaceBlock", 0),
    ("net/minecraft/world/level/block/MushroomBlock", "net/minecraft/world/level/block/VegetationBlock", 0),
    ("net/minecraft/world/level/block/MyceliumBlock", "net/minecraft/world/level/block/SpreadingSnowyBlock", 0),
    ("net/minecraft/world/level/block/NetherFungusBlock", "net/minecraft/world/level/block/VegetationBlock", 0),
    ("net/minecraft/world/level/block/NetherPortalBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/NetherRootsBlock", "net/minecraft/world/level/block/VegetationBlock", 0),
    ("net/minecraft/world/level/block/NetherSproutsBlock", "net/minecraft/world/level/block/VegetationBlock", 0),
    ("net/minecraft/world/level/block/NetherWartBlock", "net/minecraft/world/level/block/VegetationBlock", 0),
    ("net/minecraft/world/level/block/NetherrackBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/NoteBlock", "net/minecraft/world/level/block/Block", 3),
    ("net/minecraft/world/level/block/NyliumBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/ObserverBlock", "net/minecraft/world/level/block/DirectionalBlock", 0),
    ("net/minecraft/world/level/block/PathBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/PiglinWallSkullBlock", "net/minecraft/world/level/block/WallSkullBlock", 0),
    ("net/minecraft/world/level/block/PipeBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/PitcherCropBlock", "net/minecraft/world/level/block/DoublePlantBlock", 0),
    ("net/minecraft/world/level/block/PlayerHeadBlock", "net/minecraft/world/level/block/SkullBlock", 0),
    ("net/minecraft/world/level/block/PlayerWallHeadBlock", "net/minecraft/world/level/block/WallSkullBlock", 0),
    ("net/minecraft/world/level/block/PointedDripstoneBlock", "net/minecraft/world/level/block/SpeleothemBlock", 0),
    ("net/minecraft/world/level/block/PotatoBlock", "net/minecraft/world/level/block/CropBlock", 0),
    ("net/minecraft/world/level/block/PotentSulfurBlock", "net/minecraft/world/level/block/BaseEntityBlock", 0),
    ("net/minecraft/world/level/block/PowderSnowBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/PoweredBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/PoweredRailBlock", "net/minecraft/world/level/block/BaseRailBlock", 0),
    ("net/minecraft/world/level/block/PressurePlateBlock", "net/minecraft/world/level/block/BasePressurePlateBlock", 0),
    ("net/minecraft/world/level/block/PumpkinBlock", "net/minecraft/world/level/block/Block", 1),
    ("net/minecraft/world/level/block/RailBlock", "net/minecraft/world/level/block/BaseRailBlock", 0),
    ("net/minecraft/world/level/block/RedStoneOreBlock", "net/minecraft/world/level/block/Block", 1),
    ("net/minecraft/world/level/block/RedstoneLampBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/RedstoneTorchBlock", "net/minecraft/world/level/block/BaseTorchBlock", 0),
    ("net/minecraft/world/level/block/RedstoneWallTorchBlock", "net/minecraft/world/level/block/RedstoneTorchBlock", 0),
    ("net/minecraft/world/level/block/RedstoneWireBlock", "net/minecraft/world/level/block/Block", 2),
    ("net/minecraft/world/level/block/RepeaterBlock", "net/minecraft/world/level/block/DiodeBlock", 2),
    ("net/minecraft/world/level/block/RespawnAnchorBlock", "net/minecraft/world/level/block/Block", 3),
    ("net/minecraft/world/level/block/RodBlock", "net/minecraft/world/level/block/DirectionalBlock", 0),
    ("net/minecraft/world/level/block/RootedDirtBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/RotatedPillarBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/SandBlock", "net/minecraft/world/level/block/ColoredFallingBlock", 0),
    ("net/minecraft/world/level/block/SaplingBlock", "net/minecraft/world/level/block/VegetationBlock", 0),
    ("net/minecraft/world/level/block/ScaffoldingBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/SculkBlock", "net/minecraft/world/level/block/DropExperienceBlock", 0),
    ("net/minecraft/world/level/block/SculkCatalystBlock", "net/minecraft/world/level/block/BaseEntityBlock", 0),
    ("net/minecraft/world/level/block/SculkSensorBlock", "net/minecraft/world/level/block/BaseEntityBlock", 0),
    ("net/minecraft/world/level/block/SculkShriekerBlock", "net/minecraft/world/level/block/BaseEntityBlock", 0),
    ("net/minecraft/world/level/block/SculkVeinBlock", "net/minecraft/world/level/block/MultifaceSpreadeableBlock", 0),
    ("net/minecraft/world/level/block/SeaPickleBlock", "net/minecraft/world/level/block/VegetationBlock", 0),
    ("net/minecraft/world/level/block/SeagrassBlock", "net/minecraft/world/level/block/VegetationBlock", 0),
    ("net/minecraft/world/level/block/ShelfBlock", "net/minecraft/world/level/block/BaseEntityBlock", 1),
    ("net/minecraft/world/level/block/ShelfMushroomBlock", "net/minecraft/world/level/block/HorizontalDirectionalBlock", 0),
    ("net/minecraft/world/level/block/ShortDryGrassBlock", "net/minecraft/world/level/block/DryVegetationBlock", 0),
    ("net/minecraft/world/level/block/ShulkerBoxBlock", "net/minecraft/world/level/block/BaseEntityBlock", 2),
    ("net/minecraft/world/level/block/SignBlock", "net/minecraft/world/level/block/BaseEntityBlock", 3),
    ("net/minecraft/world/level/block/SkullBlock", "net/minecraft/world/level/block/AbstractSkullBlock", 0),
    ("net/minecraft/world/level/block/SlabBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/SlimeBlock", "net/minecraft/world/level/block/HalfTransparentBlock", 0),
    ("net/minecraft/world/level/block/SmallDripleafBlock", "net/minecraft/world/level/block/DoublePlantBlock", 0),
    ("net/minecraft/world/level/block/SmithingTableBlock", "net/minecraft/world/level/block/CraftingTableBlock", 6),
    ("net/minecraft/world/level/block/SmokerBlock", "net/minecraft/world/level/block/AbstractFurnaceBlock", 0),
    ("net/minecraft/world/level/block/SnifferEggBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/SnowLayerBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/SnowyBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/SoulFireBlock", "net/minecraft/world/level/block/BaseFireBlock", 0),
    ("net/minecraft/world/level/block/SoulSandBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/SpawnerBlock", "net/minecraft/world/level/block/BaseEntityBlock", 0),
    ("net/minecraft/world/level/block/SpeleothemBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/SpongeBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/SporeBlossomBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/SpreadingSnowyBlock", "net/minecraft/world/level/block/SnowyBlock", 0),
    ("net/minecraft/world/level/block/StainedGlassBlock", "net/minecraft/world/level/block/TransparentBlock", 0),
    ("net/minecraft/world/level/block/StainedGlassPaneBlock", "net/minecraft/world/level/block/IronBarsBlock", 0),
    ("net/minecraft/world/level/block/StairBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/StandingSignBlock", "net/minecraft/world/level/block/SignBlock", 0),
    ("net/minecraft/world/level/block/StemBlock", "net/minecraft/world/level/block/VegetationBlock", 0),
    ("net/minecraft/world/level/block/StonecutterBlock", "net/minecraft/world/level/block/Block", 6),
    ("net/minecraft/world/level/block/StrawBedBlock", "net/minecraft/world/level/block/AbstractBedBlock", 0),
    ("net/minecraft/world/level/block/StructureBlock", "net/minecraft/world/level/block/BaseEntityBlock", 2),
    ("net/minecraft/world/level/block/StructureVoidBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/SugarCaneBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/SulfurSpikeBlock", "net/minecraft/world/level/block/SpeleothemBlock", 0),
    ("net/minecraft/world/level/block/SweetBerryBushBlock", "net/minecraft/world/level/block/VegetationBlock", 3),
    ("net/minecraft/world/level/block/TallDryGrassBlock", "net/minecraft/world/level/block/DryVegetationBlock", 0),
    ("net/minecraft/world/level/block/TallFlowerBlock", "net/minecraft/world/level/block/DoublePlantBlock", 0),
    ("net/minecraft/world/level/block/TallGrassBlock", "net/minecraft/world/level/block/VegetationBlock", 0),
    ("net/minecraft/world/level/block/TallSeagrassBlock", "net/minecraft/world/level/block/DoublePlantBlock", 0),
    ("net/minecraft/world/level/block/TargetBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/TestBlock", "net/minecraft/world/level/block/BaseEntityBlock", 2),
    ("net/minecraft/world/level/block/TestInstanceBlock", "net/minecraft/world/level/block/BaseEntityBlock", 2),
    ("net/minecraft/world/level/block/TintedGlassBlock", "net/minecraft/world/level/block/TransparentBlock", 0),
    ("net/minecraft/world/level/block/TintedParticleLeavesBlock", "net/minecraft/world/level/block/FallingParticlesLeavesBlock", 0),
    ("net/minecraft/world/level/block/TntBlock", "net/minecraft/world/level/block/Block", 1),
    ("net/minecraft/world/level/block/TorchBlock", "net/minecraft/world/level/block/BaseTorchBlock", 0),
    ("net/minecraft/world/level/block/TorchflowerCropBlock", "net/minecraft/world/level/block/CropBlock", 0),
    ("net/minecraft/world/level/block/TransparentBlock", "net/minecraft/world/level/block/HalfTransparentBlock", 0),
    ("net/minecraft/world/level/block/TrapDoorBlock", "net/minecraft/world/level/block/HorizontalDirectionalBlock", 2),
    ("net/minecraft/world/level/block/TrappedChestBlock", "net/minecraft/world/level/block/ChestBlock", 0),
    ("net/minecraft/world/level/block/TrialSpawnerBlock", "net/minecraft/world/level/block/BaseEntityBlock", 0),
    ("net/minecraft/world/level/block/TripWireBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/TripWireHookBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/TurtleEggBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/TwistingVinesBlock", "net/minecraft/world/level/block/GrowingPlantHeadBlock", 0),
    ("net/minecraft/world/level/block/TwistingVinesPlantBlock", "net/minecraft/world/level/block/GrowingPlantBodyBlock", 0),
    ("net/minecraft/world/level/block/UntintedParticleLeavesBlock", "net/minecraft/world/level/block/FallingParticlesLeavesBlock", 0),
    ("net/minecraft/world/level/block/VaultBlock", "net/minecraft/world/level/block/BaseEntityBlock", 1),
    ("net/minecraft/world/level/block/VegetationBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/VineBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/WallBannerBlock", "net/minecraft/world/level/block/AbstractBannerBlock", 0),
    ("net/minecraft/world/level/block/WallBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/WallHangingSignBlock", "net/minecraft/world/level/block/SignBlock", 1),
    ("net/minecraft/world/level/block/WallSignBlock", "net/minecraft/world/level/block/SignBlock", 0),
    ("net/minecraft/world/level/block/WallSkullBlock", "net/minecraft/world/level/block/AbstractSkullBlock", 0),
    ("net/minecraft/world/level/block/WallTorchBlock", "net/minecraft/world/level/block/TorchBlock", 0),
    ("net/minecraft/world/level/block/WaterloggedTransparentBlock", "net/minecraft/world/level/block/TransparentBlock", 0),
    ("net/minecraft/world/level/block/WeatheringCopperBarsBlock", "net/minecraft/world/level/block/IronBarsBlock", 0),
    ("net/minecraft/world/level/block/WeatheringCopperBulbBlock", "net/minecraft/world/level/block/CopperBulbBlock", 0),
    ("net/minecraft/world/level/block/WeatheringCopperChainBlock", "net/minecraft/world/level/block/ChainBlock", 0),
    ("net/minecraft/world/level/block/WeatheringCopperChestBlock", "net/minecraft/world/level/block/CopperChestBlock", 0),
    ("net/minecraft/world/level/block/WeatheringCopperDoorBlock", "net/minecraft/world/level/block/DoorBlock", 0),
    ("net/minecraft/world/level/block/WeatheringCopperFullBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/WeatheringCopperGolemStatueBlock", "net/minecraft/world/level/block/CopperGolemStatueBlock", 1),
    ("net/minecraft/world/level/block/WeatheringCopperGrateBlock", "net/minecraft/world/level/block/WaterloggedTransparentBlock", 0),
    ("net/minecraft/world/level/block/WeatheringCopperSlabBlock", "net/minecraft/world/level/block/SlabBlock", 0),
    ("net/minecraft/world/level/block/WeatheringCopperStairBlock", "net/minecraft/world/level/block/StairBlock", 0),
    ("net/minecraft/world/level/block/WeatheringCopperTrapDoorBlock", "net/minecraft/world/level/block/TrapDoorBlock", 0),
    ("net/minecraft/world/level/block/WeatheringLanternBlock", "net/minecraft/world/level/block/LanternBlock", 0),
    ("net/minecraft/world/level/block/WeatheringLightningRodBlock", "net/minecraft/world/level/block/LightningRodBlock", 0),
    ("net/minecraft/world/level/block/WebBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/WeepingVinesBlock", "net/minecraft/world/level/block/GrowingPlantHeadBlock", 0),
    ("net/minecraft/world/level/block/WeepingVinesPlantBlock", "net/minecraft/world/level/block/GrowingPlantBodyBlock", 0),
    ("net/minecraft/world/level/block/WeightedPressurePlateBlock", "net/minecraft/world/level/block/BasePressurePlateBlock", 0),
    ("net/minecraft/world/level/block/WetSpongeBlock", "net/minecraft/world/level/block/Block", 0),
    ("net/minecraft/world/level/block/WitherRoseBlock", "net/minecraft/world/level/block/FlowerBlock", 0),
    ("net/minecraft/world/level/block/WitherSkullBlock", "net/minecraft/world/level/block/SkullBlock", 0),
    ("net/minecraft/world/level/block/WitherWallSkullBlock", "net/minecraft/world/level/block/WallSkullBlock", 0),
    ("net/minecraft/world/level/block/WoolCarpetBlock", "net/minecraft/world/level/block/CarpetBlock", 0),
    ("net/minecraft/world/level/block/piston/MovingPistonBlock", "net/minecraft/world/level/block/BaseEntityBlock", 2),
    ("net/minecraft/world/level/block/piston/PistonBaseBlock", "net/minecraft/world/level/block/DirectionalBlock", 0),
    ("net/minecraft/world/level/block/piston/PistonHeadBlock", "net/minecraft/world/level/block/DirectionalBlock", 0),
    ("net/minecraft/world/level/block/state/BlockBehaviour", "", 7),
];

#[cfg(test)]
mod tests {
    use super::*;

    const BLOCK_BEHAVIOUR: &str = "net/minecraft/world/level/block/state/BlockBehaviour";

    #[test]
    fn table_is_sorted_and_every_superclass_is_a_row() {
        assert!(CLASSES.windows(2).all(|pair| pair[0].0 < pair[1].0));
        for (class, superclass, _) in CLASSES {
            assert!(
                superclass.is_empty() || CLASSES.iter().any(|(name, _, _)| name == superclass),
                "{class}'s superclass {superclass} has no row"
            );
        }
        // `BlockBehaviour` declares all three: every chain ends at a declaration.
        assert_eq!(declaring_class(BLOCK_BEHAVIOUR, 7), Some(BLOCK_BEHAVIOUR));
    }

    /// `javap -p` of `FurnaceBlock`, `AbstractFurnaceBlock`, `BaseEntityBlock` and
    /// `CraftingTableBlock` in `server-26.3.jar`.
    #[test]
    fn furnace_and_crafting_table_chains_match_the_jar() {
        let block = |name: &str| format!("net/minecraft/world/level/block/{name}");
        let furnace = block("FurnaceBlock");
        assert_eq!(
            declaring_class(&furnace, USE_ITEM_ON),
            Some(BLOCK_BEHAVIOUR)
        );
        assert_eq!(
            declaring_class(&furnace, USE_WITHOUT_ITEM),
            Some("net/minecraft/world/level/block/AbstractFurnaceBlock")
        );
        assert_eq!(
            declaring_class(&furnace, GET_MENU_PROVIDER),
            Some("net/minecraft/world/level/block/BaseEntityBlock")
        );
        let crafting = block("CraftingTableBlock");
        assert_eq!(
            declaring_class(&crafting, USE_ITEM_ON),
            Some(BLOCK_BEHAVIOUR)
        );
        assert_eq!(
            declaring_class(&crafting, USE_WITHOUT_ITEM),
            Some("net/minecraft/world/level/block/CraftingTableBlock")
        );
        assert_eq!(
            declaring_class(&crafting, GET_MENU_PROVIDER),
            Some("net/minecraft/world/level/block/CraftingTableBlock")
        );
        for method in [USE_ITEM_ON, USE_WITHOUT_ITEM, GET_MENU_PROVIDER] {
            assert_eq!(
                declaring_class(&block("Block"), method),
                Some(BLOCK_BEHAVIOUR)
            );
        }
        assert_eq!(
            declaring_class("com/example/LibraryBlock", USE_ITEM_ON),
            None
        );
        assert!(extends(&furnace, &block("BaseEntityBlock")));
        assert!(extends(&furnace, BLOCK_BEHAVIOUR));
        assert!(!extends(&crafting, &block("BaseEntityBlock")));
        assert!(!extends("com/example/LibraryBlock", BLOCK_BEHAVIOUR));
    }
}
