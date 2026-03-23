//! Built-in shared node definitions.
//!
//! These nodes don't belong to any specific codec or processing crate.
//! They represent format-independent concepts used across the pipeline.

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
#[node(id = "zenode.decode", group = Decode, phase = Decode)]
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

// ─── QualityIntent ───

/// Format selection and quality profile for encoding.
///
/// This node controls output format selection and quality. It supports
/// both RIAPI querystring keys and JSON API fields, matching imageflow's
/// established `EncoderPreset::Auto` / `EncoderPreset::Format` ergonomics.
///
/// **RIAPI**: `?qp=high&accept.webp=true&accept.avif=true`
/// **JSON**: `{ "profile": "high", "allow_webp": true, "allow_avif": true }`
///
/// When `format` is empty (default), the pipeline auto-selects the best
/// format from the allowed set. When `format` is set (e.g., "jpeg"),
/// that format is used directly.
///
/// The `profile` field accepts both named presets and numeric values:
/// - Named: lowest, low, medium_low, medium, good, high, highest, lossless
/// - Numeric: 0-100 (mapped to codec-specific quality scales)
///
/// Matches imageflow's `EncoderPreset::Auto` for backwards compatibility.
#[derive(Node, Clone, Debug)]
#[node(id = "zenode.quality_intent", group = Encode, phase = Encode)]
#[node(tags("quality", "auto", "format", "encode"))]
pub struct QualityIntent {
    /// Quality profile: named preset or numeric 0-100.
    ///
    /// Named presets: "lowest", "low", "medium_low", "medium",
    /// "good", "high", "highest", "lossless".
    /// Numeric: "0" to "100" (codec-specific mapping).
    /// Also accepts aliases: "med" = "medium", "medium-low" = "medium_low",
    /// "medium-high" = "good".
    #[param(default = "high")]
    #[param(section = "Main", label = "Quality Profile")]
    #[kv("qp")]
    pub profile: String,

    /// Explicit output format. Empty = auto-select from allowed formats.
    ///
    /// Values: "jpeg", "png", "webp", "gif", "avif", "jxl", "keep", or "".
    /// "keep" preserves the source format.
    /// When set, the format is used directly (quality profile still applies).
    #[param(default = "")]
    #[param(section = "Main", label = "Output Format")]
    #[kv("format")]
    pub format: String,

    /// Device pixel ratio for quality adjustment.
    ///
    /// Higher DPR screens tolerate lower quality (smaller pixels).
    /// Default 1.0 = no adjustment. Imageflow default was 3.0.
    #[param(range(0.5..=10.0), default = 1.0, identity = 1.0, step = 0.5)]
    #[param(unit = "×", section = "Main", label = "Device Pixel Ratio")]
    #[kv("qp.dpr", "qp.dppx", "dpr", "dppx")]
    pub dpr: f32,

    /// Global lossless preference (true/false/keep).
    ///
    /// When true, selects lossless encoding if the format supports it.
    #[param(default = false)]
    #[param(section = "Main")]
    #[kv("lossless")]
    pub lossless: bool,

    /// Allow JPEG output. Default true (web_safe baseline).
    #[param(default = true)]
    #[param(section = "Allowed Formats")]
    pub allow_jpeg: bool,

    /// Allow PNG output. Default true (web_safe baseline).
    #[param(default = true)]
    #[param(section = "Allowed Formats")]
    pub allow_png: bool,

    /// Allow GIF output. Default true (web_safe baseline).
    #[param(default = true)]
    #[param(section = "Allowed Formats")]
    pub allow_gif: bool,

    /// Allow WebP output. Must be explicitly enabled.
    #[param(default = false)]
    #[param(section = "Allowed Formats")]
    #[kv("accept.webp")]
    pub allow_webp: bool,

    /// Allow AVIF output. Must be explicitly enabled.
    #[param(default = false)]
    #[param(section = "Allowed Formats")]
    #[kv("accept.avif")]
    pub allow_avif: bool,

    /// Allow JPEG XL output. Must be explicitly enabled.
    #[param(default = false)]
    #[param(section = "Allowed Formats")]
    #[kv("accept.jxl")]
    pub allow_jxl: bool,

    /// Allow non-sRGB color profiles in the output.
    #[param(default = false)]
    #[param(section = "Allowed Formats")]
    #[kv("accept.color_profiles")]
    pub allow_color_profiles: bool,
}

impl Default for QualityIntent {
    fn default() -> Self {
        Self {
            profile: String::from("high"),
            format: String::new(),
            dpr: 1.0,
            lossless: false,
            allow_jpeg: true,
            allow_png: true,
            allow_gif: true,
            allow_webp: false,
            allow_avif: false,
            allow_jxl: false,
            allow_color_profiles: false,
        }
    }
}
