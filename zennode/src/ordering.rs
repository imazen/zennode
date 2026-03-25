//! Node classification and coalescing types.
//!
//! # Ordering Model
//!
//! zennode does NOT reorder user-specified node sequences. Nodes execute in
//! the order the user declared them (RIAPI querystring position, JSON array
//! index, or DAG edges).
//!
//! [`NodeRole`] is a **type tag** that tells the pipeline bridge what kind
//! of planner a node feeds into — geometry planner, filter pipeline, or
//! encode config. The bridge walks the user's node list left-to-right,
//! accumulating runs of compatible nodes and flushing when the role changes.
//!
//! Adjacent nodes with the same [`CoalesceInfo::group`] and compatible roles
//! are fused into a single operation. This fusion is always equivalent to
//! naive sequential execution — never reordering.
//!
//! # Example
//!
//! ```text
//! User order: [orient, crop, resize, exposure, contrast, sharpen, encode]
//!
//! Bridge walks left-to-right:
//!   orient   → geometry run (start)
//!   crop     → geometry run (extend)
//!   resize   → geometry run (extend)
//!   exposure → FLUSH geometry → LayoutPlan. Filter run (start).
//!   contrast → filter run (extend — same coalesce group)
//!   sharpen  → filter run (extend — neighborhood, handled by Pipeline)
//!   encode   → FLUSH filter → FilterPipeline. Collect encode config.
//!
//! Result: Source → LayoutPlan → FilterPipeline → Encode
//! ```

/// What role a node plays in the pipeline.
///
/// Used by the bridge to collect nodes into compatible runs and feed
/// each run to the appropriate planner. NOT a sort key — user ordering
/// is always preserved.
///
/// When the bridge encounters a node whose role differs from the current
/// run, it flushes the accumulated run to its planner and starts a new run.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NodeRole {
    /// Source decoding and I/O configuration.
    Decode,

    /// Geometric operations: crop, resize, orient, pad, region.
    ///
    /// Adjacent geometry nodes feed into `zenlayout::Pipeline` which
    /// computes a single `LayoutPlan` executed by `zenresize::StreamingResize`.
    /// Orientation composes via D4 algebra. Last crop/constraint wins.
    Geometry,

    /// Orientation sub-role of geometry (EXIF orient, flip, rotate).
    ///
    /// Treated identically to [`Geometry`](Self::Geometry) by the bridge.
    /// Exists so downstream crates can tag orientation-only nodes distinctly
    /// from resize/crop nodes.
    Orient,

    /// Resize sub-role of geometry (constrain, scale, expand canvas).
    ///
    /// Treated identically to [`Geometry`](Self::Geometry) by the bridge.
    /// Exists so downstream crates can tag resize/layout nodes distinctly
    /// from orientation-only nodes.
    Resize,

    /// Per-pixel filter operations: exposure, contrast, color, sharpen, etc.
    ///
    /// Adjacent filter nodes feed into `zenfilters::Pipeline`.
    /// FusedAdjust-compatible filters merge into a single SIMD pass.
    /// Neighborhood filters (clarity, sharpen) use windowed execution.
    Filter,

    /// Compositing operations: blend, watermark, overlay.
    Composite,

    /// Analysis operations: face detection, saliency, quality metrics.
    ///
    /// May require full-frame materialization.
    Analysis,

    /// Quantization: palette reduction.
    Quantize,

    /// Encoding configuration: format selection, quality, codec params.
    Encode,
}

impl NodeRole {
    /// Whether this role represents a geometry operation (including orient/resize sub-roles).
    pub fn is_geometry(self) -> bool {
        matches!(self, Self::Geometry | Self::Orient | Self::Resize)
    }
}

/// How a node can be fused/coalesced with adjacent operations.
///
/// Coalescing only happens between **adjacent** nodes in the user's
/// declared order that share the same `group` name. The fused result
/// is always mathematically equivalent to sequential execution.
///
/// The bridge never moves a node past a node of a different role
/// or a different coalesce group.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoalesceInfo {
    /// Named group (e.g., `"layout_plan"`, `"fused_adjust"`).
    ///
    /// Adjacent nodes in the same group with `fusable = true` can be
    /// merged into a single operation.
    pub group: &'static str,

    /// This node contributes parameters to the coalesced operation.
    pub fusable: bool,

    /// This node IS the coalesced target (e.g., FusedAdjust itself).
    pub is_target: bool,
}
