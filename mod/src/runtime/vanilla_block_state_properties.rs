//! `BlockStateProperties` of Minecraft 26.3, captured with JDK 25 by
//! `tests/fixtures/block_state_properties/oracle/CaptureBlockStateProperties.java` against the
//! real server classes: one `FIELD|name|type|[values]|declared type` row per public static `Property` field,
//! the values in `getPossibleValues()` order spelled by `Property.getName(value)`, and the field's
//! declared class (a `getstatic` resolves a field by name and that descriptor). Kept in Rust source (not a `.txt`) because Sunrise's mod
//! staging copies only `.rs` and `.toml` files into the vendored runtime.

#[rustfmt::skip]
pub(crate) const TABLE: &str = r#"AGE_1|age|integer|[0, 1]|net/minecraft/world/level/block/state/properties/IntegerProperty
AGE_15|age|integer|[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]|net/minecraft/world/level/block/state/properties/IntegerProperty
AGE_2|age|integer|[0, 1, 2]|net/minecraft/world/level/block/state/properties/IntegerProperty
AGE_25|age|integer|[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25]|net/minecraft/world/level/block/state/properties/IntegerProperty
AGE_3|age|integer|[0, 1, 2, 3]|net/minecraft/world/level/block/state/properties/IntegerProperty
AGE_4|age|integer|[0, 1, 2, 3, 4]|net/minecraft/world/level/block/state/properties/IntegerProperty
AGE_5|age|integer|[0, 1, 2, 3, 4, 5]|net/minecraft/world/level/block/state/properties/IntegerProperty
AGE_7|age|integer|[0, 1, 2, 3, 4, 5, 6, 7]|net/minecraft/world/level/block/state/properties/IntegerProperty
ATTACHED|attached|boolean|[true, false]|net/minecraft/world/level/block/state/properties/BooleanProperty
ATTACH_FACE|face|enum|[floor, wall, ceiling]|net/minecraft/world/level/block/state/properties/EnumProperty
AXIS|axis|enum|[x, y, z]|net/minecraft/world/level/block/state/properties/EnumProperty
BAMBOO_LEAVES|leaves|enum|[none, small, large]|net/minecraft/world/level/block/state/properties/EnumProperty
BED_PART|part|enum|[head, foot]|net/minecraft/world/level/block/state/properties/EnumProperty
BELL_ATTACHMENT|attachment|enum|[floor, ceiling, single_wall, double_wall]|net/minecraft/world/level/block/state/properties/EnumProperty
BERRIES|berries|boolean|[true, false]|net/minecraft/world/level/block/state/properties/BooleanProperty
BITES|bites|integer|[0, 1, 2, 3, 4, 5, 6]|net/minecraft/world/level/block/state/properties/IntegerProperty
BLOOM|bloom|boolean|[true, false]|net/minecraft/world/level/block/state/properties/BooleanProperty
BOTTOM|bottom|boolean|[true, false]|net/minecraft/world/level/block/state/properties/BooleanProperty
CANDLES|candles|integer|[1, 2, 3, 4]|net/minecraft/world/level/block/state/properties/IntegerProperty
CAN_SUMMON|can_summon|boolean|[true, false]|net/minecraft/world/level/block/state/properties/BooleanProperty
CHEST_TYPE|type|enum|[single, left, right]|net/minecraft/world/level/block/state/properties/EnumProperty
CONDITIONAL|conditional|boolean|[true, false]|net/minecraft/world/level/block/state/properties/BooleanProperty
COPPER_GOLEM_POSE|copper_golem_pose|enum|[standing, sitting, running, star]|net/minecraft/world/level/block/state/properties/EnumProperty
CRACKED|cracked|boolean|[true, false]|net/minecraft/world/level/block/state/properties/BooleanProperty
CRAFTING|crafting|boolean|[true, false]|net/minecraft/world/level/block/state/properties/BooleanProperty
CREAKING_HEART_STATE|creaking_heart_state|enum|[uprooted, dormant, awake]|net/minecraft/world/level/block/state/properties/EnumProperty
DELAY|delay|integer|[1, 2, 3, 4]|net/minecraft/world/level/block/state/properties/IntegerProperty
DISARMED|disarmed|boolean|[true, false]|net/minecraft/world/level/block/state/properties/BooleanProperty
DISTANCE|distance|integer|[1, 2, 3, 4, 5, 6, 7]|net/minecraft/world/level/block/state/properties/IntegerProperty
DOOR_HINGE|hinge|enum|[left, right]|net/minecraft/world/level/block/state/properties/EnumProperty
DOUBLE_BLOCK_HALF|half|enum|[upper, lower]|net/minecraft/world/level/block/state/properties/EnumProperty
DOWN|down|boolean|[true, false]|net/minecraft/world/level/block/state/properties/BooleanProperty
DRAG|drag|boolean|[true, false]|net/minecraft/world/level/block/state/properties/BooleanProperty
DRIED_GHAST_HYDRATION_LEVELS|hydration|integer|[0, 1, 2, 3]|net/minecraft/world/level/block/state/properties/IntegerProperty
DUSTED|dusted|integer|[0, 1, 2, 3]|net/minecraft/world/level/block/state/properties/IntegerProperty
EAST|east|boolean|[true, false]|net/minecraft/world/level/block/state/properties/BooleanProperty
EAST_REDSTONE|east|enum|[up, side, none]|net/minecraft/world/level/block/state/properties/EnumProperty
EAST_WALL|east|enum|[none, low, tall]|net/minecraft/world/level/block/state/properties/EnumProperty
EGGS|eggs|integer|[1, 2, 3, 4]|net/minecraft/world/level/block/state/properties/IntegerProperty
ENABLED|enabled|boolean|[true, false]|net/minecraft/world/level/block/state/properties/BooleanProperty
EXTENDED|extended|boolean|[true, false]|net/minecraft/world/level/block/state/properties/BooleanProperty
EYE|eye|boolean|[true, false]|net/minecraft/world/level/block/state/properties/BooleanProperty
FACING|facing|enum|[north, east, south, west, up, down]|net/minecraft/world/level/block/state/properties/EnumProperty
FACING_HOPPER|facing|enum|[down, north, south, west, east]|net/minecraft/world/level/block/state/properties/EnumProperty
FALLING|falling|boolean|[true, false]|net/minecraft/world/level/block/state/properties/BooleanProperty
FLOWER_AMOUNT|flower_amount|integer|[1, 2, 3, 4]|net/minecraft/world/level/block/state/properties/IntegerProperty
HALF|half|enum|[top, bottom]|net/minecraft/world/level/block/state/properties/EnumProperty
HANGING|hanging|boolean|[true, false]|net/minecraft/world/level/block/state/properties/BooleanProperty
HAS_BOOK|has_book|boolean|[true, false]|net/minecraft/world/level/block/state/properties/BooleanProperty
HAS_BOTTLE_0|has_bottle_0|boolean|[true, false]|net/minecraft/world/level/block/state/properties/BooleanProperty
HAS_BOTTLE_1|has_bottle_1|boolean|[true, false]|net/minecraft/world/level/block/state/properties/BooleanProperty
HAS_BOTTLE_2|has_bottle_2|boolean|[true, false]|net/minecraft/world/level/block/state/properties/BooleanProperty
HAS_RECORD|has_record|boolean|[true, false]|net/minecraft/world/level/block/state/properties/BooleanProperty
HATCH|hatch|integer|[0, 1, 2]|net/minecraft/world/level/block/state/properties/IntegerProperty
HORIZONTAL_AXIS|axis|enum|[x, z]|net/minecraft/world/level/block/state/properties/EnumProperty
HORIZONTAL_FACING|facing|enum|[north, south, west, east]|net/minecraft/world/level/block/state/properties/EnumProperty
INVERTED|inverted|boolean|[true, false]|net/minecraft/world/level/block/state/properties/BooleanProperty
IN_WALL|in_wall|boolean|[true, false]|net/minecraft/world/level/block/state/properties/BooleanProperty
LAYERS|layers|integer|[1, 2, 3, 4, 5, 6, 7, 8]|net/minecraft/world/level/block/state/properties/IntegerProperty
LEVEL|level|integer|[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]|net/minecraft/world/level/block/state/properties/IntegerProperty
LEVEL_CAULDRON|level|integer|[1, 2, 3]|net/minecraft/world/level/block/state/properties/IntegerProperty
LEVEL_COMPOSTER|level|integer|[0, 1, 2, 3, 4, 5, 6, 7, 8]|net/minecraft/world/level/block/state/properties/IntegerProperty
LEVEL_FLOWING|level|integer|[1, 2, 3, 4, 5, 6, 7, 8]|net/minecraft/world/level/block/state/properties/IntegerProperty
LEVEL_HONEY|honey_level|integer|[0, 1, 2, 3, 4, 5]|net/minecraft/world/level/block/state/properties/IntegerProperty
LIT|lit|boolean|[true, false]|net/minecraft/world/level/block/state/properties/BooleanProperty
LOCKED|locked|boolean|[true, false]|net/minecraft/world/level/block/state/properties/BooleanProperty
MAP|map|boolean|[true, false]|net/minecraft/world/level/block/state/properties/BooleanProperty
MODE_COMPARATOR|mode|enum|[compare, subtract]|net/minecraft/world/level/block/state/properties/EnumProperty
MOISTURE|moisture|integer|[0, 1, 2, 3, 4, 5, 6, 7]|net/minecraft/world/level/block/state/properties/IntegerProperty
NATURAL|natural|boolean|[true, false]|net/minecraft/world/level/block/state/properties/BooleanProperty
NORTH|north|boolean|[true, false]|net/minecraft/world/level/block/state/properties/BooleanProperty
NORTH_REDSTONE|north|enum|[up, side, none]|net/minecraft/world/level/block/state/properties/EnumProperty
NORTH_WALL|north|enum|[none, low, tall]|net/minecraft/world/level/block/state/properties/EnumProperty
NOTE|note|integer|[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24]|net/minecraft/world/level/block/state/properties/IntegerProperty
NOTEBLOCK_INSTRUMENT|instrument|enum|[harp, basedrum, snare, hat, bass, flute, bell, guitar, chime, xylophone, iron_xylophone, cow_bell, didgeridoo, bit, banjo, pling, trumpet, trumpet_exposed, trumpet_oxidized, trumpet_weathered, zombie, skeleton, creeper, dragon, wither_skeleton, piglin, custom_head]|net/minecraft/world/level/block/state/properties/EnumProperty
OCCUPIED|occupied|boolean|[true, false]|net/minecraft/world/level/block/state/properties/BooleanProperty
OMINOUS|ominous|boolean|[true, false]|net/minecraft/world/level/block/state/properties/BooleanProperty
OPEN|open|boolean|[true, false]|net/minecraft/world/level/block/state/properties/BooleanProperty
ORIENTATION|orientation|enum|[down_east, down_north, down_south, down_west, up_east, up_north, up_south, up_west, west_up, east_up, north_up, south_up]|net/minecraft/world/level/block/state/properties/EnumProperty
PERSISTENT|persistent|boolean|[true, false]|net/minecraft/world/level/block/state/properties/BooleanProperty
PICKLES|pickles|integer|[1, 2, 3, 4]|net/minecraft/world/level/block/state/properties/IntegerProperty
PISTON_TYPE|type|enum|[normal, sticky]|net/minecraft/world/level/block/state/properties/EnumProperty
POTENT_SULFUR_STATE|potent_sulfur_state|enum|[dry, wet, dormant, erupting, continuous]|net/minecraft/world/level/block/state/properties/EnumProperty
POWER|power|integer|[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]|net/minecraft/world/level/block/state/properties/IntegerProperty
POWERED|powered|boolean|[true, false]|net/minecraft/world/level/block/state/properties/BooleanProperty
RAIL_SHAPE|shape|enum|[north_south, east_west, ascending_east, ascending_west, ascending_north, ascending_south, south_east, south_west, north_west, north_east]|net/minecraft/world/level/block/state/properties/EnumProperty
RAIL_SHAPE_STRAIGHT|shape|enum|[north_south, east_west, ascending_east, ascending_west, ascending_north, ascending_south]|net/minecraft/world/level/block/state/properties/EnumProperty
RESPAWN_ANCHOR_CHARGES|charges|integer|[0, 1, 2, 3, 4]|net/minecraft/world/level/block/state/properties/IntegerProperty
ROTATION_16|rotation|integer|[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]|net/minecraft/world/level/block/state/properties/IntegerProperty
SCULK_SENSOR_PHASE|sculk_sensor_phase|enum|[inactive, active, cooldown]|net/minecraft/world/level/block/state/properties/EnumProperty
SEGMENT_AMOUNT|segment_amount|integer|[1, 2, 3, 4]|net/minecraft/world/level/block/state/properties/IntegerProperty
SHORT|short|boolean|[true, false]|net/minecraft/world/level/block/state/properties/BooleanProperty
SHRIEKING|shrieking|boolean|[true, false]|net/minecraft/world/level/block/state/properties/BooleanProperty
SIDE_CHAIN_PART|side_chain|enum|[unconnected, right, center, left]|net/minecraft/world/level/block/state/properties/EnumProperty
SIGNAL_FIRE|signal_fire|boolean|[true, false]|net/minecraft/world/level/block/state/properties/BooleanProperty
SLAB_TYPE|type|enum|[top, bottom, double]|net/minecraft/world/level/block/state/properties/EnumProperty
SLOT_0_OCCUPIED|slot_0_occupied|boolean|[true, false]|net/minecraft/world/level/block/state/properties/BooleanProperty
SLOT_1_OCCUPIED|slot_1_occupied|boolean|[true, false]|net/minecraft/world/level/block/state/properties/BooleanProperty
SLOT_2_OCCUPIED|slot_2_occupied|boolean|[true, false]|net/minecraft/world/level/block/state/properties/BooleanProperty
SLOT_3_OCCUPIED|slot_3_occupied|boolean|[true, false]|net/minecraft/world/level/block/state/properties/BooleanProperty
SLOT_4_OCCUPIED|slot_4_occupied|boolean|[true, false]|net/minecraft/world/level/block/state/properties/BooleanProperty
SLOT_5_OCCUPIED|slot_5_occupied|boolean|[true, false]|net/minecraft/world/level/block/state/properties/BooleanProperty
SNOWY|snowy|boolean|[true, false]|net/minecraft/world/level/block/state/properties/BooleanProperty
SOUTH|south|boolean|[true, false]|net/minecraft/world/level/block/state/properties/BooleanProperty
SOUTH_REDSTONE|south|enum|[up, side, none]|net/minecraft/world/level/block/state/properties/EnumProperty
SOUTH_WALL|south|enum|[none, low, tall]|net/minecraft/world/level/block/state/properties/EnumProperty
SPELEOTHEM_THICKNESS|thickness|enum|[tip_merge, tip, frustum, middle, base]|net/minecraft/world/level/block/state/properties/EnumProperty
STABILITY_DISTANCE|distance|integer|[0, 1, 2, 3, 4, 5, 6, 7]|net/minecraft/world/level/block/state/properties/IntegerProperty
STAGE|stage|integer|[0, 1]|net/minecraft/world/level/block/state/properties/IntegerProperty
STAIRS_SHAPE|shape|enum|[straight, inner_left, inner_right, outer_left, outer_right]|net/minecraft/world/level/block/state/properties/EnumProperty
STRUCTUREBLOCK_MODE|mode|enum|[save, load, corner, data]|net/minecraft/world/level/block/state/properties/EnumProperty
TEST_BLOCK_MODE|mode|enum|[start, log, fail, accept]|net/minecraft/world/level/block/state/properties/EnumProperty
TILT|tilt|enum|[none, unstable, partial, full]|net/minecraft/world/level/block/state/properties/EnumProperty
TIP|tip|boolean|[true, false]|net/minecraft/world/level/block/state/properties/BooleanProperty
TRIAL_SPAWNER_STATE|trial_spawner_state|enum|[inactive, waiting_for_players, active, waiting_for_reward_ejection, ejecting_reward, cooldown]|net/minecraft/world/level/block/state/properties/EnumProperty
TRIGGERED|triggered|boolean|[true, false]|net/minecraft/world/level/block/state/properties/BooleanProperty
UNSTABLE|unstable|boolean|[true, false]|net/minecraft/world/level/block/state/properties/BooleanProperty
UP|up|boolean|[true, false]|net/minecraft/world/level/block/state/properties/BooleanProperty
VAULT_STATE|vault_state|enum|[inactive, active, unlocking, ejecting]|net/minecraft/world/level/block/state/properties/EnumProperty
VERTICAL_DIRECTION|vertical_direction|enum|[up, down]|net/minecraft/world/level/block/state/properties/EnumProperty
WATERLOGGED|waterlogged|boolean|[true, false]|net/minecraft/world/level/block/state/properties/BooleanProperty
WEST|west|boolean|[true, false]|net/minecraft/world/level/block/state/properties/BooleanProperty
WEST_REDSTONE|west|enum|[up, side, none]|net/minecraft/world/level/block/state/properties/EnumProperty
WEST_WALL|west|enum|[none, low, tall]|net/minecraft/world/level/block/state/properties/EnumProperty
"#;

/// Exact enum value class and `serialized=CONSTANT` pairs for every `EnumProperty` field in 26.3
/// `BlockStateProperties`, captured from the real server classes with JDK 25 (`Property.getValueClass()`
/// + `Property.getName(v)` + `Enum.name()`).
#[rustfmt::skip]
pub(crate) const ENUM_VALUES: &str = r#"ATTACH_FACE|net/minecraft/world/level/block/state/properties/AttachFace|floor=FLOOR,wall=WALL,ceiling=CEILING
AXIS|net/minecraft/core/Direction$Axis|x=X,y=Y,z=Z
BAMBOO_LEAVES|net/minecraft/world/level/block/state/properties/BambooLeaves|none=NONE,small=SMALL,large=LARGE
BED_PART|net/minecraft/world/level/block/state/properties/BedPart|head=HEAD,foot=FOOT
BELL_ATTACHMENT|net/minecraft/world/level/block/state/properties/BellAttachType|floor=FLOOR,ceiling=CEILING,single_wall=SINGLE_WALL,double_wall=DOUBLE_WALL
CHEST_TYPE|net/minecraft/world/level/block/state/properties/ChestType|single=SINGLE,left=LEFT,right=RIGHT
COPPER_GOLEM_POSE|net/minecraft/world/level/block/CopperGolemStatueBlock$Pose|standing=STANDING,sitting=SITTING,running=RUNNING,star=STAR
CREAKING_HEART_STATE|net/minecraft/world/level/block/state/properties/CreakingHeartState|uprooted=UPROOTED,dormant=DORMANT,awake=AWAKE
DOOR_HINGE|net/minecraft/world/level/block/state/properties/DoorHingeSide|left=LEFT,right=RIGHT
DOUBLE_BLOCK_HALF|net/minecraft/world/level/block/state/properties/DoubleBlockHalf|upper=UPPER,lower=LOWER
EAST_REDSTONE|net/minecraft/world/level/block/state/properties/RedstoneSide|up=UP,side=SIDE,none=NONE
EAST_WALL|net/minecraft/world/level/block/state/properties/WallSide|none=NONE,low=LOW,tall=TALL
FACING|net/minecraft/core/Direction|north=NORTH,east=EAST,south=SOUTH,west=WEST,up=UP,down=DOWN
FACING_HOPPER|net/minecraft/core/Direction|down=DOWN,north=NORTH,south=SOUTH,west=WEST,east=EAST
HALF|net/minecraft/world/level/block/state/properties/Half|top=TOP,bottom=BOTTOM
HORIZONTAL_AXIS|net/minecraft/core/Direction$Axis|x=X,z=Z
HORIZONTAL_FACING|net/minecraft/core/Direction|north=NORTH,south=SOUTH,west=WEST,east=EAST
MODE_COMPARATOR|net/minecraft/world/level/block/state/properties/ComparatorMode|compare=COMPARE,subtract=SUBTRACT
NORTH_REDSTONE|net/minecraft/world/level/block/state/properties/RedstoneSide|up=UP,side=SIDE,none=NONE
NORTH_WALL|net/minecraft/world/level/block/state/properties/WallSide|none=NONE,low=LOW,tall=TALL
NOTEBLOCK_INSTRUMENT|net/minecraft/world/level/block/state/properties/NoteBlockInstrument|harp=HARP,basedrum=BASEDRUM,snare=SNARE,hat=HAT,bass=BASS,flute=FLUTE,bell=BELL,guitar=GUITAR,chime=CHIME,xylophone=XYLOPHONE,iron_xylophone=IRON_XYLOPHONE,cow_bell=COW_BELL,didgeridoo=DIDGERIDOO,bit=BIT,banjo=BANJO,pling=PLING,trumpet=TRUMPET,trumpet_exposed=TRUMPET_EXPOSED,trumpet_oxidized=TRUMPET_OXIDIZED,trumpet_weathered=TRUMPET_WEATHERED,zombie=ZOMBIE,skeleton=SKELETON,creeper=CREEPER,dragon=DRAGON,wither_skeleton=WITHER_SKELETON,piglin=PIGLIN,custom_head=CUSTOM_HEAD
ORIENTATION|net/minecraft/core/FrontAndTop|down_east=DOWN_EAST,down_north=DOWN_NORTH,down_south=DOWN_SOUTH,down_west=DOWN_WEST,up_east=UP_EAST,up_north=UP_NORTH,up_south=UP_SOUTH,up_west=UP_WEST,west_up=WEST_UP,east_up=EAST_UP,north_up=NORTH_UP,south_up=SOUTH_UP
PISTON_TYPE|net/minecraft/world/level/block/state/properties/PistonType|normal=DEFAULT,sticky=STICKY
POTENT_SULFUR_STATE|net/minecraft/world/level/block/state/properties/PotentSulfurState|dry=DRY,wet=WET,dormant=DORMANT,erupting=ERUPTING,continuous=CONTINUOUS
RAIL_SHAPE|net/minecraft/world/level/block/state/properties/RailShape|north_south=NORTH_SOUTH,east_west=EAST_WEST,ascending_east=ASCENDING_EAST,ascending_west=ASCENDING_WEST,ascending_north=ASCENDING_NORTH,ascending_south=ASCENDING_SOUTH,south_east=SOUTH_EAST,south_west=SOUTH_WEST,north_west=NORTH_WEST,north_east=NORTH_EAST
RAIL_SHAPE_STRAIGHT|net/minecraft/world/level/block/state/properties/RailShape|north_south=NORTH_SOUTH,east_west=EAST_WEST,ascending_east=ASCENDING_EAST,ascending_west=ASCENDING_WEST,ascending_north=ASCENDING_NORTH,ascending_south=ASCENDING_SOUTH
SCULK_SENSOR_PHASE|net/minecraft/world/level/block/state/properties/SculkSensorPhase|inactive=INACTIVE,active=ACTIVE,cooldown=COOLDOWN
SIDE_CHAIN_PART|net/minecraft/world/level/block/state/properties/SideChainPart|unconnected=UNCONNECTED,right=RIGHT,center=CENTER,left=LEFT
SLAB_TYPE|net/minecraft/world/level/block/state/properties/SlabType|top=TOP,bottom=BOTTOM,double=DOUBLE
SOUTH_REDSTONE|net/minecraft/world/level/block/state/properties/RedstoneSide|up=UP,side=SIDE,none=NONE
SOUTH_WALL|net/minecraft/world/level/block/state/properties/WallSide|none=NONE,low=LOW,tall=TALL
SPELEOTHEM_THICKNESS|net/minecraft/world/level/block/state/properties/SpeleothemThickness|tip_merge=TIP_MERGE,tip=TIP,frustum=FRUSTUM,middle=MIDDLE,base=BASE
STAIRS_SHAPE|net/minecraft/world/level/block/state/properties/StairsShape|straight=STRAIGHT,inner_left=INNER_LEFT,inner_right=INNER_RIGHT,outer_left=OUTER_LEFT,outer_right=OUTER_RIGHT
STRUCTUREBLOCK_MODE|net/minecraft/world/level/block/state/properties/StructureMode|save=SAVE,load=LOAD,corner=CORNER,data=DATA
TEST_BLOCK_MODE|net/minecraft/world/level/block/state/properties/TestBlockMode|start=START,log=LOG,fail=FAIL,accept=ACCEPT
TILT|net/minecraft/world/level/block/state/properties/Tilt|none=NONE,unstable=UNSTABLE,partial=PARTIAL,full=FULL
TRIAL_SPAWNER_STATE|net/minecraft/world/level/block/entity/trialspawner/TrialSpawnerState|inactive=INACTIVE,waiting_for_players=WAITING_FOR_PLAYERS,active=ACTIVE,waiting_for_reward_ejection=WAITING_FOR_REWARD_EJECTION,ejecting_reward=EJECTING_REWARD,cooldown=COOLDOWN
VAULT_STATE|net/minecraft/world/level/block/entity/vault/VaultState|inactive=INACTIVE,active=ACTIVE,unlocking=UNLOCKING,ejecting=EJECTING
VERTICAL_DIRECTION|net/minecraft/core/Direction|up=UP,down=DOWN
WEST_REDSTONE|net/minecraft/world/level/block/state/properties/RedstoneSide|up=UP,side=SIDE,none=NONE
WEST_WALL|net/minecraft/world/level/block/state/properties/WallSide|none=NONE,low=LOW,tall=TALL
"#;

/// Convert a serialized property value (e.g. `"north"`, `"true"`, `"3"`) for a captured
/// `BlockStateProperties.<field>` into the exact Java `Value` returned by `StateHolder.getValue`.
pub(crate) fn property_value(field: &str, kind: &str, serialized: &str) -> Option<crate::runtime::Value> {
    match kind {
        "boolean" => match serialized {
            "true" => Some(crate::runtime::Value::Boxed(crate::runtime::Boxed::Boolean(true))),
            "false" => Some(crate::runtime::Value::Boxed(crate::runtime::Boxed::Boolean(false))),
            _ => None,
        },
        "integer" => serialized
            .parse::<i32>()
            .ok()
            .map(|n| crate::runtime::Value::Boxed(crate::runtime::Boxed::Integer(n))),
        "enum" => ENUM_VALUES.lines().find_map(|line| {
            let mut parts = line.split('|');
            if parts.next()? != field {
                return None;
            }
            let class = parts.next()?;
            let pairs = parts.next()?;
            pairs.split(',').find_map(|pair| {
                let (ser, constant) = pair.split_once('=')?;
                (ser == serialized)
                    .then(|| crate::runtime::Value::Ref(Some(format!("enum/{class}/{constant}"))))
            })
        }),
        _ => None,
    }
}

/// Convert a Java enum constant marker (`enum/<class>/<CONSTANT>`) to its serialized property value
/// for `BlockStateProperties.<field>` (e.g. `PistonType.DEFAULT` -> `"normal"`).
pub(crate) fn enum_serialized_name(field: &str, enum_marker: &str) -> Option<String> {
    let rest = enum_marker.strip_prefix("enum/")?;
    let (class, constant) = rest.rsplit_once('/')?;
    ENUM_VALUES.lines().find_map(|line| {
        let mut parts = line.split('|');
        if parts.next()? != field {
            return None;
        }
        if parts.next()? != class {
            return None;
        }
        let pairs = parts.next()?;
        pairs.split(',').find_map(|pair| {
            let (ser, cname) = pair.split_once('=')?;
            (cname == constant).then(|| ser.to_owned())
        })
    })
}
