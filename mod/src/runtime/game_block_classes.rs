//! Loading the prepared 26.3 vanilla block class/interface closures.
//!
//! The table is generated from the player's own Minecraft installation by the
//! Java exporter in `tools/VanillaBlockClassClosureExporter.java`.  We keep the
//! result outside this repository: a prepared resource root (or an explicit
//! JSON path) supplies it to the native host.  Simple names from the older
//! `block-behaviours.json` exporter are deliberately rejected; they cannot
//! distinguish a jar class such as `demo.jar.FireBlock` from vanilla's class.

use std::{collections::BTreeMap, fs, path::Path, sync::Arc};

use serde_json::Value as Json;

pub(crate) type Table = Arc<BTreeMap<String, Vec<String>>>;

fn exact_names(value: &Json, context: &str) -> Result<Vec<String>, String> {
    let Json::Array(names) = value else {
        return Err(format!("{context}: closure must be an array"));
    };
    let mut out = Vec::with_capacity(names.len());
    for name in names {
        let Some(name) = name.as_str() else {
            return Err(format!("{context}: closure entry is not a string"));
        };
        if name.is_empty() || !name.contains('/') || name.contains(['.', ':']) {
            return Err(format!("{context}: {name:?} is not an internal class name"));
        }
        out.push(name.to_owned());
    }
    Ok(out)
}

fn table_from_map(map: &serde_json::Map<String, Json>) -> Result<Table, String> {
    let mut table = BTreeMap::new();
    for (id, classes) in map {
        if !id.starts_with("minecraft:") {
            return Err(format!("game block class table has non-vanilla id {id:?}"));
        }
        table.insert(id.clone(), exact_names(classes, id)?);
    }
    if table.is_empty() {
        return Err("game block class table is empty".into());
    }
    Ok(Arc::new(table))
}

fn table_from_behaviours(root: &Json) -> Result<Table, String> {
    let Json::Object(object) = root else {
        return Err("block-behaviours.json must be a JSON object".into());
    };
    let Some(Json::Array(blocks)) = object.get("blocks") else {
        return Err("block-behaviours.json has no blocks array".into());
    };
    let mut table = BTreeMap::new();
    for block in blocks {
        let Json::Object(block) = block else {
            return Err("block-behaviours.json has a non-object block".into());
        };
        let Some(id) = block.get("name").and_then(Json::as_str) else {
            return Err("block-behaviours.json block has no name".into());
        };
        let closure = block
            .get("class_closure")
            .or_else(|| block.get("classClosure"))
            .ok_or_else(|| format!("{id}: block-behaviours.json has no exact class_closure"))?;
        table.insert(id.to_owned(), exact_names(closure, id)?);
    }
    if table.is_empty() {
        return Err("block-behaviours.json blocks array is empty".into());
    }
    Ok(Arc::new(table))
}

/// Parse the canonical exporter output (`{"blocks": {"minecraft:fire": [...]}}`)
/// or its direct-map equivalent.
pub(crate) fn parse(text: &str) -> Result<Table, String> {
    let root: Json =
        serde_json::from_str(text).map_err(|error| format!("invalid JSON: {error}"))?;
    let Json::Object(object) = root else {
        return Err("game block class table must be a JSON object".into());
    };
    if let Some(Json::Object(blocks)) = object.get("blocks") {
        return table_from_map(blocks);
    }
    table_from_map(&object)
}

/// Read an exact closure file produced by the exporter.
pub(crate) fn from_file(path: &Path) -> Result<Table, String> {
    let text = fs::read_to_string(path).map_err(|error| {
        format!(
            "cannot read game block class table {}: {error}",
            path.display()
        )
    })?;
    let result = if path.file_name().and_then(|name| name.to_str()) == Some("block-behaviours.json")
    {
        let root: Json = serde_json::from_str(&text)
            .map_err(|error| format!("{}: invalid JSON: {error}", path.display()))?;
        table_from_behaviours(&root)
    } else {
        parse(&text)
    };
    result.map_err(|error| format!("{}: {error}", path.display()))
}

/// Find the closure artifact in a prepared 26.3 resource root.
pub(crate) fn from_prepared(root: &Path) -> Result<Table, String> {
    let candidates = [
        root.join("vanilla-block-classes.json"),
        root.join("block-behaviours-classes.json"),
        root.join("block-behaviours.json"),
    ];
    for path in candidates {
        if path.is_file() {
            let text = fs::read_to_string(&path).map_err(|error| {
                format!(
                    "cannot read game block class table {}: {error}",
                    path.display()
                )
            })?;
            let parsed = if path.file_name().and_then(|name| name.to_str())
                == Some("block-behaviours.json")
            {
                let root: Json = serde_json::from_str(&text)
                    .map_err(|error| format!("{}: invalid JSON: {error}", path.display()))?;
                table_from_behaviours(&root)
            } else {
                parse(&text)
            };
            return parsed.map_err(|error| format!("{}: {error}", path.display()));
        }
    }
    Err(format!(
        "prepared resources {} contain no vanilla-block-classes.json; generate it from the player's 26.3 install",
        root.display()
    ))
}

#[cfg(test)]
mod tests {
    use super::{parse, table_from_behaviours};

    #[test]
    fn accepts_internal_name_closures_and_rejects_simple_names() {
        let table = parse(
            r#"{"blocks":{"minecraft:fire":["net/minecraft/world/level/block/FireBlock","java/lang/Object"]}}"#,
        )
        .unwrap();
        assert_eq!(
            table["minecraft:fire"][0],
            "net/minecraft/world/level/block/FireBlock"
        );
        let error = parse(r#"{"blocks":{"minecraft:fire":["FireBlock"]}}"#).unwrap_err();
        assert!(error.contains("internal class name"), "{error}");
        let behaviours = serde_json::json!({
            "blocks": [{
                "name": "minecraft:fire",
                "class_closure": ["net/minecraft/world/level/block/FireBlock"]
            }]
        });
        assert!(table_from_behaviours(&behaviours).is_ok());
    }
}
