use crate::{jar_object, MethodRef, NativeError, Value};
use std::collections::BTreeMap;

const OWNER: &str = "net/minecraft/world/item/ToolMaterial";
#[derive(Debug, Clone, Copy)] struct Row { durability:i32, speed:f64, attack:f64, enchant:i32 }
#[derive(Debug, Default, Clone)] pub(crate) struct State { handles:BTreeMap<String, Value> }
fn row(name:&str)->Option<Row>{Some(match name {"WOOD"=>Row{durability:59,speed:2.0,attack:0.0,enchant:15},"STONE"=>Row{durability:131,speed:4.0,attack:1.0,enchant:5},"COPPER"=>Row{durability:190,speed:5.0,attack:1.0,enchant:13},"IRON"=>Row{durability:250,speed:6.0,attack:2.0,enchant:14},"DIAMOND"=>Row{durability:1561,speed:8.0,attack:3.0,enchant:10},"GOLD"=>Row{durability:32,speed:12.0,attack:0.0,enchant:22},"NETHERITE"=>Row{durability:2031,speed:9.0,attack:4.0,enchant:15},_=>return None})}
pub(crate) fn static_field(state:&mut State,r:&MethodRef)->Option<Result<Value,NativeError>>{if r.owner!=OWNER||r.descriptor!="Lnet/minecraft/world/item/ToolMaterial;"{return None} if row(&r.name).is_none(){return None} let v=state.handles.entry(r.name.clone()).or_insert_with(||jar_object::handle(format!("{OWNER}#{}",r.name)));Some(Ok(v.clone()))}
pub(crate) fn instance_call(_state:&mut State,r:&MethodRef,receiver:&Value,_args:&[Value])->Option<Result<Value,NativeError>>{let key=crate::jar_object::instance_key(receiver)?;let name=key.rsplit_once('#')?.1;let row=row(name)?;if r.owner!=OWNER{return None} Some(Ok(match r.name.as_str(){"durability"=>Value::Int(row.durability),"speed"=>Value::Float(row.speed),"attackDamageBonus"=>Value::Float(row.attack),"enchantmentValue"=>Value::Int(row.enchant),_=>return None})))}
