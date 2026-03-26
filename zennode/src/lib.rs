//! Self-documenting node definitions for image processing pipelines.
//!
//! `zennode` provides a trait-based system for defining pipeline operations
//! with full parameter schemas, RIAPI querystring parsing, and JSON Schema
//! generation — all with permanent backwards compatibility.
//!
//! # Quick Start
//!
//! ```rust,ignore
//! use zennode::*;
//!
//! #[derive(Node, Clone, Debug, Default)]
//! #[node(id = "myfilter.brightness", group = Tone, role = Filter)]
//! pub struct Brightness {
//!     /// Amount of brightness adjustment
//!     #[param(range(-1.0..=1.0), default = 0.0, identity = 0.0, step = 0.05)]
//!     #[param(unit = "", section = "Main")]
//!     pub amount: f32,
//! }
//! ```
//!
//! # Features
//!
//! - `derive` (default) — enables `#[derive(Node)]` and `#[derive(NodeEnum)]`
//! - `std` (default) — enables `std::error::Error` impl
//! - `serde` — enables serde `Serialize`/`Deserialize` on param types
//! - `json-schema` — enables JSON Schema generation (implies `serde`)

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

// Allow the derive macro (which emits `::zennode::`) to work inside this crate.
extern crate self as zennode;

pub mod error;
pub mod format;
pub mod kv;
pub mod ordering;
pub mod param;
pub mod registry;
pub mod schema;
pub mod traits;
pub mod version;

/// zennode does not define any built-in nodes. See [`nodes`] module docs.
#[cfg(feature = "derive")]
pub mod nodes;

#[cfg(feature = "serde")]
pub mod serde_impl;

#[cfg(feature = "json-schema")]
pub mod json_schema;

// Re-exports for convenience
pub use error::NodeError;
pub use format::{AlphaHandling, FormatHint, PixelFormatPreference};
pub use kv::{KvPairs, KvWarning, KvWarningKind};
pub use ordering::{CoalesceInfo, NodeRole};
/// Backwards compatibility alias.
pub type Phase = NodeRole;
pub use param::{ParamMap, ParamValue};
pub use registry::{KvResult, NodeRegistry};
pub use schema::{EnumVariant, NodeGroup, NodeSchema, ParamDesc, ParamKind, SliderMapping};
pub use traits::{NodeDef, NodeInstance};
pub use version::VersionSet;

// Re-export derive macros
#[cfg(feature = "derive")]
pub use zennode_derive::{Node, NodeEnum};

/// Private re-exports used by the derive macro. Not public API.
#[doc(hidden)]
pub mod __private {
    pub use alloc::boxed::Box;
    pub use alloc::string::{String, ToString};
    pub use alloc::vec::Vec;

    #[cfg(feature = "serde")]
    pub use serde_json;
}
