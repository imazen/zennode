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

    /// Render all registered nodes as a Markdown reference document.
    pub fn to_markdown(&self) -> alloc::string::String {
        use alloc::fmt::Write;
        let mut md = alloc::string::String::from("# Node Reference\n\n");
        let _ = write!(md, "{} nodes registered.\n\n", self.nodes.len());
        for def in &self.nodes {
            md.push_str(&def.schema().to_markdown());
        }
        md
    }
}

/// JSON conversion methods (requires `serde` feature).
#[cfg(feature = "serde")]
impl NodeRegistry {
    /// Look up a node definition by its JSON key (or ID as fallback).
    pub fn get_by_json_key(&self, key: &str) -> Option<&'static dyn NodeDef> {
        self.nodes
            .iter()
            .find(|n| n.schema().effective_json_key() == key)
            .copied()
    }

    /// Deserialize a wrapped node: `{"constrain": {"mode": "fit", "w": 800}}`.
    ///
    /// The JSON value must be an object with exactly one key, which identifies
    /// the node type via its `json_key` (or `id` as fallback).
    pub fn node_from_json(
        &self,
        json: &serde_json::Value,
    ) -> Result<Box<dyn NodeInstance>, NodeError> {
        let obj = json
            .as_object()
            .ok_or_else(|| NodeError::Other("expected JSON object".into()))?;
        if obj.len() != 1 {
            return Err(NodeError::Other(alloc::format!(
                "expected exactly one key, got {}",
                obj.len()
            )));
        }
        let (key, inner) = obj.iter().next().unwrap();

        let def = self
            .get_by_json_key(key)
            .ok_or_else(|| NodeError::UnknownNode(key.clone()))?;
        let schema = def.schema();

        let inner_obj = inner
            .as_object()
            .ok_or_else(|| NodeError::Other("node params must be a JSON object".into()))?;

        // Check for unknown fields if deny_unknown_fields is set
        if schema.deny_unknown_fields {
            for json_key in inner_obj.keys() {
                let known = schema.params.iter().any(|p| p.matches_json_key(json_key));
                if !known {
                    return Err(NodeError::UnknownParam {
                        node: schema.id,
                        param: json_key.clone(),
                    });
                }
            }
        }

        // Convert JSON to ParamMap using schema metadata
        let param_map = self.json_to_param_map(schema, inner_obj)?;
        def.create(&param_map)
    }

    /// Serialize a node instance as `{"json_key": {...params...}}`.
    ///
    /// `ParamValue::None` is omitted from the output (skip-serializing-if-none).
    pub fn node_to_json(&self, node: &dyn NodeInstance) -> serde_json::Value {
        let schema = node.schema();
        let params = node.to_params();
        let inner = self.param_map_to_json(schema, &params);
        serde_json::json!({ schema.effective_json_key(): inner })
    }

    /// Deserialize a pipeline: `[{"constrain": {...}}, {"encode": {...}}]`.
    pub fn pipeline_from_json(
        &self,
        json: &serde_json::Value,
    ) -> Result<Vec<Box<dyn NodeInstance>>, NodeError> {
        let arr = json
            .as_array()
            .ok_or_else(|| NodeError::Other("expected JSON array for pipeline".into()))?;
        arr.iter().map(|item| self.node_from_json(item)).collect()
    }

    /// Serialize a pipeline to JSON array.
    pub fn pipeline_to_json(&self, nodes: &[Box<dyn NodeInstance>]) -> serde_json::Value {
        let arr: Vec<serde_json::Value> = nodes
            .iter()
            .map(|n| self.node_to_json(n.as_ref()))
            .collect();
        serde_json::Value::Array(arr)
    }

    /// Convert a JSON object to a ParamMap using schema for type resolution.
    fn json_to_param_map(
        &self,
        schema: &crate::schema::NodeSchema,
        obj: &serde_json::Map<alloc::string::String, serde_json::Value>,
    ) -> Result<ParamMap, NodeError> {
        use crate::param::ParamValue;
        let mut map = ParamMap::new();

        for param in schema.params {
            // Find the JSON value by json_name, name, or aliases
            let json_key = param.effective_json_name();
            let value = obj
                .get(json_key)
                .or_else(|| {
                    if json_key != param.name {
                        obj.get(param.name)
                    } else {
                        None
                    }
                })
                .or_else(|| param.json_aliases.iter().find_map(|a| obj.get(*a)));

            let Some(value) = value else { continue };

            if value.is_null() {
                map.insert(param.name.into(), ParamValue::None);
                continue;
            }

            let pv = match &param.kind {
                crate::schema::ParamKind::Float { .. } => {
                    value.as_f64().map(|v| ParamValue::F32(v as f32))
                }
                crate::schema::ParamKind::Int { .. } => value
                    .as_i64()
                    .and_then(|v| i32::try_from(v).ok())
                    .map(ParamValue::I32),
                crate::schema::ParamKind::U32 { .. } => value
                    .as_u64()
                    .and_then(|v| u32::try_from(v).ok())
                    .map(ParamValue::U32),
                crate::schema::ParamKind::Bool { .. } => value.as_bool().map(ParamValue::Bool),
                crate::schema::ParamKind::Str { .. } => {
                    value.as_str().map(|s| ParamValue::Str(s.into()))
                }
                crate::schema::ParamKind::Enum { .. } => {
                    value.as_str().map(|s| ParamValue::Str(s.into()))
                }
                crate::schema::ParamKind::Json { .. }
                | crate::schema::ParamKind::Object { .. }
                | crate::schema::ParamKind::TaggedUnion { .. } => {
                    Some(ParamValue::Json(value.to_string()))
                }
                crate::schema::ParamKind::FloatArray { len, .. } => {
                    value.as_array().and_then(|arr| {
                        if arr.len() == *len {
                            let floats: Vec<f32> = arr
                                .iter()
                                .filter_map(|v| v.as_f64().map(|f| f as f32))
                                .collect();
                            if floats.len() == *len {
                                Some(ParamValue::F32Array(floats))
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                }
                crate::schema::ParamKind::Color { .. } => value.as_array().and_then(|arr| {
                    if arr.len() == 4 {
                        let f: Vec<f32> = arr
                            .iter()
                            .filter_map(|v| v.as_f64().map(|f| f as f32))
                            .collect();
                        if f.len() == 4 {
                            Some(ParamValue::Color([f[0], f[1], f[2], f[3]]))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }),
                // Future ParamKind variants: serialize as string
                #[allow(unreachable_patterns)]
                _ => Some(ParamValue::Str(value.to_string())),
            };

            if let Some(pv) = pv {
                map.insert(param.name.into(), pv);
            }
        }

        Ok(map)
    }

    /// Convert a ParamMap to a JSON object, skipping None values.
    fn param_map_to_json(
        &self,
        schema: &crate::schema::NodeSchema,
        params: &ParamMap,
    ) -> serde_json::Value {
        use crate::param::ParamValue;
        let mut obj = serde_json::Map::new();

        for param in schema.params {
            let Some(value) = params.get(param.name) else {
                continue;
            };
            if value.is_none() {
                continue; // skip-serializing-if-none (default behavior)
            }

            let json_key = param.effective_json_name();
            let jv = match value {
                ParamValue::None => unreachable!(),
                ParamValue::F32(v) => serde_json::json!(*v),
                ParamValue::I32(v) => serde_json::json!(*v),
                ParamValue::U32(v) => serde_json::json!(*v),
                ParamValue::Bool(v) => serde_json::json!(*v),
                ParamValue::Str(v) | ParamValue::Enum(v) => serde_json::json!(v),
                ParamValue::F32Array(v) => serde_json::json!(v),
                ParamValue::Color(v) => serde_json::json!(v),
                ParamValue::Json(text) => {
                    // Parse JSON text and embed as structured value
                    serde_json::from_str(text).unwrap_or(serde_json::Value::Null)
                }
            };
            obj.insert(json_key.into(), jv);
        }

        serde_json::Value::Object(obj)
    }
}

impl Default for NodeRegistry {
    fn default() -> Self {
        Self::new()
    }
}
