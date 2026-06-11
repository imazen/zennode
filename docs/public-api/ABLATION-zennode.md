# zennode public-API ablation report

**Date:** 2026-06-11
**Snapshot commit:** 36fe88c
**Crates analyzed:** `zennode` (1,194 default / 1,309 all-features items) + `zennode-derive` (3 items)
**Grep template:** `ugrep -r --include="*.rs" --include="*.toml" "<symbol>" /home/lilith/work/ --exclude-dir=target --exclude-dir=.jj`

## Consumer context

zennode is not yet published to crates.io (`# zennode = { path = ... }` commented out in zenjpeg, zenbitmaps, and peer codec Cargo.tomls). The codec `zennode` feature gates are gated behind `# uncomment when on crates.io` comments across the org. No org-wide crate currently takes a live dependency on zennode. The intended consumers are: zenpipe, zencodecs, per-codec node defs (zenjpeg/zenwebp/etc.), imageflow graph engine — all wired up once published.

Because zennode is the infrastructure crate (traits, derive macros, registry, schema), essentially the entire surface IS the intended public API. Conservative stance: flag only things clearly inconsistent with stated design intent.

## Summary

**0 items flagged for action.**

### Observations (informational, no action needed)

1. **`pub type zennode::Phase = zennode::ordering::NodeRole`** — documented as a backwards-compatibility alias in `lib.rs` and in `schema.rs` doc comment. No external consumers found in the current org scan (expected, since crate is unpublished). The alias is intentional and documented. KEEP.

2. **`pub mod zennode::serde_impl`** — module appears in the `all-features` snapshot (line 1847) but contains only trait impls (`Serialize`/`Deserialize`) and zero pub types or free functions. It shows as a bare module entry, which is benign — it's gated by `#[cfg(feature = "serde")]` and serves as the implementation home for serde support on all zennode types. No leaked internals. KEEP.

3. **`pub mod zennode::json_schema`** — gated by `#[cfg(feature = "json-schema")]`. Exposes `node_to_json_schema`, `registry_to_json_schema`, `registry_to_openapi_schemas`, `registry_querystring_keys`, `querystring_to_json_schema`, `querystring_key_registry`, and `QsKey`. These are intentional tooling/schema generation APIs. No external consumers yet (unpublished crate). KEEP.

4. **`pub mod zennode::nodes`** — a documentation-only module (gated by `derive` feature) explaining that no built-in nodes are defined here; nodes belong in consuming crates. Correct and intentional. KEEP.

5. **`zennode-derive`**: 3-item surface (`#[derive(Node)]`, `#[derive(NodeEnum)]`). Minimal and correct.

### No zencodec adapter scan needed

zennode does not wrap zencodec traits and has no streaming-decoder adapter pattern — it is purely a node-definition infrastructure crate.

## Flagged items

| # | Item | Category | Proposal | Confidence |
|---|------|----------|----------|------------|
| — | (none) | — | — | — |

**0 flagged. 0 % of surface.**

## Digest

zennode's public surface is the product: schema types, traits, KV parsing, registry, derive macros, and optional JSON-schema/serde feature-gated modules. All modules serve documented purposes. The `Phase` backwards-compat alias is documented inline. No leaked internals, no zero-consumer items meeting the mistake bar (the crate is unpublished so every item is technically unconsumed externally, but that is expected and noted in the Cargo.toml comments).
