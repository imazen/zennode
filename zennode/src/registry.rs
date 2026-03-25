//! Node registry — aggregation point for node definitions.

use alloc::boxed::Box;
use alloc::string::ToString;
use alloc::vec::Vec;

use crate::error::NodeError;
use crate::kv::{KvPairs, KvWarning, KvWarningKind};
use crate::param::ParamMap;
use crate::schema::NodeGroup;
use crate::traits::{NodeDef, NodeInstance};

/// Aggregation point for node definitions from across the ecosystem.
///
/// Aggregating crates (zenimage, imageflow4) populate a registry
/// and expose it for UI generation, serialization, and RIAPI parsing.
pub struct NodeRegistry {
    nodes: Vec<&'static dyn NodeDef>,
}

/// Result of parsing a querystring through the registry.
pub struct KvResult {
    /// Node instances created from consumed keys.
    pub instances: Vec<Box<dyn NodeInstance>>,
    /// Warnings from parsing (unrecognized keys, invalid values, etc.).
    pub warnings: Vec<KvWarning>,
}

impl NodeRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    /// Register a single node definition.
    pub fn register(&mut self, def: &'static dyn NodeDef) {
        self.nodes.push(def);
    }

    /// Register multiple node definitions at once.
    pub fn register_all(&mut self, defs: &[&'static dyn NodeDef]) {
        self.nodes.extend_from_slice(defs);
    }

    /// Look up a node definition by its ID.
    pub fn get(&self, id: &str) -> Option<&'static dyn NodeDef> {
        self.nodes.iter().find(|n| n.schema().id == id).copied()
    }

    /// All registered node definitions.
    pub fn all(&self) -> &[&'static dyn NodeDef] {
        &self.nodes
    }

    /// All nodes in a particular group.
    pub fn by_group(&self, group: NodeGroup) -> Vec<&'static dyn NodeDef> {
        self.nodes
            .iter()
            .filter(|n| n.schema().group == group)
            .copied()
            .collect()
    }

    /// All nodes that have a particular tag.
    pub fn by_tag(&self, tag: &str) -> Vec<&'static dyn NodeDef> {
        self.nodes
            .iter()
            .filter(|n| n.schema().tags.contains(&tag))
            .copied()
            .collect()
    }

    /// Create a node instance by ID and parameter map.
    pub fn create(&self, id: &str, params: &ParamMap) -> Result<Box<dyn NodeInstance>, NodeError> {
        let def = self
            .get(id)
            .ok_or_else(|| NodeError::UnknownNode(id.to_string()))?;
        def.create(params)
    }

    /// Parse a RIAPI querystring against all registered nodes.
    ///
    /// Each node definition attempts to consume its relevant keys.
    /// Returns all created instances plus any warnings.
    pub fn from_querystring(&self, qs: &str) -> KvResult {
        let mut kv = KvPairs::from_querystring(qs);
        let mut instances = Vec::new();

        for def in &self.nodes {
            match def.from_kv(&mut kv) {
                Ok(Some(instance)) => instances.push(instance),
                Ok(None) => {}
                Err(e) => {
                    kv.warn(
                        def.schema().id,
                        KvWarningKind::InvalidValue,
                        alloc::format!("error creating {}: {e}", def.schema().id),
                    );
                }
            }
        }

        // Warn about unconsumed keys
        let mut warnings: Vec<KvWarning> = kv.warnings().to_vec();
        for (key, _value) in kv.unconsumed() {
            warnings.push(KvWarning {
                key: key.to_string(),
                kind: KvWarningKind::UnrecognizedKey,
                message: alloc::format!("unrecognized key: {key}"),
            });
        }

        KvResult {
            instances,
            warnings,
        }
    }
}

impl Default for NodeRegistry {
    fn default() -> Self {
        Self::new()
    }
}
