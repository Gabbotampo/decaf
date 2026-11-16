//! Vanilla registry `ResourceKey` constants used by 26.3 mod bootstrap.
use crate::runtime::{MethodRef, Value};
const ENCHANTMENTS: &str = "net/minecraft/world/item/enchantment/Enchantments";
const KEY: &str = "Lnet/minecraft/resources/ResourceKey;";
fn id(name: &str) -> String { name.to_ascii_lowercase() }
pub(crate) fn static_field(r: &MethodRef) -> Option<Value> {
    if r.owner != ENCHANTMENTS || r.descriptor != KEY { return None; }
    if r.name.is_empty() || !r.name.bytes().all(|b| b.is_ascii_uppercase() || b == b'_') { return None; }
    Some(crate::runtime::host_ids::resource_key_named("minecraft:enchantment", &id(&r.name)))
}
#[cfg(test)]
mod tests {
 use super::*;
 #[test] fn canonical_enchantment_key_is_interned() {
  let r=MethodRef{owner:ENCHANTMENTS.into(),name:"KNOCKBACK".into(),descriptor:KEY.into()};
  assert_eq!(static_field(&r), Some(crate::runtime::host_ids::resource_key_named("minecraft:enchantment","knockback")));
 }
}
