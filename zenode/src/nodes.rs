//! Built-in shared node definitions.
//!
//! These nodes don't belong to any specific codec or processing crate.
//! They represent format-independent concepts used across the pipeline.
//!
//! Note: `QualityIntent` has moved to `zencodecs::zenode_defs::QualityIntentNode`.

extern crate alloc;
use alloc::string::String;

use crate::*;

// ─── Decode ───

/// Decode an image from an I/O source.
///
/// The I/O binding (bytes, file, buffer) is handled by the pipeline
/// runtime, not by this node. The `io_id` identifies which I/O slot
/// to read from.
///
/// Format detection is automatic (from magic bytes). The decode params
/// control behavior that applies across all formats.
#[derive(Node, Clone, Debug)]
#[node(id = "zenode.decode", group = Decode, role = Decode)]
#[node(tags("io", "decode"))]
pub struct Decode {
    /// I/O slot identifier (assigned by the job builder).
    #[param(range(0..=255), default = 0, step = 1)]
    #[param(section = "Main")]
    pub io_id: i32,

    /// HDR gain map handling mode.
    ///
    /// "sdr_only" — ignore any gain map, decode SDR base image (default).
    /// "hdr_reconstruct" — apply gain map to reconstruct full HDR.
    /// "preserve" — keep SDR base + gain map as separate pipeline streams.
    #[param(default = "sdr_only")]
    #[param(section = "HDR", label = "HDR Mode")]
    pub hdr_mode: String,

    /// Color management intent.
    ///
    /// "preserve" — pass ICC/CICP metadata through, don't convert pixels (default).
    /// "srgb" — convert to sRGB at decode time.
    #[param(default = "preserve")]
    #[param(section = "Color", label = "Color Intent")]
    pub color_intent: String,

    /// Minimum output dimension hint for decoder prescaling.
    ///
    /// JPEG decoders can prescale to 1/2, 1/4, or 1/8 during decode for speed.
    /// Set to the smallest dimension you need. 0 = no prescaling (default).
    #[param(range(0..=65535), default = 0, step = 1)]
    #[param(unit = "px", section = "Performance", label = "Min Size Hint")]
    pub min_size: u32,
}

impl Default for Decode {
    fn default() -> Self {
        Self {
            io_id: 0,
            hdr_mode: String::from("sdr_only"),
            color_intent: String::from("preserve"),
            min_size: 0,
        }
    }
}
