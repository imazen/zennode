# zennode ![CI](https://img.shields.io/github/actions/workflow/status/imazen/zennode/ci.yml?style=for-the-badge) ![MSRV](https://img.shields.io/badge/MSRV-1.85-blue?style=for-the-badge) ![License](https://img.shields.io/badge/license-Apache--2.0%20OR%20MIT-blue?style=for-the-badge)

A self-describing node definition system for image processing pipelines.

zennode provides the trait infrastructure, derive macros, parameter schemas, RIAPI querystring parsing, JSON Schema generation, and node registry that the zen ecosystem uses to define pipeline operations. It defines no nodes itself -- nodes live in the crates that implement them.

## Quick Start

```rust
use zennode::*;

#[derive(Node, Clone, Debug, Default)]
#[node(id = "myfilter.brightness", group = Tone, role = Filter)]
pub struct Brightness {
    /// Amount of brightness adjustment
    #[param(range(-1.0..=1.0), default = 0.0, identity = 0.0, step = 0.05)]
    #[param(unit = "", section = "Main")]
    pub amount: f32,
}
```

The `#[derive(Node)]` macro generates a `NodeDef` implementation (the factory/schema holder), a `NodeInstance` implementation (a live instance with parameter values), and a static singleton -- all from the struct definition. Doc comments on fields become parameter descriptions. The full `NodeSchema` is available at zero cost through `&'static` references.

Parameter enums get their own derive:

```rust
#[derive(NodeEnum, Clone, Debug, Default)]
pub enum FitMode {
    #[default]
    #[variant(label = "Max", description = "Fit within bounds")]
    Max,
    #[variant(label = "Crop", description = "Fill bounds, crop excess")]
    Crop,
}
```

## What It Provides

**Schema introspection.** Every node carries a `NodeSchema` with 13 fields (id, label, description, group, role, params, tags, coalesce info, format hints, version, compat_version, json_key, deny_unknown_fields). Every parameter carries a `ParamDesc` with 14 fields (name, label, description, kind, unit, section, slider mapping, kv_keys, since_version, visible_when, optional, json_name, json_aliases). Parameter types span 11 `ParamKind` variants: Float, Int, U32, Bool, Str, Enum, FloatArray, Color, Json, Object, and TaggedUnion.

**Node registry.** `NodeRegistry` aggregates node definitions from across the ecosystem. Look up nodes by id, group, or tag. Create instances from parameter maps. Parse RIAPI querystrings against all registered nodes (with consumption tracking and warnings for unrecognized keys). Generate Markdown documentation for every registered node.

**RIAPI querystring parsing.** `KvPairs` is a consumption-tracking parser -- multiple node definitions each claim their relevant keys from the same querystring, and unconsumed keys generate warnings. Typed accessors (`take_f32`, `take_i32`, `take_bool`) handle parsing and error reporting.

**Operation coalescing.** `NodeRole` (9 roles: Decode, Geometry, Orient, Resize, Filter, Composite, Analysis, Quantize, Encode) tells the pipeline bridge what kind of planner a node feeds into. `CoalesceInfo` marks adjacent compatible nodes for fusion into a single operation -- always equivalent to sequential execution, never reordering.

**JSON round-trip.** With the `serde` feature, the registry can serialize/deserialize individual nodes and full pipelines as JSON. Nodes serialize as `{"json_key": {...params...}}`, pipelines as arrays. Unknown field rejection is opt-in per node via `#[node(deny_unknown_fields)]`.

**JSON Schema generation.** With the `json-schema` feature, generate JSON Schema 2020-12 documents with `x-zennode-*` extensions for slider mappings, units, sections, and identity values. Produces both per-node schemas and full pipeline schemas suitable for OpenAPI 3.1.

## Feature Flags

| Feature | Default | Description |
|---------|---------|-------------|
| `std` | yes | `std::error::Error` impl |
| `derive` | yes | `#[derive(Node)]` and `#[derive(NodeEnum)]` macros |
| `serde` | no | Serialize/Deserialize on param types, JSON pipeline round-trip (implies `std`) |
| `json-schema` | no | JSON Schema 2020-12 generation (implies `serde`) |

The library is `no_std + alloc` compatible when both `std` and `derive` are disabled.

## Integration Pattern

Each sibling crate in the zen ecosystem follows the same pattern:

1. Add zennode as an optional dependency behind a feature flag:
   ```toml
   [dependencies]
   zennode = { version = "0.1", path = "../zennode/zennode", optional = true }
   ```

2. Define nodes in a `zennode_defs` module, feature-gated:
   ```rust
   #[cfg(feature = "zennode")]
   pub mod zennode_defs;
   ```

3. That module defines node structs with `#[derive(Node)]` and provides a `register()` function:
   ```rust
   pub fn register(registry: &mut zennode::NodeRegistry) {
       registry.register(&ENCODE_JPEG_DEF);
       registry.register(&DECODE_JPEG_DEF);
   }
   ```

4. The aggregator crate (zenpipe) calls each crate's `register()` behind feature flags to build a `full_registry()` containing every available node.

This pattern is used by zenjpeg, zenpng, zenwebp, zengif, zenavif, zenjxl, zentiff, zenbitmaps, zenfilters, zenresize, zenlayout, zenquant, and zencodecs.

## Workspace Structure

```
zennode/          # Workspace root
  zennode/        # Library crate (traits, schema, registry, KV parsing)
  zennode-derive/ # Proc macro crate (#[derive(Node)], #[derive(NodeEnum)])
```

## Image tech I maintain

| | |
|:--|:--|
| State of the art codecs<sup>[1]</sup> | [zenjpeg] · [zenpng] · [zenwebp] · [zengif] · [zenavif] ([rav1d-safe] · [zenrav1e] · [zenavif-parse] · [zenavif-serialize]) · [zenjxl] ([jxl-encoder] · [zenjxl-decoder]) · [zentiff] · [zenbitmaps] · [heic] · [zenraw] · [zenpdf] · [ultrahdr] · [mozjpeg-rs] · [webpx] |
| Compression | [zenflate] · [zenzop] |
| Processing | [zenresize] · [zenfilters] · [zenquant] · [zenblend] |
| Metrics | [zensim] · [fast-ssim2] · [butteraugli] · [resamplescope-rs] · [codec-eval] · [codec-corpus] |
| Pixel types & color | [zenpixels] · [zenpixels-convert] · [linear-srgb] · [garb] |
| Pipeline | [zenpipe] · [zencodec] · [zencodecs] · [zenlayout] · **zennode** |
| ImageResizer | [ImageResizer] (C#) — 24M+ NuGet downloads across all packages |
| [Imageflow][] | Image optimization engine (Rust) — [.NET][imageflow-dotnet] · [node][imageflow-node] · [go][imageflow-go] — 9M+ NuGet downloads across all packages |
| [Imageflow Server][] | [The fast, safe image server](https://www.imazen.io/) (Rust+C#) — 552K+ NuGet downloads, deployed by Fortune 500s and major brands |

<sup>[1]</sup> <sub>as of 2026</sub>

### General Rust awesomeness

[archmage] · [magetypes] · [enough] · [whereat] · [zenbench] · [cargo-copter]

[And other projects](https://www.imazen.io/open-source) · [GitHub @imazen](https://github.com/imazen) · [GitHub @lilith](https://github.com/lilith) · [lib.rs/~lilith](https://lib.rs/~lilith) · [NuGet](https://www.nuget.org/profiles/imazen) (over 30 million downloads / 87 packages)

## License

Apache-2.0 OR MIT

[zenjpeg]: https://github.com/imazen/zenjpeg
[zenpng]: https://github.com/imazen/zenpng
[zenwebp]: https://github.com/imazen/zenwebp
[zengif]: https://github.com/imazen/zengif
[zenavif]: https://github.com/imazen/zenavif
[zenjxl]: https://github.com/imazen/zenjxl
[zentiff]: https://github.com/imazen/zentiff
[zenbitmaps]: https://github.com/imazen/zenbitmaps
[heic]: https://github.com/imazen/heic-decoder-rs
[zenraw]: https://github.com/imazen/zenraw
[zenpdf]: https://github.com/imazen/zenpdf
[ultrahdr]: https://github.com/imazen/ultrahdr
[jxl-encoder]: https://github.com/imazen/jxl-encoder
[zenjxl-decoder]: https://github.com/imazen/zenjxl-decoder
[rav1d-safe]: https://github.com/imazen/rav1d-safe
[zenrav1e]: https://github.com/imazen/zenrav1e
[mozjpeg-rs]: https://github.com/imazen/mozjpeg-rs
[zenavif-parse]: https://github.com/imazen/zenavif-parse
[zenavif-serialize]: https://github.com/imazen/zenavif-serialize
[webpx]: https://github.com/imazen/webpx
[zenflate]: https://github.com/imazen/zenflate
[zenzop]: https://github.com/imazen/zenzop
[zenresize]: https://github.com/imazen/zenresize
[zenfilters]: https://github.com/imazen/zenfilters
[zenquant]: https://github.com/imazen/zenquant
[zenblend]: https://github.com/imazen/zenblend
[zensim]: https://github.com/imazen/zensim
[fast-ssim2]: https://github.com/imazen/fast-ssim2
[butteraugli]: https://github.com/imazen/butteraugli
[zenpixels]: https://github.com/imazen/zenpixels
[zenpixels-convert]: https://github.com/imazen/zenpixels
[linear-srgb]: https://github.com/imazen/linear-srgb
[garb]: https://github.com/imazen/garb
[zenpipe]: https://github.com/imazen/zenpipe
[zencodec]: https://github.com/imazen/zencodec
[zencodecs]: https://github.com/imazen/zencodecs
[zenlayout]: https://github.com/imazen/zenlayout
[Imageflow]: https://github.com/imazen/imageflow
[Imageflow Server]: https://github.com/imazen/imageflow-server
[imageflow-dotnet]: https://github.com/imazen/imageflow-dotnet
[imageflow-node]: https://github.com/imazen/imageflow-node
[imageflow-go]: https://github.com/imazen/imageflow-go
[ImageResizer]: https://github.com/imazen/resizer
[archmage]: https://github.com/imazen/archmage
[magetypes]: https://github.com/imazen/archmage
[enough]: https://github.com/imazen/enough
[whereat]: https://github.com/lilith/whereat
[zenbench]: https://github.com/imazen/zenbench
[cargo-copter]: https://github.com/imazen/cargo-copter
[resamplescope-rs]: https://github.com/imazen/resamplescope-rs
[codec-eval]: https://github.com/imazen/codec-eval
[codec-corpus]: https://github.com/imazen/codec-corpus
