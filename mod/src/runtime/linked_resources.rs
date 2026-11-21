//! A linked jar's resources, found by `Class.getResourceAsStream` and
//! `getResource` the way a JVM class path finds them.
//!
//! A JVM's class loader looks a resource up across every jar on the class
//! path, the running jar first and then the others in order, and the first jar
//! that holds the name answers. The host used to answer only the running jar's
//! own resources, so a linked library reading its bundled defaults got `null`:
//! Forge Config API Port's `ForgeConfigApiPortConfig.loadFrom` does
//! `Objects.requireNonNull(getClass().getResourceAsStream("/forgeconfigapiport.toml"))`
//! and failed with an NPE in every Fabric jar that links it.
//!
//! The linked table keeps each linked jar's resources (the bytes `NativeJar`
//! already bounds: 4 MiB a resource, 32 MiB a jar) under the first jar that
//! names them, as it keeps classes. The running jar's own resources still
//! answer first.

use super::*;

impl LinkedClassTable {
    /// Keep `jar`'s resources whose names no earlier jar in the table took (for classpath lookup),
    /// and record `jar`'s `data/` resources as an ordered datapack layer for recipe/tag loading.
    pub(crate) fn link_resources(&mut self, id: &str, jar: &NativeJar) {
        let mut data_layer = BTreeMap::new();
        for (name, bytes) in &jar.resource_bytes {
            if name.ends_with(".class") {
                continue;
            }
            if name.starts_with("data/") {
                data_layer.insert(name.clone(), bytes.clone());
            }
            self.resources
                .entry(name.clone())
                .or_insert_with(|| (format!("resource-bytes/{name}"), bytes.clone()));
        }
        if !data_layer.is_empty() {
            self.data_layers.push((id.to_owned(), data_layer));
        }
    }

    /// The marker and bytes of a resource a linked jar holds.
    pub(crate) fn resource(&self, name: &str) -> Option<(&str, &[u8])> {
        self.resources
            .get(name)
            .map(|(marker, bytes)| (marker.as_str(), bytes.as_slice()))
    }

    /// Ordered `data/` resource layers across all linked jars (lowest to highest priority).
    pub(crate) fn data_layers(&self) -> &[(String, BTreeMap<String, Vec<u8>>)] {
        &self.data_layers
    }

    /// Total `data/` entries across all linked jar layers (used for cache invalidation).
    pub(crate) fn data_layer_entry_count(&self) -> usize {
        self.data_layers.iter().map(|(_, m)| m.len()).sum()
    }
}
