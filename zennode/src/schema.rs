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

    /// JSON key for whole-node serialization: `{"constrain": {...params...}}`.
    ///
    /// Empty string means use the `id`. Set via `#[node(json_key = "constrain")]`.
    pub json_key: &'static str,

    /// Whether to reject unknown fields during JSON deserialization.
    ///
    /// Set via `#[node(deny_unknown_fields)]`.
    pub deny_unknown_fields: bool,
}

impl NodeSchema {
    /// Backwards-compatible accessor for [`role`](Self::role).
    ///
    /// [`Phase`](crate::Phase) is a type alias for [`NodeRole`].
    pub fn phase(&self) -> NodeRole {
        self.role
    }

    /// Effective JSON key for whole-node serialization.
    ///
    /// Returns `json_key` if non-empty, otherwise `id`.
    pub fn effective_json_key(&self) -> &'static str {
        if self.json_key.is_empty() {
            self.id
        } else {
            self.json_key
        }
    }

    /// Render this schema as Markdown documentation.
    pub fn to_markdown(&self) -> alloc::string::String {
        use alloc::fmt::Write;
        let mut md = alloc::string::String::new();
        let _ = write!(md, "### `{}`\n\n", self.id);
        if !self.description.is_empty() {
            let _ = write!(md, "{}\n\n", self.description);
        }
        let _ = write!(
            md,
            "**Group:** {:?} | **Role:** {:?}",
            self.group, self.role
        );
        if !self.tags.is_empty() {
            let _ = write!(md, " | **Tags:** {}", self.tags.join(", "));
        }
        md.push_str("\n\n");
        if !self.params.is_empty() {
            md.push_str("| Parameter | Type | Default | Range | KV Keys | Description |\n");
            md.push_str("|-----------|------|---------|-------|---------|-------------|\n");
            for p in self.params {
                let (ty, default, range) = match &p.kind {
                    ParamKind::Float {
                        min, max, default, ..
                    } => (
                        "f32",
                        alloc::format!("{default}"),
                        alloc::format!("{min}..{max}"),
                    ),
                    ParamKind::Int { min, max, default } => (
                        "i32",
                        alloc::format!("{default}"),
                        alloc::format!("{min}..{max}"),
                    ),
                    ParamKind::U32 { min, max, default } => (
                        "u32",
                        alloc::format!("{default}"),
                        alloc::format!("{min}..{max}"),
                    ),
                    ParamKind::Bool { default } => (
                        "bool",
                        alloc::format!("{default}"),
                        alloc::string::String::new(),
                    ),
                    ParamKind::Str { default } => (
                        "string",
                        alloc::format!("\"{default}\""),
                        alloc::string::String::new(),
                    ),
                    ParamKind::Enum { default, variants } => {
                        let names: alloc::vec::Vec<&str> =
                            variants.iter().map(|v| v.name).collect();
                        ("enum", alloc::format!("\"{default}\""), names.join(" \\| "))
                    }
                    ParamKind::FloatArray {
                        len,
                        min,
                        max,
                        default,
                        ..
                    } => (
                        "f32[]",
                        alloc::format!("[{default}; {len}]"),
                        alloc::format!("{min}..{max}"),
                    ),
                    ParamKind::Color { default } => (
                        "color",
                        alloc::format!("{default:?}"),
                        alloc::string::String::new(),
                    ),
                    ParamKind::Json { .. } => (
                        "json",
                        alloc::string::String::from("(complex)"),
                        alloc::string::String::new(),
                    ),
                    ParamKind::Object { params } => (
                        "object",
                        alloc::format!("{} fields", params.len()),
                        alloc::string::String::new(),
                    ),
                    ParamKind::TaggedUnion { variants } => {
                        let tags: alloc::vec::Vec<&str> = variants.iter().map(|v| v.tag).collect();
                        ("union", tags.join(" \\| "), alloc::string::String::new())
                    }
                };
                let keys = if p.kv_keys.is_empty() {
                    alloc::string::String::from("—")
                } else {
                    p.kv_keys
                        .iter()
                        .map(|k| alloc::format!("`{k}`"))
                        .collect::<alloc::vec::Vec<_>>()
                        .join(", ")
                };
                let _ = writeln!(
                    md,
                    "| `{}` | {} | {} | {} | {} | {} |",
                    p.name,
                    ty,
                    default,
                    range,
                    keys,
                    p.description.replace('\n', " "),
                );
            }
            md.push('\n');
        }
        md
    }
}

/// A single parameter descriptor.
#[derive(Clone, Debug, PartialEq)]
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

    /// Whether this parameter can be [`ParamValue::None`](crate::ParamValue::None) (explicitly absent).
    ///
    /// Optional parameters map to `Option<T>` fields in the Rust struct.
    /// When `true`, UIs should distinguish "unset" from "set to default."
    pub optional: bool,

    /// JSON field name override. Empty string means use [`name`](Self::name).
    ///
    /// Set via `#[param(json_name = "sharpen_percent")]`.
    /// Use [`effective_json_name()`](Self::effective_json_name) to get the resolved name.
    pub json_name: &'static str,

    /// Additional JSON field names accepted during deserialization.
    ///
    /// Set via `#[param(json_alias = "old_name")]`.
    pub json_aliases: &'static [&'static str],
}

impl ParamDesc {
    /// Effective JSON field name: `json_name` if non-empty, else `name`.
    pub fn effective_json_name(&self) -> &'static str {
        if self.json_name.is_empty() {
            self.name
        } else {
            self.json_name
        }
    }

    /// Whether a given JSON key matches this parameter (by name or alias).
    pub fn matches_json_key(&self, key: &str) -> bool {
        let eff = self.effective_json_name();
        if key == eff || key == self.name {
            return true;
        }
        self.json_aliases.contains(&key)
    }
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
    /// Opaque JSON structure described by an inline JSON Schema fragment.
    ///
    /// For types that don't derive `Node` but need JSON param support.
    /// Prefer `Object` or `TaggedUnion` for types that derive `Node`.
    ///
    /// # Example
    ///
    /// ```ignore
    /// #[param(json_schema = r#"{"type":"object","properties":{"x":{"type":"number"}}}"#)]
    /// pub hints: Option<MyHintsStruct>,
    /// ```
    Json {
        /// JSON Schema 2020-12 fragment as a JSON string.
        json_schema: &'static str,
        /// Default value as a JSON string. Empty string means no default.
        default_json: &'static str,
    },
    /// Nested object with discoverable sub-parameters.
    ///
    /// The field type must implement [`NodeParams`]. Auto-detected by
    /// `#[derive(Node)]` when a field's type also derives `Node`.
    /// UIs render this as a collapsible sub-group with individual controls.
    Object {
        /// Sub-field descriptors with full UX metadata.
        params: &'static [ParamDesc],
    },
    /// Tagged union (discriminated enum) with variant-specific parameters.
    ///
    /// The field type must implement [`NodeParams`]. UIs render this as
    /// a variant selector (dropdown) plus variant-specific params.
    TaggedUnion {
        /// Variant descriptors. Unit variants have empty `params`.
        variants: &'static [TaggedVariant],
    },
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

/// Trait for types usable as structured node parameters.
///
/// Auto-implemented by `#[derive(Node)]`:
/// - **Structs with `#[node(id)]`**: full Node + `NodeParams` (sub-fields discoverable)
/// - **Structs without `#[node(id)]`**: `NodeParams` only (sub-struct for nesting)
/// - **Enums**: `NodeParams` with tagged variant descriptors
///
/// Used by `ParamKind::Object` and `ParamKind::TaggedUnion` to provide
/// full UX metadata (ranges, labels, sliders) for nested parameters.
///
/// # Example
///
/// ```rust,ignore
/// // Sub-struct (no node id) — only NodeParams is generated
/// #[derive(Node, Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
/// pub struct MyHints {
///     pub down_filter: Option<String>,
///     #[param(range(0.0..=100.0))]
///     pub sharpen_percent: Option<f32>,
/// }
/// ```
pub trait NodeParams {
    /// The `ParamKind` for this type when used as a field in another node.
    ///
    /// For structs: `ParamKind::Object { params }`.
    /// For enums: `ParamKind::TaggedUnion { variants }`.
    const PARAM_KIND: ParamKind;
}

// Keep JsonParam as an alias for backwards compat with the Json param codegen
pub use NodeParams as JsonParam;

/// A variant in a tagged union parameter.
///
/// Describes one arm of an enum used as a node parameter.
/// Unit variants have an empty `params` slice.
#[derive(Clone, Debug, PartialEq)]
pub struct TaggedVariant {
    /// Machine name (snake_case, matching serde serialization).
    pub tag: &'static str,
    /// Human-readable label.
    pub label: &'static str,
    /// Short description from doc comments.
    pub description: &'static str,
    /// Parameters for this variant (empty for unit variants).
    pub params: &'static [ParamDesc],
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
