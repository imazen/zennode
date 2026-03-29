//! JSON Schema generation from node schemas.
#![allow(unreachable_patterns)]
//!
//! Generates JSON Schema 2020-12 documents with `x-zennode-*` extensions
//! for slider mappings, units, sections, identity values, and pipeline metadata.

extern crate std;

use serde_json::{Value, json};

use crate::registry::NodeRegistry;
use crate::schema::{NodeSchema, ParamDesc, ParamKind};

/// Generate a JSON Schema document for a single node.
pub fn node_to_json_schema(schema: &NodeSchema) -> Value {
    let mut properties = serde_json::Map::new();
    let required: std::vec::Vec<&str> = std::vec::Vec::new();

    for param in schema.params {
        properties.insert(param.name.into(), param_to_schema(param));
    }

    let mut node_schema = json!({
        "type": "object",
        "title": schema.label,
        "description": schema.description,
        "properties": properties,
        "additionalProperties": false,
        "x-zennode-id": schema.id,
        "x-zennode-group": serde_json::to_value(schema.group).unwrap_or(Value::Null),
        "x-zennode-role": serde_json::to_value(schema.role).unwrap_or(Value::Null),
        "x-zennode-version": schema.version,
        "x-zennode-compat-version": schema.compat_version,
    });

    if !schema.tags.is_empty() {
        node_schema["x-zennode-tags"] = json!(schema.tags);
    }

    if let Some(ref coalesce) = schema.coalesce {
        node_schema["x-zennode-coalesce"] = serde_json::to_value(coalesce).unwrap_or(Value::Null);
    }

    node_schema["x-zennode-format"] = serde_json::to_value(schema.format).unwrap_or(Value::Null);

    if !schema.inputs.is_empty() {
        let ports: std::vec::Vec<Value> = schema
            .inputs
            .iter()
            .map(|p| {
                json!({
                    "name": p.name,
                    "label": p.label,
                    "edge_kind": match p.edge_kind {
                        crate::schema::EdgeKind::Input => "input",
                        crate::schema::EdgeKind::Canvas => "canvas",
                    },
                    "required": p.required,
                    "variadic": p.variadic,
                    "from_io_id": p.from_io_id,
                })
            })
            .collect();
        node_schema["x-zennode-inputs"] = json!(ports);
    }

    if !required.is_empty() {
        node_schema["required"] = json!(required);
    }

    node_schema
}

/// Generate a JSON Schema document with `$defs` for all registered nodes.
pub fn registry_to_json_schema(registry: &NodeRegistry) -> Value {
    let mut defs = serde_json::Map::new();

    for def in registry.all() {
        let schema = def.schema();
        defs.insert(schema.id.into(), node_to_json_schema(schema));
    }

    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$defs": defs,
    })
}

/// Generate OpenAPI 3.1 `components/schemas` section from a registry.
pub fn registry_to_openapi_schemas(registry: &NodeRegistry) -> Value {
    let mut schemas = serde_json::Map::new();

    for def in registry.all() {
        let schema = def.schema();
        // Use dotted ID but replace dots with underscores for OpenAPI compatibility
        let key = schema.id.replace('.', "_");
        schemas.insert(key, node_to_json_schema(schema));
    }

    json!({ "schemas": schemas })
}

// ─── Querystring key registry ───

/// A single RIAPI querystring key with its metadata.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct QsKey {
    /// The querystring key (e.g., "w", "jpeg.quality").
    pub key: &'static str,
    /// The node ID that consumes this key (e.g., "zenresize.constrain").
    pub node_id: &'static str,
    /// The node label (e.g., "Constrain Size").
    pub node_label: &'static str,
    /// The parameter name on the node (e.g., "w").
    pub param_name: &'static str,
    /// The parameter label (e.g., "Width").
    pub param_label: &'static str,
    /// The parameter description.
    pub param_description: &'static str,
    /// JSON Schema for the parameter value.
    pub value_schema: Value,
    /// Whether this key is an alias (not the primary key for this param).
    pub is_alias: bool,
    /// The primary key if this is an alias, or self if primary.
    pub primary_key: &'static str,
}

/// Extract all RIAPI querystring keys from a node registry.
///
/// Returns a flat list of every key handled by any registered node,
/// with full metadata about what each key does, its value type/range,
/// and which node/parameter it maps to.
pub fn registry_querystring_keys(registry: &NodeRegistry) -> std::vec::Vec<QsKey> {
    let mut keys = std::vec::Vec::new();

    for def in registry.all() {
        let schema = def.schema();
        for param in schema.params {
            if param.kv_keys.is_empty() {
                continue;
            }
            let value_schema = param_value_schema(param);
            let primary = param.kv_keys[0];

            for (i, &kv_key) in param.kv_keys.iter().enumerate() {
                keys.push(QsKey {
                    key: kv_key,
                    node_id: schema.id,
                    node_label: schema.label,
                    param_name: param.name,
                    param_label: param.label,
                    param_description: param.description,
                    value_schema: value_schema.clone(),
                    is_alias: i > 0,
                    primary_key: primary,
                });
            }
        }
    }

    keys
}

/// Generate a JSON Schema for validating RIAPI querystrings.
///
/// The schema describes a flat object where each property is a querystring key,
/// with the value's type, range, and default derived from the node parameter.
/// All properties are optional (querystrings are sparse by nature).
pub fn querystring_to_json_schema(registry: &NodeRegistry) -> Value {
    let keys = registry_querystring_keys(registry);
    let mut properties = serde_json::Map::new();

    for qk in &keys {
        let mut prop = qk.value_schema.clone();
        prop["title"] = json!(qk.param_label);
        if !qk.param_description.is_empty() {
            prop["description"] = json!(qk.param_description);
        }
        prop["x-zennode-node"] = json!(qk.node_id);
        prop["x-zennode-param"] = json!(qk.param_name);
        if qk.is_alias {
            prop["x-zennode-alias-of"] = json!(qk.primary_key);
        }
        properties.insert(qk.key.into(), prop);
    }

    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "title": "RIAPI Querystring",
        "description": "Querystring parameters for image processing. All properties are optional.",
        "type": "object",
        "properties": properties,
        "additionalProperties": true,
    })
}

/// Generate a structured querystring key registry as JSON.
///
/// Groups keys by node, with metadata for each key. Useful for documentation
/// generation, client SDK hints, and IDE autocomplete.
pub fn querystring_key_registry(registry: &NodeRegistry) -> Value {
    let keys = registry_querystring_keys(registry);

    // Group by node_id.
    let mut by_node: std::collections::BTreeMap<&str, std::vec::Vec<&QsKey>> =
        std::collections::BTreeMap::new();
    for qk in &keys {
        by_node.entry(qk.node_id).or_default().push(qk);
    }

    let mut nodes = serde_json::Map::new();
    for (node_id, node_keys) in &by_node {
        let node_label = node_keys[0].node_label;
        let params: std::vec::Vec<Value> = node_keys
            .iter()
            .filter(|k| !k.is_alias)
            .map(|k| {
                let aliases: std::vec::Vec<&str> = node_keys
                    .iter()
                    .filter(|a| a.param_name == k.param_name && a.is_alias)
                    .map(|a| a.key)
                    .collect();
                let mut entry = json!({
                    "key": k.key,
                    "param": k.param_name,
                    "label": k.param_label,
                    "value_schema": k.value_schema,
                });
                if !k.param_description.is_empty() {
                    entry["description"] = json!(k.param_description);
                }
                if !aliases.is_empty() {
                    entry["aliases"] = json!(aliases);
                }
                entry
            })
            .collect();

        nodes.insert(
            (*node_id).into(),
            json!({
                "label": node_label,
                "keys": params,
            }),
        );
    }

    json!({
        "version": 1,
        "nodes": nodes,
    })
}

/// Generate a bare value schema for a parameter (no title/description/extensions).
/// Used for querystring key schemas where we only need the value type.
fn param_value_schema(param: &ParamDesc) -> Value {
    match &param.kind {
        ParamKind::Float { min, max, default, .. } => json!({
            "type": "number", "minimum": min, "maximum": max, "default": default,
        }),
        ParamKind::Int { min, max, default } => json!({
            "type": "integer", "minimum": min, "maximum": max, "default": default,
        }),
        ParamKind::U32 { min, max, default } => json!({
            "type": "integer", "minimum": min, "maximum": max, "default": default,
        }),
        ParamKind::Bool { default } => json!({
            "type": "boolean", "default": default,
        }),
        ParamKind::Str { default } => json!({
            "type": "string", "default": default,
        }),
        ParamKind::Enum { variants, default } => {
            let names: std::vec::Vec<&str> = variants.iter().map(|v| v.name).collect();
            json!({ "type": "string", "enum": names, "default": default })
        }
        _ => json!({ "type": "string" }),
    }
}

fn param_to_schema(param: &ParamDesc) -> Value {
    let mut schema = match &param.kind {
        ParamKind::Float {
            min,
            max,
            default,
            identity,
            step,
        } => {
            let mut s = json!({
                "type": "number",
                "minimum": min,
                "maximum": max,
                "default": default,
            });
            s["x-zennode-identity"] = json!(identity);
            s["x-zennode-step"] = json!(step);
            s
        }
        ParamKind::Int { min, max, default } => json!({
            "type": "integer",
            "minimum": min,
            "maximum": max,
            "default": default,
        }),
        ParamKind::U32 { min, max, default } => json!({
            "type": "integer",
            "minimum": min,
            "maximum": max,
            "default": default,
        }),
        ParamKind::Bool { default } => json!({
            "type": "boolean",
            "default": default,
        }),
        ParamKind::Str { default } => json!({
            "type": "string",
            "default": default,
        }),
        ParamKind::Enum { variants, default } => {
            let names: std::vec::Vec<&str> = variants.iter().map(|v| v.name).collect();
            let labels: std::vec::Vec<Value> = variants
                .iter()
                .map(|v| {
                    json!({
                        "name": v.name,
                        "label": v.label,
                        "description": v.description,
                    })
                })
                .collect();
            json!({
                "type": "string",
                "enum": names,
                "default": default,
                "x-zennode-enum-labels": labels,
            })
        }
        ParamKind::FloatArray {
            len,
            min,
            max,
            default,
            labels,
        } => json!({
            "type": "array",
            "items": { "type": "number", "minimum": min, "maximum": max },
            "minItems": len,
            "maxItems": len,
            "default": std::vec![*default; *len],
            "x-zennode-labels": labels,
        }),
        ParamKind::Color { default } => json!({
            "type": "array",
            "items": { "type": "number" },
            "minItems": 4,
            "maxItems": 4,
            "default": default,
            "x-zennode-color": true,
        }),
        ParamKind::Json {
            json_schema,
            default_json,
        } => {
            // Parse and embed the JSON Schema fragment directly
            let mut s = serde_json::from_str::<Value>(json_schema).unwrap_or_else(|_| json!({}));
            if !default_json.is_empty() {
                if let Ok(def) = serde_json::from_str::<Value>(default_json) {
                    s["default"] = def;
                }
            }
            s
        }
        ParamKind::Object { params } => {
            let mut properties = serde_json::Map::new();
            for sub_param in *params {
                properties.insert(
                    sub_param.effective_json_name().into(),
                    param_to_schema(sub_param),
                );
            }
            json!({
                "type": "object",
                "properties": properties,
            })
        }
        ParamKind::TaggedUnion { variants } => {
            // Externally-tagged serde format: {"variant_name": {...fields...}} or "variant_name"
            let one_of: std::vec::Vec<Value> = variants
                .iter()
                .map(|v| {
                    if v.params.is_empty() {
                        // Unit variant: just the tag string
                        json!({ "const": v.tag })
                    } else {
                        // Struct variant: {"tag": {"field": type, ...}}
                        let mut inner_props = serde_json::Map::new();
                        for p in v.params {
                            inner_props.insert(p.effective_json_name().into(), param_to_schema(p));
                        }
                        let required: std::vec::Vec<&str> = v
                            .params
                            .iter()
                            .filter(|p| !p.optional)
                            .map(|p| p.effective_json_name())
                            .collect();
                        let mut inner = json!({
                            "type": "object",
                            "properties": inner_props,
                        });
                        if !required.is_empty() {
                            inner["required"] = json!(required);
                        }
                        json!({
                            "type": "object",
                            "properties": { v.tag: inner },
                            "required": [v.tag],
                        })
                    }
                })
                .collect();
            json!({ "oneOf": one_of })
        }
        _ => json!({ "type": "string" }),
    };

    // Add common extensions
    schema["title"] = json!(param.label);
    if !param.description.is_empty() {
        schema["description"] = json!(param.description);
    }
    if param.optional {
        // Wrap type in oneOf to allow null
        if let Some(base_type) = schema.get("type").cloned() {
            schema["type"] = json!([base_type, "null"]);
        }
        schema["x-zennode-optional"] = json!(true);
    }
    if !param.unit.is_empty() {
        schema["x-zennode-unit"] = json!(param.unit);
    }
    if !param.section.is_empty() {
        schema["x-zennode-section"] = json!(param.section);
    }
    schema["x-zennode-slider"] = serde_json::to_value(param.slider).unwrap_or(Value::Null);
    if !param.kv_keys.is_empty() {
        schema["x-zennode-kv-keys"] = json!(param.kv_keys);
    }
    if param.since_version > 1 {
        schema["x-zennode-since-version"] = json!(param.since_version);
    }
    if !param.visible_when.is_empty() {
        schema["x-zennode-visible-when"] = json!(param.visible_when);
    }

    schema
}
