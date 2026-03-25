//! zennode does not define any built-in nodes.
//!
//! Node definitions belong in the crates that implement them:
//! - Codec encode nodes: `zenjpeg`, `zenpng`, `zenwebp`, etc. (feature `zennode`)
//! - Quality intent: `zencodecs` (feature `zennode`)
//! - Filter nodes: `zenfilters` (feature `zennode`)
//! - Geometry nodes: `zenlayout`, `zenresize` (feature `zennode`)
//! - Blend/composite: `zenblend` (feature `zennode`)
//! - Quantization: `zenquant` (feature `zennode`)
//!
//! Decode is not a node — it's configured by the executor (zenpipe) based on
//! probed format and job-level settings.
//!
//! zennode provides only the infrastructure: traits, derive macros, KV parsing,
//! registry, and schema types. No nodes.
