# Changelog

## [Unreleased]

### Changed

- Dependency requirements refreshed: `syn` 3.0.4 → 3.0.10 in `zennode-derive`;
  `serde_json` 1.0.151 → 1.0.154 in `zennode`; lockfiles and `docs/public-api/`
  internal snapshot counts refreshed (`assert_fields_are_eq` on fieldless enums).
- Remove banned `macos-26-intel` runner from `.github/workflows/ci.yml`.
- Dependency requirements refreshed to the current releases: `quote` 1.0.45 →
  1.0.47 and `proc-macro2` 1.0.106 → 1.0.107 in `zennode-derive`; `serde`
  1.0.228 → 1.0.229 and `serde_json` 1.0.149 → 1.0.151 in `zennode` (both the
  optional dependency and the dev-dependency); `zenutils-apidoc` 0.1.0 → 0.1.1
  in the workspace-excluded apidoc runner. `syn` was already on 3.0.4. Derive
  output is exercised by `zennode/tests/derive_node.rs` and
  `derive_enum.rs`; the suite is unchanged at 7 suites / 77 passed / 0 failed.

### Fixed
- **Pushes to `main` now cancel their superseded CI runs.** `ci.yml` keyed its concurrency group on `${{ github.head_ref || github.run_id }}`. `github.head_ref` is populated only for `pull_request` events, so on a push it was empty and the group fell through to `github.run_id` — unique per run, so no two pushes ever shared a group and `cancel-in-progress` could never fire. Every push started a full matrix that ran to completion even when several commits landed seconds apart. Now keyed on `${{ github.ref }}`, which is set for both event types (`refs/heads/main` on push, `refs/pull/N/merge` on a PR), so PR cancellation is unchanged and consecutive pushes supersede each other.

### Added
- Versioned public-API surface snapshots at `docs/public-api/<crate>.txt` for
  `zennode` and `zennode-derive`, regenerated on every `cargo test` run via
  `zennode/tests/public_api_doc.rs` (`ZEN_API_DOC=check` verifies in CI,
  `=off` skips); `justfile` recipes `api-doc` / `api-doc-check`

### Changed
- `zennode-derive` now builds on `syn 3.0.4` (was `2.0.117`). Build-time only — `syn` never appears in the public surface of a proc-macro crate, so this is not a breaking change for consumers. It *removes* a duplicate `syn` from the dependency graph rather than adding one: `serde_derive 1.0.229` already requires `syn ^3`, so any consumer that enables `zennode/serde` was compiling both 2.x and 3.x. Verified by building the local `zennode-derive` against `zennode`'s derive tests through a temporary `[patch.crates-io]` (the workspace's `zennode-derive` dependency resolves from the registry, not by path, so the in-tree macro source is otherwise not exercised by `cargo test`): 51/51 tests pass.
- Exclude `tests/`, `ENCODE-FULL.md`, `ENCODE-PATHS.md`, `SPEC.md` from published packages; add `.workongoing`/`.jj/` to `.gitignore`
- Overhaul `README.md` (current-API quick start, registry + graph-topology/`inputs` docs, crosslink footer) and ship a generated crates.io-only `README.crates.md`; both crates' `readme` now points at it

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
