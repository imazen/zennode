//! Runtime parameter value types.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// A concrete parameter value for get/set operations at runtime.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub enum ParamValue {
    /// Explicitly absent — the parameter is unset.
    ///
    /// Used for `Option<T>` fields: `None` in the Rust struct maps to
    /// `ParamValue::None`, distinct from "set to default."
    None,
    /// 32-bit float.
    F32(f32),
    /// Signed 32-bit integer.
    I32(i32),
    /// Unsigned 32-bit integer.
    U32(u32),
    /// Boolean.
    Bool(bool),
    /// Free-form string.
    Str(String),
    /// Enum variant name (snake_case).
    Enum(String),
    /// Fixed-size float array.
    F32Array(Vec<f32>),
    /// RGBA color.
    Color([f32; 4]),
    /// Opaque JSON value, stored as a JSON text string.
    ///
    /// Carries nested objects, tagged unions, or any complex structure
    /// whose wire format is a JSON object/array. The schema describes
    /// the structure via an embedded JSON Schema fragment.
    ///
    /// Use `as_json_str()` to get the raw JSON text, or deserialize
    /// with `serde_json::from_str` to a concrete type.
    Json(String),
}

impl ParamValue {
    /// Returns `true` if this is `ParamValue::None` (explicitly absent).
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    /// Returns `true` if this is not `ParamValue::None`.
    pub fn is_some(&self) -> bool {
        !self.is_none()
    }

    /// Try to extract as `f32`. Converts from `I32` and `U32` if possible.
    pub fn as_f32(&self) -> Option<f32> {
        match self {
            Self::F32(v) => Some(*v),
            Self::I32(v) => Some(*v as f32),
            Self::U32(v) => Some(*v as f32),
            _ => None,
        }
    }

    /// Try to extract as `i32`.
    pub fn as_i32(&self) -> Option<i32> {
        match self {
            Self::I32(v) => Some(*v),
            Self::F32(v) => Some(*v as i32),
            Self::U32(v) => i32::try_from(*v).ok(),
            _ => None,
        }
    }

    /// Try to extract as `u32`.
    pub fn as_u32(&self) -> Option<u32> {
        match self {
            Self::U32(v) => Some(*v),
            Self::I32(v) => u32::try_from(*v).ok(),
            Self::F32(v) => Some(*v as u32),
            _ => None,
        }
    }

    /// Try to extract as `bool`.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(v) => Some(*v),
            _ => None,
        }
    }

    /// Try to extract as a string reference.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::Str(v) | Self::Enum(v) => Some(v),
            _ => None,
        }
    }

    /// Try to extract as a float array reference.
    pub fn as_f32_array(&self) -> Option<&[f32]> {
        match self {
            Self::F32Array(v) => Some(v),
            _ => None,
        }
    }

    /// Try to extract as an RGBA color.
    pub fn as_color(&self) -> Option<[f32; 4]> {
        match self {
            Self::Color(v) => Some(*v),
            _ => None,
        }
    }

    /// Try to extract as a JSON text string reference.
    pub fn as_json_str(&self) -> Option<&str> {
        match self {
            Self::Json(v) => Some(v),
            _ => None,
        }
    }
}

/// Ordered map of parameter name to value.
///
/// Uses `BTreeMap` for deterministic iteration order.
pub type ParamMap = BTreeMap<String, ParamValue>;
