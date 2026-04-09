# Schema Versioning, Enforcement, and Codegen Plan

Status: design draft. Not yet implemented.

This document describes the plan for evolving the zennode schema system to
support multi-version nodes, mechanically-enforced compatibility, and
multi-language code generation for downstream client SDKs.

## Goals

1. **Single source of truth** per `(node_id, major_version)` pair: the Rust
   struct annotated with `#[derive(Node)]` in the crate that owns it.
2. **Breaking changes fail the build** — narrowing a numeric range, removing
   a parameter, removing an enum variant, or otherwise violating wire-level
   backward compatibility within an existing major version causes CI to fail
   at both the crate that introduced the change and any downstream aggregator
   (such as `zenpipe`) that pins the schema.
3. **Additive changes require explicit snapshot refresh** but do not break
   downstream builds, with the refresh step itself running compatibility
   checks so it cannot silently accept breaking changes.
4. **Multi-language code generation** derives every language client
   (Rust, C#, TypeScript, Go, Python) from the same schema artifacts with
   coordinated version pinning via named profiles.
5. **Clients that only generate wire format** (JSON, RIAPI querystrings, CLI
   argv, HTTP bodies) pay zero compile-time cost for the execution engine.
   Only clients that explicitly want in-process execution pull in the full
   zen crate tree.

## Two-Dimensional Versioning

Schema versioning uses a `(major, minor)` pair per node:

- **Major version** = a separate Rust type in a separate module. Breaking
  changes create new major versions. Old majors stay registered and runnable
  for as long as the profile supports them.
- **Minor version** = additive evolution within a single Rust type. Tracked
  by `#[param(since_minor = N)]` attributes on individually-added parameters.

**Rule: minor = same struct, major = new struct.** Never conflate the two.

### Source of truth: the Rust struct

Each node's canonical definition is a `#[derive(Node)]` struct in a versioned
module within its owning crate. Example layout:

```text
zenfilters/src/filters/contrast/
├── mod.rs      // pub use v2::Contrast; plus pub mod v1, pub mod v2
├── v1.rs       // frozen — old algo, kept for reproducibility
└── v2.rs       // current — active development, minor version bumps here
```

```rust
// zenfilters/src/filters/contrast/v2.rs
#[derive(Node, Clone, Debug, Default)]
#[node(id = "zenfilters.contrast", major = 2, minor = 3)]
#[node(group = Tone, role = Filter, coalesce = "fused_adjust")]
#[non_exhaustive]
pub struct Contrast {
    /// Contrast strength (positive = increase, negative = flatten).
    #[param(range(-1.0..=1.0), default = 0.0, identity = 0.0, step = 0.05)]
    #[param(slider = SquareFromSlider, unit = "", section = "Main")]
    #[kv("s.contrast")]
    pub amount: f32,

    /// Pivot point for the power curve. Added in 2.1.
    #[param(range(0.0..=1.0), default = 0.5691, step = 0.01, since_minor = 1)]
    #[param(section = "Advanced")]
    pub pivot: f32,
}

impl Filter for Contrast { /* SIMD power curve */ }
```

The derive macro captures all metadata into a static `&'static NodeSchema`.
JSON Schema, language clients, and documentation are all derived from this
single declaration.

### Schema fields added for the two-dimensional model

```rust
pub struct NodeSchema {
    pub id: &'static str,
    pub major: u32,               // NEW: Rust-type-level versioning
    pub minor: u32,               // NEW: additive-evolution versioning
    pub compat_minor: u32,        // oldest minor this code can deserialize
    // ... other existing fields
}

pub struct ParamDesc {
    pub since_minor: u32,         // NEW: minor in which this param was added
                                  //      (meaningful only within one major)
    // ...
}
```

The existing `version: u32` and `compat_version: u32` fields are superseded
by `major`/`minor`/`compat_minor`. A compatibility shim keeps `version()` and
`compat_version()` as method accessors during the transition.

## Enforcement Mechanism

The enforcement strategy is **per-major snapshot files with additive-compat
checking**, in two layers.

### Layer 1: per-crate snapshots

Each zen crate commits a snapshot file per `(node_id, major)` in a
`schemas/` directory adjacent to its source:

```text
zenfilters/
├── src/filters/contrast/
│   ├── v1.rs
│   └── v2.rs
└── schemas/
    └── zenfilters.contrast/
        ├── v1.json       # frozen — any byte-level change fails CI
        └── v2.json       # updates allowed only for additive changes
```

Each snapshot file is the full JSON Schema for that `(node_id, major)` pair,
as emitted by `zennode::json_schema::node_to_json_schema`. Reusing the
existing export format means no translation layer introduces ambiguity
between runtime output and snapshot data.

A per-crate test walks the registry and, for every `(id, major)` with a
committed snapshot, runs the additive-compat checker:

```rust
// zenfilters/tests/schema_snapshot.rs
// Generated test per (node_id, major) — the #[test] bodies are emitted
// at build time by xtask based on files in schemas/.

#[test]
fn snapshot_zenfilters_contrast_v2() {
    zennode::snapshot::assert_snapshot_matches(
        zenfilters::filters::contrast::v2::CONTRAST_NODE.schema(),
        include_str!("../schemas/zenfilters.contrast/v2.json"),
    );
}
```

Behavior of `assert_snapshot_matches`:

- **Exact match** — test passes silently.
- **Additive-only diff** — test emits a warning with the command to refresh
  (`cargo xtask snapshot --node zenfilters.contrast --major 2`). In CI, the
  test is **configured to fail** when the snapshot is out of date, forcing
  drift to be committed. In local development, the behavior can be set to
  warn-only via an environment variable.
- **Breaking diff** — test fails with a detailed `CompatReport`, listing
  each breaking change with field names and old/new values, plus the
  instruction to either revert the change or create a new major version.

### Layer 2: zenpipe aggregate snapshots

`zenpipe` is the aggregator that walks `full_registry()` to collect every
node from every registered zen crate. It also commits its own copy of the
schemas for every node it references:

```text
zenpipe/
└── schemas/
    ├── zenpipe/                           # nodes zenpipe owns directly
    │   ├── zenpipe.composite/v1.json
    │   └── zenlayout.constrain/v1.json
    └── imported/                          # pinned copies from dependencies
        ├── zenfilters.contrast/v2.json
        ├── zenfilters.exposure/v2.json
        └── zencodecs.encode_jpeg/v1.json
```

`zenpipe`'s CI runs an aggregate snapshot test that:

1. Walks `full_registry()` and emits the current schema for every
   registered `(id, major)`.
2. For each, loads the matching file under `schemas/imported/` (or
   `schemas/zenpipe/` for zenpipe's own nodes).
3. Runs the additive-compat checker between imported snapshot and current
   runtime schema.
4. **Fails on any difference**, not just breaking changes. `zenpipe` pins
   schemas at exact byte level — if a zen dep changed anything, explicit
   refresh is required.

An explicit `cargo xtask refresh-imported-schemas` command regenerates the
`imported/` directory from the current dep versions. The refresh task
itself runs `check_additive_compat` between the previous imported snapshot
and the new one, and **refuses to overwrite on breaking changes**.

This gives two layers of enforcement:

- **Layer 1** (zen crate) catches breakage at the earliest possible point,
  before the crate can publish.
- **Layer 2** (zenpipe) catches breakage that slipped past Layer 1 (force
  pushes, CI bypass, yanked-and-republished crates, incompatible version
  bumps) before zenpipe can pick up the change.

## Additive Compatibility Rules

The `zennode::snapshot::check_additive_compat` function applies the
following rules:

| Change | Verdict | Version bump |
|---|---|---|
| Add optional parameter (`Option<T>` or with default) | additive | minor |
| Add required parameter | **breaking** | new major |
| Remove any parameter | **breaking** | new major |
| Rename parameter with `#[param(json_alias = "old_name")]` | additive | minor |
| Rename parameter without alias | **breaking** | new major |
| Change parameter kind (f32 → i32, String → Enum) | **breaking** | new major |
| Numeric `min` decreases or `max` increases (widening) | additive | minor |
| Numeric `min` increases or `max` decreases (narrowing) | **breaking** | new major |
| Enum variant added | additive | minor |
| Enum variant removed | **breaking** | new major |
| KV alias added | additive | minor |
| KV key removed | **breaking** | new major (old querystrings would stop matching) |
| Default value changed | warning | minor with warning |
| Identity value changed | warning | minor with warning |
| Step value changed | silent | minor |
| Unit / label / section / description changed | silent | minor |
| `since_minor` added to new param | additive | minor |
| `compat_minor` increased | **breaking** (drops support for older minors) | evaluated separately |
| `major` mismatched with committed snapshot | **breaking** (always) | new major |

The `CompatReport` returned by `check_additive_compat` lists all detected
changes, categorized into `breaking: Vec<BreakingChange>` and
`additive: Vec<AdditiveChange>`. A breaking change list with any entries
blocks the build; an additive-only change list prompts snapshot refresh.

## Wire Format and Profiles

### Wire format versioning

The wire envelope carries an explicit API version that maps to a set of
major versions per node:

```json
{
  "api_version": "zen-2026-q2",
  "framewise": {
    "steps": [
      {"decode":      {"io_id": 0}},
      {"constrain":   {"w": 800, "mode": "fit_crop"}},
      {"contrast":    {"amount": 0.5}},
      {"contrast@1":  {"amount": 0.5}},
      {"encode_jpeg": {"quality": 85}}
    ]
  }
}
```

Resolution rules:

1. **Unpinned node name** (`"contrast"`): look up in the active profile. If
   present, use that major. Otherwise, fall through to `latest_major()` (or
   fail if the profile is in strict mode).
2. **`@N` suffix** (`"contrast@1"`): use major N exactly. Fail if the
   registry does not contain a `NodeDef` for that `(id, major)` pair.
3. **`api_version` omitted**: use the library's compiled-in default profile.

### API profiles

A profile is an immutable, named coordinate describing:

- Exact versions of every zen crate it references
- Major version of every node it exposes
- Whether unlisted nodes fall through to `latest_major()` or are rejected

```toml
# profiles/zen-2026-q2.toml
name = "zen-2026-q2"
description = "Q2 2026 profile — adds contrast v2 and exposure v2"

[crates]
zenfilters  = "0.4.2"
zencodecs   = "0.3.1"
zenresize   = "0.5.0"
zenpipe     = "0.8.0"

[nodes]
"zenfilters.contrast"  = 2    # default major
"zenfilters.exposure"  = 2
"zenresize.constrain"  = 1
"zencodecs.encode_jpeg" = 1
# ... every node explicitly listed

[resolution]
fallback = "latest_major"     # or "strict" to reject unlisted nodes
```

Profiles are published on a cadence (roughly quarterly). **Old profiles are
never modified.** If a node needs to be retroactively added or changed,
create a new profile with a new name.

### Registry lookup

The registry index becomes `(id, major)`:

```rust
pub struct RegistryEntry {
    pub def: &'static dyn NodeDef,
    pub id: &'static str,
    pub major: u32,
    pub minor: u32,
}

impl NodeRegistry {
    /// Resolve (id, major) → NodeDef.
    pub fn get_versioned(&self, id: &str, major: u32) -> Option<&'static dyn NodeDef>;

    /// Get the latest registered major for an id.
    pub fn latest_major(&self, id: &str) -> Option<u32>;

    /// Look up using an active profile (profile resolves unpinned names).
    pub fn get_with_profile(&self, id: &str, profile: &Profile) -> Option<&'static dyn NodeDef>;
}
```

## Multi-Language Code Generation

### Architecture

`zen-codegen` is a language-neutral Rust crate that reads JSON Schema
(with zennode's `x-zennode-*` extensions) and emits source files for
target languages. Modular by language with shared helpers:

```rust
// zen-codegen/src/lib.rs
pub struct CodegenInput {
    pub profile_name: String,
    pub schemas: serde_json::Value,        // aggregated_nodes.json
    pub qs_keys: serde_json::Value,        // aggregated_qs_keys.json
    pub profile_config: ProfileConfig,     // node → major mapping
}

pub struct GeneratedFiles {
    pub files: Vec<(std::path::PathBuf, String)>,
}

pub trait Target {
    fn name(&self) -> &'static str;
    fn generate(&self, input: &CodegenInput) -> GeneratedFiles;
}

pub mod rust;        // structs + #[derive(Serialize, Deserialize)]
pub mod csharp;      // class + [DataContract]
pub mod typescript;  // interfaces + runtime validation
pub mod go;          // structs + json tags + methods
pub mod python;      // Pydantic models
```

The existing `zenpipe::codegen_csharp` logic is ported as the initial `csharp`
target. Rust codegen is new. TypeScript, Go, and Python follow.

### Per-target naming

| Source | Rust | C# | TS | Go | Python |
|---|---|---|---|---|---|
| `zenfilters.contrast` (major 2, default) | `Contrast` | `Contrast` | `Contrast` | `Contrast` | `Contrast` |
| `zenfilters.contrast` (major 2, explicit) | `ContrastV2` | `ContrastV2` | `ContrastV2` | `ContrastV2` | `ContrastV2` |
| `zenfilters.contrast` (major 1, non-default) | `ContrastV1` | `ContrastV1` | `ContrastV1` | `ContrastV1` | `ContrastV1` |
| `zenresize.ConstraintMode::FitCrop` | `ConstraintMode::FitCrop` | `ConstraintMode.FitCrop` | `ConstraintMode.FitCrop` | `ConstraintModeFitCrop` | `ConstraintMode.FIT_CROP` |

### What each target emits

- Typed structs or classes with validating constructors (`new`, `tryNew`,
  `withClamping` or equivalent)
- Typed enums for string-valued parameters (ConstraintMode, ResampleFilter,
  GravityAnchor, etc.) — no stringly-typed params leak through to clients
- Serde derives or equivalent serialization annotations
- Doc comments from schema `description` fields
- A top-level discriminated union (`NodeV3` in Rust, equivalent in each
  other language) with one variant per `(node_id, major)` registered in
  the profile
- Profile-default type aliases (e.g., `type Contrast = ContrastV2` when the
  active profile pins contrast to major 2)
- Wire key resolution respecting `@N` explicit pinning
- Static metadata tables for runtime introspection

### Codegen as a commit step, not a runtime step

All generated code is **committed** to the consuming crates. The generator
runs via `cargo xtask regenerate` when a profile updates. CI verifies that
committed generated files match what the generator would produce for the
pinned profile:

1. Consumer crate's CI runs `cargo xtask regenerate --check`.
2. The check regenerates into a temp directory.
3. Diffs against committed files.
4. Fails if different, instructing the developer to commit the regen.

This preserves fast compile times (no build.rs codegen), makes generated
code reviewable, and ensures no drift between committed source and profile.

## Consumer Design

### Rust consumer: `imageflow-api` (example)

A consumer crate that generates wire format and optionally executes
pipelines. Cargo features separate wire-only from execution:

```toml
[features]
default = []                         # wire types + serializers, no executor

cli        = []                       # CliExecutor via std::process::Command
http       = []                       # HttpTransport trait + HttpExecutor<T>
in-process = [                        # InProcessExecutor via zenpipe
    "dep:zenpipe",
    "dep:zennode",
    "dep:zencodecs",
    "dep:zenfilters",
]
```

Key invariant: **the public type identity does not depend on features.**
`imageflow_api::nodes::Contrast` is always the same generated type, whether
or not `in-process` is on. The feature adds conversion impls
(`NodeV3::into_node_instance()`, `impl From<&Contrast> for zennode::Box<dyn NodeInstance>`)
but never swaps the type identity.

With no features enabled:

- Dependencies: `serde`, `serde_json`, `thiserror`, optionally `url`
- Produces: JSON body, RIAPI querystring, CLI argv vector, HTTP body bytes
- Compile time: fast (seconds)
- Binary size: minimal

With `in-process`:

- Dependencies: add the full zen tree
- Gains: `InProcessExecutor::execute(&Job) -> Result<JobResult, Error>`
- Compile time: slow (depends on full zen build)

### In-process execution path

```rust
#[cfg(feature = "in-process")]
impl NodeV3 {
    pub fn into_node_instance(&self) -> Box<dyn zennode::NodeInstance> {
        match self {
            Self::Contrast(c)   => c.into_node_instance(),
            Self::Exposure(c)   => c.into_node_instance(),
            Self::Constrain(c)  => c.into_node_instance(),
            Self::EncodeJpeg(c) => c.into_node_instance(),
            // One arm per variant, all generated, all trivial
        }
    }
}
```

Each variant's `into_node_instance` is a trivial field copy into the
corresponding zen crate's `zennode_defs::*` struct. This bypasses the
`ParamMap` round-trip of the current bridge and gives direct
typed-struct-to-typed-struct conversion.

## CI Workflow

### Per zen crate

```yaml
jobs:
  build-matrix:
    - cargo build, test, clippy, fmt
    - cargo semver-checks
    - platforms: ubuntu, ubuntu-arm, windows, windows-arm, macos, macos-intel
    - targets: default + i686 via cross + wasm32

  schema-snapshot:
    - cargo test --test schema_snapshot  # per (id, major) tests
    - cargo xtask snapshot --check        # drift detection
```

### zenpipe

```yaml
  aggregate-snapshot:
    - cargo test --test aggregate_snapshot
    - cargo xtask refresh-imported-schemas --check

  schema-export:
    - cargo test --features json-schema,nodes-all dump_schemas_to_files
    - upload /tmp/zen-schemas/ as artifact
```

### Consumer crates (imageflow-api and language clients)

```yaml
  regenerate-check:
    - cargo xtask regenerate --check   # verifies committed generated files

  wire-golden:
    - cargo test --test wire_golden    # serialize reference values,
                                       # diff against committed fixtures

  build-matrix:
    strategy:
      matrix:
        features: ["", "cli", "http", "in-process", "cli,http,in-process"]
        platforms: [ubuntu, windows, windows-arm, macos, macos-intel, wasm32]
```

## Versioning Scheme

Three independent version axes:

1. **Zen crate versions** (`zenfilters 0.3.2`, `zencodecs 0.2.0`):
   Conventional SemVer at the Rust API level. Adding a new node-major is a
   minor crate bump. Removing an old node-major is a major crate bump.

2. **Profile versions** (`zen-2026-q2`):
   Named, immutable once released. Pin exact crate versions and per-node
   majors. New profile = new name; never edit old ones.

3. **Consumer crate versions** (e.g., `imageflow-api 2.0.0`):
   SemVer at the Rust public API level. Switching to a new profile is a
   major bump when it changes generated type identities. Additive profile
   updates (new nodes, new params) are minor bumps.

### Example trajectory

| Event | Zen crate | Profile | Consumer |
|---|---|---|---|
| Add `contrast.pivot` param (additive, minor) | `zenfilters 0.3.3` | — | — |
| Refresh profile with new zenfilters | — | `zen-2026-q2.1` | — |
| Consumer regenerates, picks up new param | — | — | `imageflow-api 2.0.1` |
| Bump contrast to major 2 (new Rust type) | `zenfilters 0.4.0` | — | — |
| Publish new profile with new default major | — | `zen-2026-q3` | — |
| Consumer adopts new profile (new default types) | — | — | `imageflow-api 3.0.0` |

## Semver Enforcement Matrix

| Layer | Tool | Catches |
|---|---|---|
| Zen crate Rust API | `cargo semver-checks` | type/fn rename, removal, signature change |
| Zen crate schema | snapshot test + `check_additive_compat` | range narrowing, param removal, variant removal |
| Zenpipe aggregate | aggregate snapshot test | any zen dep schema change not explicitly refreshed |
| Profile validator | CI script | profile pinning mismatch, missing snapshots |
| Cross-profile compat | CI script | breaking change between consecutive profiles for shared nodes |
| Consumer Rust API | `cargo semver-checks` | generated type rename/removal |
| Consumer wire | golden serialization test | JSON shape changes without major bump |

Every layer is a different check. A breaking change that slips through
requires bypassing all of them simultaneously.

## Worked Examples

### Scenario A: narrow a range

Developer narrows `Contrast.amount` range from `-1.0..=1.0` to `-0.5..=0.5`:

1. Edit `zenfilters/src/filters/contrast/v2.rs`.
2. Run `cargo test` in zenfilters.
3. `snapshot_zenfilters_contrast_v2` calls `check_additive_compat`, detects
   narrowing, fails with:
   ```
   BREAKING: zenfilters.contrast v2 param 'amount' max narrowed from 1.0 to 0.5.

   This is not additive-compatible. Options:
     1. Revert the change.
     2. Bump to v3: create src/filters/contrast/v3.rs with the new struct,
        add it to the registry, commit schemas/zenfilters.contrast/v3.json.
   ```
4. CI blocks the PR.

### Scenario B: breaking change bypasses Layer 1

Hypothetically, zenfilters publishes a broken v2 (CI bypass, force push):

1. Zenpipe CI picks up new zenfilters version.
2. `aggregate_snapshot` test loads `schemas/imported/zenfilters.contrast/v2.json`
   (old wide range) and compares to what `full_registry()` now produces.
3. `check_additive_compat` detects narrowing, test fails with the same
   diagnostic.
4. Developer attempts `cargo xtask refresh-imported-schemas`.
5. Refresh task runs compat check on itself, refuses to overwrite:
   *"zenfilters.contrast v2 narrowed range on 'amount'. Cannot refresh. Revert
   zenfilters or bump to v3."*
6. Zenpipe cannot build or publish until upstream is fixed.

### Scenario C: additive change (happy path)

Developer adds a new optional `Contrast.smoothing` parameter:

1. Edit `zenfilters/src/filters/contrast/v2.rs`, add field with
   `#[param(since_minor = 2)]`.
2. Run `cargo test` in zenfilters.
3. Snapshot test reports *"snapshot out of date; additive change detected"*
   with the xtask command.
4. Run `cargo xtask snapshot --node zenfilters.contrast --major 2`. Snapshot
   file updates, bumping `minor` to 2.
5. Commit the diff (v2.rs + v2.json together).
6. CI passes. Zenfilters publishes 0.3.3.
7. Zenpipe `cargo update` picks up 0.3.3.
8. Zenpipe's `aggregate_snapshot` test reports *"imported snapshot out of date
   (additive)"*.
9. Run `cargo xtask refresh-imported-schemas`. Refresh task runs compat check,
   sees additive-only change, updates `schemas/imported/zenfilters.contrast/v2.json`.
10. Commit the refreshed file. Zenpipe 0.8.x publishes.
11. Profile CI rebuilds `zen-2026-q2`, regenerates aggregated artifacts.
12. Consumer (e.g., imageflow-api) runs `cargo xtask regenerate`, commits
    new generated code with the `smoothing()` accessor.
13. Consumer 2.0.1 publishes.

Every non-automatic step is an explicit human-in-the-loop confirmation.

## Implementation Roadmap

The plan is deliberately staged so each step provides value independently.

1. **Add `zennode::snapshot` module** with `check_additive_compat`,
   `CompatReport`, and the full rule set from the compat table.

2. **Add `major` and `minor` fields to `NodeSchema`** and update the derive
   macro to parse `#[node(major = N, minor = M)]`. Keep back-compat shims
   for the existing `version` field.

3. **Add `since_minor` to `ParamDesc`** and update the derive macro to parse
   `#[param(since_minor = N)]`.

4. **Implement `assert_snapshot_matches`** in `zennode::snapshot` — the
   integration-test helper that zen crates call from their snapshot tests.

5. **Pick a pilot node (e.g., Exposure)** and carry it through the full
   workflow in zenfilters:
   - Merge `filters::Exposure` and `zennode_defs::Exposure` into one type
     in a versioned module (`filters::exposure::v1`)
   - Commit `schemas/zenfilters.exposure/v1.json`
   - Add the snapshot test
   - Wire the snapshot test into CI
   - Verify a deliberate breaking change fails the build as expected

6. **Roll out to remaining zenfilters nodes**, one or two at a time.

7. **Apply the same pattern to zencodecs and zenpipe's own `zennode_defs`.**

8. **Add zenpipe's aggregate snapshot layer** (imported schemas + refresh
   task).

9. **Extract `zen-codegen` as a Rust crate** with Rust and C# targets to
   start. Port `zenpipe::codegen_csharp` as the C# initial implementation.

10. **Add TypeScript, Go, Python targets** to zen-codegen as needed by
    downstream language clients.

11. **Define the profile format** and build the profile validator CI script.

12. **Build the consumer crate skeleton** (typed wire format + three-feature
    executor model) and connect it to the zen-codegen Rust target.

13. **Convert stringly-typed parameters** (`mode`, `down_filter`, `gravity`,
    `align_mode`) to `#[derive(NodeEnum)]` types. This is orthogonal to
    versioning but significantly improves the generated client UX.

14. **Add implementation versioning** (`#[node(impl_version = ...)]`) as a
    third axis only if users with reproducibility requirements ask for it.

## Open Questions

- **Repository layout**: whether zen pipeline crates consolidate into a
  single Cargo workspace, whether the consumer crate lives in its own repo
  or alongside other wire-format crates, and where profile definitions and
  multi-language codegen live. This is a separate planning discussion that
  should converge before implementation starts.

- **Compile-time enforcement**: whether snapshot checking should eventually
  move from integration tests into the derive macro itself (reading the
  snapshot file at compile time and failing the build with a proc-macro
  diagnostic). Tests are sufficient to start; a proc-macro version can be
  added later if test-based enforcement proves insufficient.

- **Profile cadence and naming**: how often new profiles are cut, whether
  they use date-based names (`zen-2026-q2`) or semver-like names
  (`zen-v3.1`), and how old profiles are eventually retired.

- **Implementation version pinning**: whether to expose per-node
  implementation version pinning (different algorithm variants under the
  same schema version) as a user-facing feature or treat it as an internal
  reproducibility concern only.

- **Deprecation policy**: whether profiles keep all majors forever (image
  processing default, optimizes for pipeline reproducibility) or drop old
  majors after a sliding window (API-backend default, optimizes for
  maintenance burden).
