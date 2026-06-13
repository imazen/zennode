# zennode ![CI](https://img.shields.io/github/actions/workflow/status/imazen/zennode/ci.yml?style=flat-square&label=CI) ![crates.io](https://img.shields.io/crates/v/zennode?style=flat-square) [![lib.rs](https://img.shields.io/crates/v/zennode?style=flat-square&label=lib.rs&color=blue)](https://lib.rs/crates/zennode) ![docs.rs](https://img.shields.io/docsrs/zennode?style=flat-square) ![License](https://img.shields.io/crates/l/zennode?style=flat-square)

Self-documenting **node definitions** for image-processing pipelines.

`zennode` is a trait-based system for declaring pipeline operations with full parameter schemas, [RIAPI](https://github.com/imazen/imageflow)-style querystring parsing, and JSON Schema generation — designed for permanent backwards compatibility. You describe an operation's parameters *once*, with ranges/defaults/units, and get validation, querystring parsing, machine-readable schemas, and docs for free.

It is `#![no_std]` + `alloc` and `#![forbid(unsafe_code)]`.

## Define a node

Derive `Node` on a struct and annotate its parameters:

```rust
use zennode::Node;

#[derive(Node, Clone, Debug, Default)]
#[node(id = "filter.brightness", group = Tone, role = Filter)]
pub struct Brightness {
    /// Amount of brightness adjustment.
    #[param(range(-1.0..=1.0), default = 0.0, identity = 0.0, step = 0.05)]
    #[param(unit = "", section = "Main")]
    pub amount: f32,
}
```

- `id` is the stable identifier used in querystrings and the registry.
- `group` / `role` classify the node (`role` doubles as the pipeline `Phase`).
- `#[param(...)]` captures the range, default, the `identity` value (the no-op setting), step, unit, and UI section. The doc comment becomes the parameter's description.

## Register and drive nodes

The derive emits a `static BRIGHTNESS_NODE` (a `&'static dyn NodeDef`, named `<STRUCT_NAME>_NODE` in screaming-snake-case). Register it into a `NodeRegistry`, which then parses querystrings, instantiates nodes, and emits schemas:

```rust
use zennode::{NodeRegistry, ParamMap, NodeGroup};

let mut registry = NodeRegistry::new();
registry.register(&BRIGHTNESS_NODE);            // or register_all(&[&BRIGHTNESS_NODE, ..])

// Parse a RIAPI-style querystring into validated key/value params + warnings.
let parsed = registry.from_querystring("brightness.amount=0.2");

// Look up and construct a node instance from params.
if let Some(def) = registry.get("filter.brightness") {
    let mut params = ParamMap::new();
    // params.set(...) per the node's schema
    let _node = registry.create("filter.brightness", &params)?;
}

// Browse the registry, or emit human-readable docs.
let tone_nodes = registry.by_group(NodeGroup::Tone);
let markdown_docs = registry.to_markdown();
# Ok::<(), zennode::NodeError>(())
```

Querystring parsing is lenient and reports problems via `KvWarning` rather than failing hard, so an unknown or out-of-range key degrades gracefully instead of rejecting the whole request — the right default for a public image URL API.

## Key types

| type | purpose |
|------|---------|
| `#[derive(Node)]` / `#[derive(NodeEnum)]` | generate the schema + `NodeDef` from a struct/enum |
| `NodeDef` / `NodeInstance` | the static definition vs. a constructed, parameterized node |
| `NodeRegistry` | register nodes; `from_querystring`, `create`, `get`, `by_group`, `by_tag`, `to_markdown` |
| `ParamMap` / `ParamValue` | validated parameter values |
| `NodeRole` (= `Phase`) / `NodeGroup` | classification |
| `VersionSet` | backwards-compatibility versioning |
| `NodeError`, `KvWarning` | typed errors and non-fatal parse warnings |

## Features

| feature | default | effect |
|---------|---------|--------|
| `derive` | ✅ | `#[derive(Node)]` and `#[derive(NodeEnum)]` |
| `std` | ✅ | `std::error::Error` impl (omit for `no_std`) |
| `serde` | — | `Serialize`/`Deserialize` on the param/schema types |
| `json-schema` | — | JSON Schema generation (implies `serde`) |

## License

Apache-2.0 OR MIT.
