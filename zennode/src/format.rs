//! Pixel format preference types.
//!
//! These are coarse classifications — not tied to `zenpixels::PixelDescriptor`.
//! Aggregating crates map these to concrete pixel format types.

/// Pixel format preferences for a node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FormatHint {
    /// Preferred input pixel format.
    pub preferred: PixelFormatPreference,
    /// How this node handles the alpha channel.
    pub alpha: AlphaHandling,
    /// Whether this node changes image dimensions.
    pub changes_dimensions: bool,
    /// Whether this node needs access to neighboring pixels.
    pub is_neighborhood: bool,
}

impl Default for FormatHint {
    fn default() -> Self {
        Self {
            preferred: PixelFormatPreference::Any,
            alpha: AlphaHandling::Process,
            changes_dimensions: false,
            is_neighborhood: false,
        }
    }
}

/// Preferred pixel format for processing.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PixelFormatPreference {
    /// No preference — works with any format.
    #[default]
    Any,
    /// Planar Oklab f32 (zenfilters: perceptual operations).
    OklabF32,
    /// Interleaved RGBA f32 linear light (resize, composite).
    LinearF32,
    /// Premultiplied linear f32 (blend, composite).
    PremulLinearF32,
    /// Interleaved RGBA u8 sRGB (pass-through, canvas ops).
    Srgb8,
    /// Scene-referred linear f32 (RAW, HDR).
    SceneLinearF32,
}

/// How a node handles the alpha channel.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AlphaHandling {
    /// Alpha is processed along with color channels.
    #[default]
    Process,
    /// Alpha channel is left untouched (most filters).
    Skip,
    /// Requires premultiplied alpha input (blend operations).
    RequirePremul,
    /// Explicitly modifies the alpha channel.
    ModifyAlpha,
}
