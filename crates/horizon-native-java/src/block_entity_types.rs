use crate::{game_object, MethodRef, Value};
const OWNER: &str = "net/minecraft/world/level/block/entity/BlockEntityTypes";
const DESC: &str = "Lnet/minecraft/world/level/block/entity/BlockEntityType;";
const FIELDS: &[(&str,&str)] = &[("CHEST","chest"),("TRAPPED_CHEST","trapped_chest"),("ENDER_CHEST","ender_chest"),("SHULKER_BOX","shulker_box"),("BELL","bell"),("SIGN","sign"),("DECORATED_POT","decorated_pot"),("ENCHANTING_TABLE","enchanting_table"),("LECTERN","lectern")];
pub(crate) fn static_field(r:&MethodRef)->Option<Value>{ if r.owner!=OWNER || r.descriptor!=DESC{return None} let id=FIELDS.iter().find(|(n,_)|*n==r.name)?.1; Some(game_object::client_value(format!("client/block_entity_type/minecraft:{id}"))) }
