//! Derive macros for zennode node definitions.
//!
//! Provides `#[derive(Node)]` for structs and `#[derive(NodeEnum)]` for enums.

mod attrs;
mod codegen;
mod node;
mod node_enum;

use proc_macro::TokenStream;

/// Derive macro for defining a pipeline node from a struct.
///
/// # Struct-level attributes
///
/// **Required:**
/// - `#[node(id = "crate.name")]` — permanent fully-qualified identifier
/// - `#[node(group = Tone)]` — [`NodeGroup`] variant
/// - `#[node(phase = DisplayAdjust)]` — [`Phase`] variant
///
/// **Optional:**
/// - `#[node(label = "...")]` — human label (default: struct name with spaces)
/// - `#[node(version = 2)]` — schema version (default: 1)
/// - `#[node(compat_version = 1)]` — min compat version (default: 1)
/// - `#[node(coalesce = "group")]` — coalesce group (implies fusable)
/// - `#[node(fusable)]` — can be fused in its coalesce group
/// - `#[node(coalesce_target)]` — this IS the fused op
/// - `#[node(neighborhood)]` — neighborhood operation
/// - `#[node(changes_dimensions)]` — changes image dimensions
/// - `#[node(format(preferred = OklabF32, alpha = Skip))]` — format hints
/// - `#[node(tags("basic", "tone"))]` — discovery tags
///
/// # Field-level attributes
///
/// **`#[param(...)]`:**
/// - `range(-5.0..=5.0)` — min/max
/// - `default = 0.0` — default value
/// - `identity = 0.0` — identity/no-op value
/// - `step = 0.1` — UI step increment
/// - `unit = "stops"` — display unit
/// - `section = "Main"` — UI sub-section
/// - `slider = SquareFromSlider` — slider mapping
/// - `label = "..."` — human label (default: field name titlecased)
/// - `since = 2` — schema version that added this param
/// - `labels("R", "O", "Y")` — per-element labels for arrays
/// - `visible_when = "mode=advanced"` — conditional visibility
///
/// **`#[kv("key1", "key2")]`:** RIAPI querystring keys
#[proc_macro_derive(Node, attributes(node, param, kv))]
pub fn derive_node(input: TokenStream) -> TokenStream {
    node::derive_node_impl(input)
}

/// Derive macro for enum types used as node parameters.
///
/// Generates variant metadata, `Display`, `FromStr`, and optional serde support.
///
/// # Variant attributes
///
/// - `#[variant(label = "...")]` — override derived label
/// - `#[variant(alias = "old_name")]` — additional parse alias
#[proc_macro_derive(NodeEnum, attributes(variant))]
pub fn derive_node_enum(input: TokenStream) -> TokenStream {
    node_enum::derive_node_enum_impl(input)
}
