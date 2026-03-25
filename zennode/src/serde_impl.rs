//! Serde support for zennode types.
//!
//! Wildcard arms on `#[non_exhaustive]` enums are intentional for forward compatibility.
#![allow(unreachable_patterns)]

use serde::{Deserialize, Serialize};

use crate::format::{AlphaHandling, FormatHint, PixelFormatPreference};
use crate::ordering::{CoalesceInfo, NodeRole};
use crate::param::ParamValue;
use crate::schema::{EnumVariant, NodeGroup, NodeSchema, ParamDesc, ParamKind, SliderMapping};

// --- ParamValue: untagged serialization ---

impl Serialize for ParamValue {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::F32(v) => serializer.serialize_f32(*v),
            Self::I32(v) => serializer.serialize_i32(*v),
            Self::U32(v) => serializer.serialize_u32(*v),
            Self::Bool(v) => serializer.serialize_bool(*v),
            Self::Str(v) | Self::Enum(v) => serializer.serialize_str(v),
            Self::F32Array(v) => v.serialize(serializer),
            Self::Color(v) => v.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for ParamValue {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(deserializer)?;
        Ok(json_to_param_value(&value))
    }
}

fn json_to_param_value(value: &serde_json::Value) -> ParamValue {
    match value {
        serde_json::Value::Bool(b) => ParamValue::Bool(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                if let Ok(v) = i32::try_from(i) {
                    return ParamValue::I32(v);
                }
                if let Ok(v) = u32::try_from(i) {
                    return ParamValue::U32(v);
                }
            }
            ParamValue::F32(n.as_f64().unwrap_or(0.0) as f32)
        }
        serde_json::Value::String(s) => ParamValue::Str(s.clone()),
        serde_json::Value::Array(arr) => {
            let floats: alloc::vec::Vec<f32> = arr
                .iter()
                .filter_map(|v| v.as_f64().map(|f| f as f32))
                .collect();
            if floats.len() == 4 {
                ParamValue::Color([floats[0], floats[1], floats[2], floats[3]])
            } else {
                ParamValue::F32Array(floats)
            }
        }
        _ => ParamValue::Str(alloc::string::String::new()),
    }
}

// --- Schema types: Serialize only (for export) ---

impl Serialize for NodeGroup {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(match self {
            Self::Decode => "decode",
            Self::Encode => "encode",
            Self::Tone => "tone",
            Self::ToneRange => "tone_range",
            Self::ToneMap => "tone_map",
            Self::Color => "color",
            Self::Detail => "detail",
            Self::Effects => "effects",
            Self::Geometry => "geometry",
            Self::Layout => "layout",
            Self::Canvas => "canvas",
            Self::Composite => "composite",
            Self::Quantize => "quantize",
            Self::Analysis => "analysis",
            Self::Hdr => "hdr",
            Self::Raw => "raw",
            Self::Auto => "auto",
            Self::Other => "other",
            _ => "other",
        })
    }
}

impl Serialize for NodeRole {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(match self {
            Self::Decode => "decode",
            Self::Geometry => "geometry",
            Self::Filter => "filter",
            Self::Composite => "composite",
            Self::Analysis => "analysis",
            Self::Quantize => "quantize",
            Self::Encode => "encode",
            _ => "other",
        })
    }
}

impl Serialize for SliderMapping {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(match self {
            Self::Linear => "linear",
            Self::SquareFromSlider => "square_from_slider",
            Self::FactorCentered => "factor_centered",
            Self::Logarithmic => "logarithmic",
            Self::NotSlider => "not_slider",
            _ => "linear",
        })
    }
}

impl Serialize for PixelFormatPreference {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(match self {
            Self::Any => "any",
            Self::OklabF32 => "oklab_f32",
            Self::LinearF32 => "linear_f32",
            Self::PremulLinearF32 => "premul_linear_f32",
            Self::Srgb8 => "srgb8",
            Self::SceneLinearF32 => "scene_linear_f32",
            _ => "any",
        })
    }
}

impl Serialize for AlphaHandling {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(match self {
            Self::Process => "process",
            Self::Skip => "skip",
            Self::RequirePremul => "require_premul",
            Self::ModifyAlpha => "modify_alpha",
            _ => "process",
        })
    }
}

impl Serialize for FormatHint {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("FormatHint", 4)?;
        s.serialize_field("preferred", &self.preferred)?;
        s.serialize_field("alpha", &self.alpha)?;
        s.serialize_field("changes_dimensions", &self.changes_dimensions)?;
        s.serialize_field("is_neighborhood", &self.is_neighborhood)?;
        s.end()
    }
}

impl Serialize for CoalesceInfo {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("CoalesceInfo", 3)?;
        s.serialize_field("group", &self.group)?;
        s.serialize_field("fusable", &self.fusable)?;
        s.serialize_field("is_target", &self.is_target)?;
        s.end()
    }
}

impl Serialize for ParamKind {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(None)?;
        match self {
            Self::Float {
                min,
                max,
                default,
                identity,
                step,
            } => {
                map.serialize_entry("type", "float")?;
                map.serialize_entry("min", min)?;
                map.serialize_entry("max", max)?;
                map.serialize_entry("default", default)?;
                map.serialize_entry("identity", identity)?;
                map.serialize_entry("step", step)?;
            }
            Self::Int { min, max, default } => {
                map.serialize_entry("type", "int")?;
                map.serialize_entry("min", min)?;
                map.serialize_entry("max", max)?;
                map.serialize_entry("default", default)?;
            }
            Self::U32 { min, max, default } => {
                map.serialize_entry("type", "u32")?;
                map.serialize_entry("min", min)?;
                map.serialize_entry("max", max)?;
                map.serialize_entry("default", default)?;
            }
            Self::Bool { default } => {
                map.serialize_entry("type", "bool")?;
                map.serialize_entry("default", default)?;
            }
            Self::Str { default } => {
                map.serialize_entry("type", "string")?;
                map.serialize_entry("default", default)?;
            }
            Self::Enum { variants, default } => {
                map.serialize_entry("type", "enum")?;
                map.serialize_entry("default", default)?;
                let names: alloc::vec::Vec<&str> = variants.iter().map(|v| v.name).collect();
                map.serialize_entry("variants", &names)?;
            }
            Self::FloatArray {
                len,
                min,
                max,
                default,
                labels,
            } => {
                map.serialize_entry("type", "float_array")?;
                map.serialize_entry("len", len)?;
                map.serialize_entry("min", min)?;
                map.serialize_entry("max", max)?;
                map.serialize_entry("default", default)?;
                map.serialize_entry("labels", labels)?;
            }
            Self::Color { default } => {
                map.serialize_entry("type", "color")?;
                map.serialize_entry("default", default)?;
            }
            _ => {
                map.serialize_entry("type", "unknown")?;
            }
        }
        map.end()
    }
}

impl Serialize for EnumVariant {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("EnumVariant", 3)?;
        s.serialize_field("name", &self.name)?;
        s.serialize_field("label", &self.label)?;
        s.serialize_field("description", &self.description)?;
        s.end()
    }
}

impl Serialize for ParamDesc {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("ParamDesc", 10)?;
        s.serialize_field("name", &self.name)?;
        s.serialize_field("label", &self.label)?;
        s.serialize_field("description", &self.description)?;
        s.serialize_field("kind", &self.kind)?;
        s.serialize_field("unit", &self.unit)?;
        s.serialize_field("section", &self.section)?;
        s.serialize_field("slider", &self.slider)?;
        s.serialize_field("kv_keys", &self.kv_keys)?;
        s.serialize_field("since_version", &self.since_version)?;
        s.serialize_field("visible_when", &self.visible_when)?;
        s.end()
    }
}

impl Serialize for NodeSchema {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("NodeSchema", 12)?;
        s.serialize_field("id", &self.id)?;
        s.serialize_field("label", &self.label)?;
        s.serialize_field("description", &self.description)?;
        s.serialize_field("group", &self.group)?;
        s.serialize_field("role", &self.role)?;
        s.serialize_field("params", &self.params)?;
        s.serialize_field("tags", &self.tags)?;
        s.serialize_field("coalesce", &self.coalesce)?;
        s.serialize_field("format", &self.format)?;
        s.serialize_field("version", &self.version)?;
        s.serialize_field("compat_version", &self.compat_version)?;
        s.end()
    }
}
