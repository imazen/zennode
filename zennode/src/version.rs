//! Version management for node registries.

use alloc::collections::BTreeMap;

/// A version set maps node IDs to their schema versions.
///
/// Aggregating crates use this to pin the API surface:
/// "in API v3, `zenfilters.exposure` uses schema v2".
#[derive(Clone, Debug, Default)]
pub struct VersionSet {
    versions: BTreeMap<&'static str, u32>,
    /// The public-facing API version number that users see.
    pub api_version: u32,
}

impl VersionSet {
    /// Create a new version set for the given API version.
    pub fn new(api_version: u32) -> Self {
        Self {
            versions: BTreeMap::new(),
            api_version,
        }
    }

    /// Set the schema version for a node ID.
    pub fn set(&mut self, id: &'static str, version: u32) {
        self.versions.insert(id, version);
    }

    /// Get the schema version for a node ID.
    pub fn get(&self, id: &str) -> Option<u32> {
        self.versions.get(id).copied()
    }
}
