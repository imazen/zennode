# Changelog

## [Unreleased]

### Changed
- Exclude `tests/`, `ENCODE-FULL.md`, `ENCODE-PATHS.md`, `SPEC.md` from published packages; add `.workongoing`/`.jj/` to `.gitignore`

## 0.1.0 — 2026-03-28

Initial release.

### Derive macros (`zennode-derive`)

- `#[derive(Node)]` generates `NodeDef` (factory/schema holder), `NodeInstance` (live
  instance with parameter values), and a `&'static` singleton from a struct definition.
  Doc comments on fields become parameter descriptions.
- `#[derive(NodeEnum)]` generates `Display`/`FromStr` for enum parameters, plus
  `EnumVariant` descriptors with labels and descriptions.
- Structs without `#[node(id)]` generate `NodeParams` only (for nesting as sub-objects).
- Enums with `#[derive(Node)]` generate `NodeParams` with `TaggedVariant` descriptors.

### Schema types

- `NodeSchema` with 13 fields: id, label, description, group, role, params, tags,
  coalesce, format, version, compat_version, json_key, deny_unknown_fields.
- `ParamDesc` with 13 fields: name, label, description, kind, unit, section, slider,
  kv_keys, since_version, visible_when, optional, json_name, json_aliases.
- 11 `ParamKind` variants: Float, Int, U32, Bool, Str, Enum, FloatArray, Color,
  Json, Object, TaggedUnion.
- `ParamValue` runtime enum with typed accessors and `None` for optional params.
- `SliderMapping` (Linear, SquareFromSlider, FactorCentered, Logarithmic, NotSlider).
- `NodeGroup` (18 categories: Decode through Other).
- `NodeParams` trait for recursive schema introspection on nested structs/enums.

### Node registry

- `NodeRegistry`: register, look up by id/group/tag, create instances from `ParamMap`.
- RIAPI querystring parsing across all registered nodes with consumption tracking.
- `to_markdown()` on both `NodeSchema` and `NodeRegistry` for doc generation.

### RIAPI querystring parsing

- `KvPairs`: percent-decoded key-value parser with consumption tracking.
- Typed accessors: `take_f32`, `take_i32`, `take_u32`, `take_bool`, `take_str`,
  `take_enum`.
- `snapshot()` for tracing which keys were consumed and by whom.
- Unconsumed keys surface as warnings.

### Pipeline roles and coalescing

- `NodeRole` with 9 variants: Decode, Geometry, Orient, Resize, Filter, Composite,
  Analysis, Quantize, Encode.
- `CoalesceInfo` for adjacent-node fusion (same group, always equivalent to sequential
  execution, never reordering).

### Feature flags

- `std` (default): `std::error::Error` impl on `NodeError`.
- `derive` (default): re-exports `#[derive(Node)]` and `#[derive(NodeEnum)]`.
- `serde`: JSON round-trip for nodes and pipelines. Nodes serialize as
  `{"json_key": {...params...}}`. Per-node `deny_unknown_fields` opt-in.
- `json-schema`: JSON Schema 2020-12 generation with `x-zennode-*` extensions for
  slider mappings, units, sections, and identity values.
- `no_std + alloc` compatible when both `std` and `derive` are disabled.
