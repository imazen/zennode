# Full Encode Responsibilities

## Everything that happens at encode time

```
Processed pixels (streaming Source)
        │
        ▼
┌─ ENCODE ORCHESTRATION ──────────────────────────────────┐
│                                                          │
│  1. FORMAT RESOLUTION                                    │
│     QualityIntent → FormatDecision (zencodecs)           │
│     OR explicit codec node → direct config               │
│                                                          │
│  2. PIXEL FORMAT NEGOTIATION                             │
│     Encoder declares supported PixelDescriptors          │
│     Pipeline output may need conversion                  │
│     (e.g., OklabF32 → RGBA8_SRGB for JPEG)              │
│     This is a streaming format conversion, NOT matte     │
│                                                          │
│  3. ALPHA → MATTE (if target format has no alpha)        │
│     RGBA source → JPEG output: composite over matte      │
│     Matte color from QualityIntent/FormatDecision         │
│     Streaming: per-strip alpha composite                  │
│                                                          │
│  4. COLOR SPACE                                          │
│     If source has ICC profile and target supports it:     │
│       → embed ICC in output                              │
│     If source is wide gamut and target is sRGB:           │
│       → CMS conversion (moxcms) as streaming transform   │
│     CICP signaling for HDR formats (AVIF, JXL)           │
│                                                          │
│  5. ORIENTATION                                          │
│     If remaining_orientation != Identity:                 │
│       If target format supports EXIF orientation tag:     │
│         → set tag, skip pixel rotation (free)             │
│       Else:                                              │
│         → pixel rotation was already done in pipeline     │
│                                                          │
│  6. METADATA EMBEDDING                                   │
│     EXIF: rotation tag, camera info, GPS, timestamps     │
│     XMP: editing history, rights, descriptions           │
│     ICC: color profile (see #4)                          │
│     Source: from decode probe, passed through pipeline    │
│     Policy: strip, preserve, or selective                │
│                                                          │
│  7. SIDECAR EMBEDDING                                    │
│     Gain map:                                            │
│       → ProcessedSidecar from sidecar pipeline            │
│       → Encode gain map pixels (codec-specific format)    │
│       → Embed in container:                              │
│           JPEG: MPF structure (UltraHDR)                 │
│           AVIF: tmap auxiliary item                       │
│           JXL: jhgm container box                        │
│       → Include ISO 21496-1 metadata                     │
│     Depth map:                                           │
│       → Similar to gain map                              │
│       → JPEG: MPF, HEIC: auxl                            │
│                                                          │
│  8. SUPPLEMENT PASSTHROUGH                                │
│     Thumbnails: regenerate at output size? or strip?      │
│     MPF segments: only if same container format           │
│     Policy: SupplementPolicy (Preserve/Strip/Only)        │
│                                                          │
│  9. STREAMING EXECUTION                                  │
│     Encoder accepts strips via push_rows()                │
│     Sidecar is small — materialized, passed before encode │
│     Metadata/supplements attached before first strip      │
│     finish() produces final encoded bytes                 │
│                                                          │
│  10. ANIMATION                                           │
│      Per-frame: repeat steps 2-6 for each frame           │
│      Frame timing, disposal, blend mode                   │
│      Shared palette (GIF) or per-frame encode (WebP)      │
│      Gain maps don't apply to animation                   │
│                                                          │
└──────────────────────────────────────────────────────────┘
```

## Who owns what

```
zencodecs:
  ├── Format resolution (FormatDecision)
  ├── Sidecar extraction from source container
  ├── Sidecar embedding into output container
  ├── Metadata extraction from source
  ├── Metadata embedding into output
  ├── Supplement policy enforcement
  ├── Streaming encoder construction (streaming_encoder)
  ├── Animation frame decode/encode coordination
  └── Pixel format negotiation (supported descriptors)

zenpipe:
  ├── Pixel pipeline (streaming Source chain)
  ├── Sidecar geometry tracking (SidecarPlan)
  ├── Format conversion insertion (ensure_format)
  ├── Alpha → matte compositing (RemoveAlpha node)
  ├── CMS color conversion (IccTransformSource)
  ├── Streaming execution (execute source → sink)
  └── Orientation tracking (remaining_orientation)

zennode:
  └── Schema definitions, derive macro, KV parsing, registry
      (no execution, no encoding)

Codec crates (zenjpeg, etc.):
  ├── EncoderConfig + with_generic_quality()
  ├── Encoder implementation (push_rows, finish)
  ├── Format-specific sidecar encoding (gain map → MPF/tmap/jhgm)
  ├── Metadata serialization into format containers
  └── Node schema (EncodeJpeg) + to_encoder_config()
```

## The encode function in zenpipe

```rust
/// Full encode: pixel source → encoded bytes.
///
/// Handles all 10 concerns above.
pub fn encode(
    /// Streaming pixel source (output of the processing pipeline).
    source: Box<dyn Source>,

    /// How encoding was requested. Exactly one of:
    /// - FormatDecision (from zencodecs, paths 1-4, 6-7)
    /// - Explicit codec node (path 5)
    encode_request: EncodeRequest,

    /// Metadata to embed. From decode probe, filtered by policy.
    metadata: Option<ImageMetadata>,

    /// Processed sidecar (gain map). From SidecarPlan pipeline.
    /// None if no gain map, or if hdr_mode = sdr_only.
    sidecar: Option<ProcessedSidecar>,

    /// Remaining orientation not applied by the pixel pipeline.
    /// If the encoder supports EXIF orientation, this becomes a tag.
    /// Otherwise it was already applied as pixel transforms.
    remaining_orientation: Orientation,

    /// Codec registry (which codecs are available).
    registry: &CodecRegistry,
) -> Result<EncodeOutput, PipeError>
```

```rust
pub enum EncodeRequest {
    /// Format auto-selected or profile-resolved by zencodecs.
    Decision(FormatDecision),
    /// Explicit codec config from a direct codec node.
    /// The node's `to_encoder_config()` produces the config.
    DirectConfig {
        format: ImageFormat,
        config: Box<dyn Any + Send>,  // downcast to codec-specific config
    },
}
```

## Internal flow

```
encode(source, request, metadata, sidecar, orientation, registry):

  1. Resolve encoder config
     Decision → registry.config_for_format(decision.format)
                  .with_generic_quality(decision.quality)
                  .apply_hints(decision.hints)
     DirectConfig → use as-is

  2. Build streaming encoder
     encoder = zencodecs::streaming_encoder(format, config, w, h, registry)

  3. Attach metadata (before first strip)
     if metadata: encoder.set_metadata(metadata)
     if orientation can be EXIF tag: encoder.set_orientation(orientation)

  4. Attach sidecar (before first strip, small — already materialized)
     if sidecar: encoder.set_gain_map(sidecar.pixels, sidecar.params)

  5. Adapt pixel format
     source = ensure_format(source, encoder.preferred_descriptor())
     if source has alpha and format doesn't support alpha:
       source = apply_matte(source, matte_color)

  6. Stream
     while strip = source.next():
       encoder.push_rows(strip)

  7. Finish
     output = encoder.finish()
     return output  // includes embedded metadata, sidecar, supplements
```

## Animation variation

```
encode_animated(
    frame_source: FrameSource,
    encode_request, metadata, registry,
):
  encoder = animated_encoder(format, config, registry)
  encoder.set_metadata(metadata)  // once, before frames

  for frame in frame_source:
    // Each frame goes through steps 5-6 above
    adapted = ensure_format(frame.source, encoder.preferred_descriptor())
    if needs_matte: adapted = apply_matte(adapted, matte)
    encoder.push_frame(adapted, frame.duration, frame.disposal)

  output = encoder.finish()
```

## What changes in zenpipe

Currently zenpipe has `EncoderSink` that wraps a `DynEncoder`. The full
`encode()` function above replaces manual sink construction with a
higher-level API that handles metadata, sidecars, format adaptation,
and orientation automatically.

`EncoderSink` stays as the low-level primitive. `encode()` builds on it.
