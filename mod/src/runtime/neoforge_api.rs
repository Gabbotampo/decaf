//! Small, exact answers for the NeoForge API's loader state.
//!
//! These calls are safe to answer because the native runtime owns the active
//! mod id while it runs a constructor or lifecycle callback.  Calls which
//! need the real game or loader state continue through the counted boundary.

use crate::runtime::{Host, MethodRef, StdHost, Value, host_service};

const MOD_LOADING_CONTEXT: &str = "net/neoforged/fml/ModLoadingContext";
const FML_LOADER: &str = "net/neoforged/fml/loading/FMLLoader";

pub(crate) fn static_call(_host: &mut StdHost, r: &MethodRef) -> Option<Value> {
    match (r.owner.as_str(), r.name.as_str(), r.descriptor.as_str()) {
        (MOD_LOADING_CONTEXT, "get", "()Lnet/neoforged/fml/ModLoadingContext;") => {
            Some(host_service::loader_singleton(MOD_LOADING_CONTEXT))
        }
        ("net/neoforged/fml/loading/DatagenModLoader", "isRunningDataGen", "()Z") => {
            Some(Value::Int(0))
        }
        (FML_LOADER, "isProduction", "()Z") => Some(Value::Int(1)),
        (FML_LOADER, "getGameDir", "()Ljava/nio/file/Path;") => {
            Some(crate::runtime::host_path::path_value("/horizon/game"))
        }
        _ => None,
    }
}

pub(crate) fn instance_call(host: &mut StdHost, r: &MethodRef, receiver: &Value) -> Option<Value> {
    let key = host_service::loader_key(receiver)?;
    match (
        key,
        r.owner.as_str(),
        r.name.as_str(),
        r.descriptor.as_str(),
    ) {
        (
            MOD_LOADING_CONTEXT,
            MOD_LOADING_CONTEXT,
            "getActiveContainer",
            "()Lnet/neoforged/fml/ModContainer;",
        ) => {
            let id = host
                .neoforge_imc()
                .and_then(|(imc, _)| {
                    let active = crate::runtime::neoforge_imc::active_id(imc);
                    active.map(str::to_owned)
                })
                .unwrap_or_else(|| "minecraft".to_owned());
            Some(host.neoforge_container(&id))
        }
        (
            MOD_LOADING_CONTEXT,
            MOD_LOADING_CONTEXT,
            "getActiveNamespace",
            "()Ljava/lang/String;",
        ) => Some(Value::Ref(Some(
            host.neoforge_imc()
                .and_then(|(imc, _)| {
                    let active = crate::runtime::neoforge_imc::active_id(imc);
                    active.map(str::to_owned)
                })
                .unwrap_or_else(|| "minecraft".to_owned()),
        ))),
        (FML_LOADER, FML_LOADER, "getGameDir", "()Ljava/nio/file/Path;") => {
            Some(crate::runtime::host_path::path_value("/horizon/game"))
        }
        _ => None,
    }
}
