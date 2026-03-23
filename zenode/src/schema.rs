//! Static schema types for node definitions.
//!
//! All types here use `&'static` references — schemas are constructed as
//! statics/consts with zero heap allocation.

use crate::format::FormatHint;
use crate::ordering::{CoalesceInfo, NodeRole};

/// Complete static schema for one node type.
pub struct NodeSchema {
    /// Permanent fully-qualified identifier: `"crate.operation"`.
    ///
    /// Once published, this MUST NEVER change.
    pub id: &'static str,

    /// Human-readable label for UI (e.g., `"Exposure"`).
    pub label: &'static str,

    /// One-line description for tooltips.
    pub description: &'static str,

    /// UI grouping category.
    pub group: NodeGroup,

    /// What role this node plays in the pipeline.
    ///
    /// Used by the bridge to collect compatible node runs and feed them
    /// to the appropriate planner. NOT a sort key — user ordering is
    /// always preserved.
    pub role: NodeRole,

    /// Parameter descriptors in display order.
    pub params: &'static [ParamDesc],

    /// Discovery/filter tags (e.g., `["basic", "tone"]`).
    pub tags: &'static [&'static str],

    /// Coalescing/fusion info. `None` means not fusable.
    pub coalesce: Option<CoalesceInfo>,

    /// Pixel format preferences.
    pub format: FormatHint,

    /// Schema version (monotonically increasing per node ID).
    pub version: u32,

    /// Oldest version this schema can deserialize from.
    pub compat_version: u32,
}

impl NodeSchema {
    /// Backwards-compatible accessor for [`role`](Self::role).
    ///
    /// [`Phase`](crate::Phase) is a type alias for [`NodeRole`].
    pub fn phase(&self) -> NodeRole {
        self.role
    }
}

/// A single parameter descriptor.
pub struct ParamDesc {
    /// Machine name — matches the Rust struct field name.
    pub name: &'static str,

    /// Human-readable label for UI.
    pub label: &'static str,

    /// Description for tooltips. Sourced from doc comments by the derive macro.
    pub description: &'static str,

    /// Type, range, and default.
    pub kind: ParamKind,

    /// Display unit (`"stops"`, `"°"`, `"×"`, `"%"`, `"px"`, `""`).
    pub unit: &'static str,

    /// UI sub-section (`"Main"`, `"Advanced"`, `"Masking"`).
    pub section: &'static str,

    /// How to map a UI slider position to this parameter's value.
    pub slider: SliderMapping,

    /// RIAPI querystring keys that map to this parameter.
    pub kv_keys: &'static [&'static str],

    /// Schema version that introduced this parameter.
    pub since_version: u32,

    /// Conditional visibility expression: `"param_name=value"` or `""` for always visible.
    pub visible_when: &'static str,
}

/// Parameter type, range, and default value.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub enum ParamKind {
    /// Continuous float parameter.
    Float {
        min: f32,
        max: f32,
        default: f32,
        /// Value at which the parameter has no effect.
        identity: f32,
        /// Suggested UI step increment.
        step: f32,
    },
    /// Signed integer parameter.
    Int { min: i32, max: i32, default: i32 },
    /// Unsigned integer parameter.
    U32 { min: u32, max: u32, default: u32 },
    /// Boolean toggle.
    Bool { default: bool },
    /// Free-form string.
    Str { default: &'static str },
    /// Enumeration of named variants.
    Enum {
        variants: &'static [EnumVariant],
        default: &'static str,
    },
    /// Fixed-size float array (e.g., HSL per-hue weights).
    FloatArray {
        len: usize,
        min: f32,
        max: f32,
        default: f32,
        /// Per-element labels (e.g., `["Red", "Orange", ...]`).
        labels: &'static [&'static str],
    },
    /// RGBA color.
    Color { default: [f32; 4] },
}

/// An enum variant descriptor.
#[derive(Clone, Debug, PartialEq)]
pub struct EnumVariant {
    /// Machine name (snake_case, used for serialization).
    pub name: &'static str,
    /// Human-readable label.
    pub label: &'static str,
    /// Short description.
    pub description: &'static str,
}

/// How a parameter maps to a UI slider.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SliderMapping {
    /// Direct 1:1 mapping.
    Linear,
    /// `param = slider²` — first half of slider range is more sensitive.
    SquareFromSlider,
    /// Slider 0–1 maps to param 0–2, with center (0.5) = identity (1.0).
    FactorCentered,
    /// Logarithmic mapping for large ranges.
    Logarithmic,
    /// Not suitable for a single slider (arrays, curves, enums).
    NotSlider,
}

/// Node group for UI categorization.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NodeGroup {
    Decode,
    Encode,
    Tone,
    ToneRange,
    ToneMap,
    Color,
    Detail,
    Effects,
    Geometry,
    Layout,
    Canvas,
    Composite,
    Quantize,
    Analysis,
    Hdr,
    Raw,
    Auto,
    Other,
}
