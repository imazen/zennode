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
