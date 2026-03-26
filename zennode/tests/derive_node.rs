//! Integration tests for `#[derive(Node)]`.

extern crate alloc;

use zennode::*;

/// Exposure adjustment in photographic stops.
/// Each stop doubles or halves brightness.
#[derive(Node, Clone, Debug, Default)]
#[node(id = "test.exposure", group = Tone, role = Filter)]
#[node(coalesce = "fused_adjust")]
#[node(format(preferred = OklabF32, alpha = Skip))]
#[node(tags("basic", "tone"))]
pub struct Exposure {
    /// Exposure compensation in stops
    #[param(range(-5.0..=5.0), default = 0.0, identity = 0.0, step = 0.1)]
    #[param(unit = "EV", section = "Main")]
    #[kv("exposure", "ev")]
    pub stops: f32,
}

/// Resize to specified dimensions.
#[derive(Node, Clone, Debug, Default)]
#[node(id = "test.resize", group = Geometry, role = Geometry)]
#[node(changes_dimensions)]
#[node(format(preferred = LinearF32, alpha = RequirePremul))]
pub struct Resize {
    /// Target width in pixels
    #[param(range(0..=65535), default = 0, step = 1)]
    #[param(unit = "px", section = "Main")]
    #[kv("w", "width")]
    pub width: u32,

    /// Target height in pixels
    #[param(range(0..=65535), default = 0, step = 1)]
    #[param(unit = "px", section = "Main")]
    #[kv("h", "height")]
    pub height: u32,

    /// Whether to use sharp downsampling
    #[param(default = false)]
    #[param(section = "Advanced")]
    #[kv("sharp")]
    pub sharp_yuv: bool,
}

#[test]
fn schema_metadata() {
    let schema = EXPOSURE_NODE.schema();
    assert_eq!(schema.id, "test.exposure");
    assert_eq!(schema.label, "Exposure");
    assert_eq!(schema.group, NodeGroup::Tone);
    assert_eq!(schema.role, NodeRole::Filter);
    assert_eq!(schema.version, 1);
    assert_eq!(schema.tags, &["basic", "tone"]);
    assert!(schema.coalesce.is_some());
    let coal = schema.coalesce.as_ref().unwrap();
    assert_eq!(coal.group, "fused_adjust");
    assert!(coal.fusable);
    assert!(!coal.is_target);
    assert_eq!(schema.format.preferred, PixelFormatPreference::OklabF32);
    assert_eq!(schema.format.alpha, AlphaHandling::Skip);
}

#[test]
fn param_metadata() {
    let schema = EXPOSURE_NODE.schema();
    assert_eq!(schema.params.len(), 1);
    let p = &schema.params[0];
    assert_eq!(p.name, "stops");
    assert_eq!(p.label, "Stops");
    assert_eq!(p.unit, "EV");
    assert_eq!(p.section, "Main");
    assert_eq!(p.kv_keys, &["exposure", "ev"]);
    match &p.kind {
        ParamKind::Float {
            min,
            max,
            default,
            identity,
            step,
        } => {
            assert_eq!(*min, -5.0);
            assert_eq!(*max, 5.0);
            assert_eq!(*default, 0.0);
            assert_eq!(*identity, 0.0);
            assert_eq!(*step, 0.1);
        }
        _ => panic!("expected Float"),
    }
}

#[test]
fn create_default() {
    let instance = EXPOSURE_NODE.create_default().unwrap();
    assert_eq!(instance.schema().id, "test.exposure");
    assert_eq!(instance.get_param("stops"), Some(ParamValue::F32(0.0)));
    assert!(instance.is_identity());
}

#[test]
fn create_with_params() {
    let mut params = ParamMap::new();
    params.insert("stops".into(), ParamValue::F32(1.5));
    let instance = EXPOSURE_NODE.create(&params).unwrap();
    assert_eq!(instance.get_param("stops"), Some(ParamValue::F32(1.5)));
    assert!(!instance.is_identity());
}

#[test]
fn set_and_get_param() {
    let mut exp = Exposure::default();
    assert!(exp.set_param("stops", ParamValue::F32(2.0)));
    assert_eq!(exp.get_param("stops"), Some(ParamValue::F32(2.0)));
    assert!(!exp.set_param("nonexistent", ParamValue::F32(1.0)));
}

#[test]
fn to_params_round_trip() {
    let exp = Exposure { stops: 1.5 };
    let params = exp.to_params();
    assert_eq!(params.get("stops"), Some(&ParamValue::F32(1.5)));

    let instance = EXPOSURE_NODE.create(&params).unwrap();
    assert_eq!(instance.get_param("stops"), Some(ParamValue::F32(1.5)));
}

#[test]
fn from_kv_basic() {
    let mut kv = KvPairs::from_querystring("exposure=1.5");
    let instance = EXPOSURE_NODE.from_kv(&mut kv).unwrap().unwrap();
    assert_eq!(instance.get_param("stops"), Some(ParamValue::F32(1.5)));
    assert_eq!(kv.unconsumed().count(), 0);
}

#[test]
fn from_kv_alias() {
    let mut kv = KvPairs::from_querystring("ev=2.0");
    let instance = EXPOSURE_NODE.from_kv(&mut kv).unwrap().unwrap();
    assert_eq!(instance.get_param("stops"), Some(ParamValue::F32(2.0)));
}

#[test]
fn from_kv_no_match() {
    let mut kv = KvPairs::from_querystring("w=800");
    let result = EXPOSURE_NODE.from_kv(&mut kv).unwrap();
    assert!(result.is_none());
}

#[test]
fn multi_field_node() {
    let schema = RESIZE_NODE.schema();
    assert_eq!(schema.params.len(), 3);
    assert_eq!(schema.params[0].name, "width");
    assert_eq!(schema.params[1].name, "height");
    assert_eq!(schema.params[2].name, "sharp_yuv");
    assert!(schema.format.changes_dimensions);
}

#[test]
fn multi_field_from_kv() {
    let mut kv = KvPairs::from_querystring("w=800&h=600&sharp=true");
    let instance = RESIZE_NODE.from_kv(&mut kv).unwrap().unwrap();
    assert_eq!(instance.get_param("width"), Some(ParamValue::U32(800)));
    assert_eq!(instance.get_param("height"), Some(ParamValue::U32(600)));
    assert_eq!(
        instance.get_param("sharp_yuv"),
        Some(ParamValue::Bool(true))
    );
    assert_eq!(kv.unconsumed().count(), 0);
}

#[test]
fn downcast() {
    let instance = EXPOSURE_NODE.create_default().unwrap();
    let exp = instance.as_any().downcast_ref::<Exposure>().unwrap();
    assert_eq!(exp.stops, 0.0);
}

#[test]
fn clone_boxed() {
    let instance = EXPOSURE_NODE.create_default().unwrap();
    let cloned = instance.clone_boxed();
    assert_eq!(cloned.get_param("stops"), instance.get_param("stops"));
}

#[test]
fn registry_basic() {
    let mut registry = NodeRegistry::new();
    registry.register(&EXPOSURE_NODE);
    registry.register(&RESIZE_NODE);

    assert!(registry.get("test.exposure").is_some());
    assert!(registry.get("test.resize").is_some());
    assert!(registry.get("test.nonexistent").is_none());
    assert_eq!(registry.all().len(), 2);
}

#[test]
fn registry_from_querystring() {
    let mut registry = NodeRegistry::new();
    registry.register(&EXPOSURE_NODE);
    registry.register(&RESIZE_NODE);

    let result = registry.from_querystring("exposure=1.5&w=800&h=600");
    assert_eq!(result.instances.len(), 2);
    // Exposure should have stops=1.5
    let exp = &result.instances[0];
    assert_eq!(exp.schema().id, "test.exposure");
    assert_eq!(exp.get_param("stops"), Some(ParamValue::F32(1.5)));
    // Resize should have width=800, height=600
    let rsz = &result.instances[1];
    assert_eq!(rsz.schema().id, "test.resize");
    assert_eq!(rsz.get_param("width"), Some(ParamValue::U32(800)));
}

#[test]
fn registry_warns_unrecognized_keys() {
    let mut registry = NodeRegistry::new();
    registry.register(&EXPOSURE_NODE);

    let result = registry.from_querystring("exposure=1.5&unknown=foo");
    assert_eq!(result.instances.len(), 1);
    assert!(
        result
            .warnings
            .iter()
            .any(|w| w.kind == KvWarningKind::UnrecognizedKey && w.key == "unknown")
    );
}

// ─── Float array support ───

/// HSL-style per-hue adjustment with [f32; 8] array fields.
#[derive(Node, Clone, Debug)]
#[node(id = "test.hsl_adjust", group = Color, role = Filter)]
#[node(format(preferred = OklabF32, alpha = Skip))]
pub struct HslAdjust {
    /// Per-hue hue shift
    #[param(range(-180.0..=180.0), default = 0.0, identity = 0.0, step = 1.0)]
    #[param(unit = "\u{00b0}", section = "Hue", slider = NotSlider)]
    #[param(labels(
        "Red", "Orange", "Yellow", "Green", "Cyan", "Blue", "Purple", "Magenta"
    ))]
    pub hue: [f32; 8],

    /// Per-hue saturation multiplier
    #[param(range(0.0..=3.0), default = 1.0, identity = 1.0, step = 0.05)]
    #[param(unit = "\u{00d7}", section = "Saturation", slider = NotSlider)]
    #[param(labels(
        "Red", "Orange", "Yellow", "Green", "Cyan", "Blue", "Purple", "Magenta"
    ))]
    pub saturation: [f32; 8],

    /// Per-hue luminance offset
    #[param(range(-0.5..=0.5), default = 0.0, identity = 0.0, step = 0.01)]
    #[param(section = "Luminance", slider = NotSlider)]
    #[param(labels(
        "Red", "Orange", "Yellow", "Green", "Cyan", "Blue", "Purple", "Magenta"
    ))]
    pub luminance: [f32; 8],
}

/// B&W mixer weights.
#[derive(Node, Clone, Debug)]
#[node(id = "test.bw_mixer", group = Color, role = Filter)]
#[node(format(preferred = OklabF32, alpha = Skip))]
pub struct BwMixer {
    /// Per-hue grayscale contribution weights
    #[param(range(0.0..=2.0), default = 1.0, identity = 1.0, step = 0.05)]
    #[param(unit = "\u{00d7}", section = "Main", slider = NotSlider)]
    #[param(labels(
        "Red", "Orange", "Yellow", "Green", "Cyan", "Blue", "Purple", "Magenta"
    ))]
    pub weights: [f32; 8],
}

/// Minimal array node for targeted tests.
#[derive(Node, Clone, Debug)]
#[node(id = "test.array_node", group = Color, role = Filter)]
pub struct ArrayNode {
    /// Test array field
    #[param(range(-180.0..=180.0), default = 0.0, identity = 0.0)]
    #[param(labels("R", "O", "Y", "G", "C", "B", "P", "M"))]
    pub hue: [f32; 8],
}

#[test]
fn array_schema_metadata() {
    let schema = ARRAY_NODE_NODE.schema();
    assert_eq!(schema.id, "test.array_node");
    assert_eq!(schema.params.len(), 1);
    let p = &schema.params[0];
    assert_eq!(p.name, "hue");
    assert_eq!(p.label, "Hue");
    match &p.kind {
        ParamKind::FloatArray {
            len,
            min,
            max,
            default,
            labels,
        } => {
            assert_eq!(*len, 8);
            assert_eq!(*min, -180.0);
            assert_eq!(*max, 180.0);
            assert_eq!(*default, 0.0);
            assert_eq!(*labels, &["R", "O", "Y", "G", "C", "B", "P", "M"]);
        }
        other => panic!("expected FloatArray, got {:?}", other),
    }
}

#[test]
fn array_default_values() {
    let instance = ARRAY_NODE_NODE.create_default().unwrap();
    let val = instance.get_param("hue").unwrap();
    match &val {
        ParamValue::F32Array(arr) => {
            assert_eq!(arr.len(), 8);
            assert!(arr.iter().all(|v| *v == 0.0));
        }
        other => panic!("expected F32Array, got {:?}", other),
    }
    assert!(instance.is_identity());
}

#[test]
fn array_get_param() {
    let node = ArrayNode {
        hue: [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0],
    };
    let val = node.get_param("hue").unwrap();
    assert_eq!(
        val,
        ParamValue::F32Array(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0])
    );
}

#[test]
fn array_set_param() {
    let mut node = ArrayNode { hue: [0.0; 8] };
    let new_values = vec![10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0];
    assert!(node.set_param("hue", ParamValue::F32Array(new_values.clone())));
    assert_eq!(node.hue, [10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0]);

    // Verify round-trip through get_param
    assert_eq!(
        node.get_param("hue"),
        Some(ParamValue::F32Array(new_values))
    );
}

#[test]
fn array_set_param_wrong_length_rejected() {
    let mut node = ArrayNode { hue: [0.0; 8] };

    // Too few elements
    assert!(!node.set_param("hue", ParamValue::F32Array(vec![1.0, 2.0, 3.0])));
    // All still zeros (unchanged)
    assert_eq!(node.hue, [0.0; 8]);

    // Too many elements
    assert!(!node.set_param("hue", ParamValue::F32Array(vec![1.0; 10])));
    assert_eq!(node.hue, [0.0; 8]);

    // Wrong type entirely
    assert!(!node.set_param("hue", ParamValue::F32(42.0)));
    assert_eq!(node.hue, [0.0; 8]);
}

#[test]
fn array_is_identity() {
    let node = ArrayNode { hue: [0.0; 8] };
    assert!(node.is_identity());

    let node2 = ArrayNode {
        hue: [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0],
    };
    assert!(!node2.is_identity());
}

#[test]
fn array_to_params_round_trip() {
    let original = ArrayNode {
        hue: [1.0, -2.0, 3.0, -4.0, 5.0, -6.0, 7.0, -8.0],
    };
    let params = original.to_params();
    let instance = ARRAY_NODE_NODE.create(&params).unwrap();
    let restored = instance.as_any().downcast_ref::<ArrayNode>().unwrap();
    assert_eq!(original.hue, restored.hue);
}

#[test]
fn array_create_with_params() {
    let mut params = ParamMap::new();
    params.insert(
        "hue".into(),
        ParamValue::F32Array(vec![10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0]),
    );
    let instance = ARRAY_NODE_NODE.create(&params).unwrap();
    let node = instance.as_any().downcast_ref::<ArrayNode>().unwrap();
    assert_eq!(node.hue, [10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0]);
    assert!(!instance.is_identity());
}

#[test]
fn hsl_adjust_schema() {
    let schema = HSL_ADJUST_NODE.schema();
    assert_eq!(schema.id, "test.hsl_adjust");
    assert_eq!(schema.params.len(), 3);
    assert_eq!(schema.params[0].name, "hue");
    assert_eq!(schema.params[0].section, "Hue");
    assert_eq!(schema.params[1].name, "saturation");
    assert_eq!(schema.params[1].section, "Saturation");
    assert_eq!(schema.params[2].name, "luminance");
    assert_eq!(schema.params[2].section, "Luminance");

    // All three should be FloatArray with len 8
    for p in schema.params {
        match &p.kind {
            ParamKind::FloatArray { len, labels, .. } => {
                assert_eq!(*len, 8);
                assert_eq!(labels.len(), 8);
                assert_eq!(labels[0], "Red");
                assert_eq!(labels[7], "Magenta");
            }
            other => panic!("expected FloatArray for {}, got {:?}", p.name, other),
        }
    }
}

#[test]
fn hsl_adjust_identity() {
    let node = HslAdjust {
        hue: [0.0; 8],
        saturation: [1.0; 8],
        luminance: [0.0; 8],
    };
    assert!(node.is_identity());

    let mut node2 = node.clone();
    node2.hue[3] = 10.0;
    assert!(!node2.is_identity());
}

#[test]
fn bw_mixer_schema() {
    let schema = BW_MIXER_NODE.schema();
    assert_eq!(schema.id, "test.bw_mixer");
    assert_eq!(schema.params.len(), 1);
    match &schema.params[0].kind {
        ParamKind::FloatArray {
            len,
            min,
            max,
            default,
            labels,
        } => {
            assert_eq!(*len, 8);
            assert_eq!(*min, 0.0);
            assert_eq!(*max, 2.0);
            assert_eq!(*default, 1.0);
            assert_eq!(labels[0], "Red");
        }
        other => panic!("expected FloatArray, got {:?}", other),
    }
}

#[test]
fn bw_mixer_identity() {
    let node = BwMixer { weights: [1.0; 8] };
    assert!(node.is_identity());

    let node2 = BwMixer {
        weights: [1.0, 1.0, 0.5, 1.0, 1.0, 1.0, 1.0, 1.0],
    };
    assert!(!node2.is_identity());
}

// ─── Option<T> tests ───

/// Constraint with optional fields — models imageflow-style ergonomics.
#[derive(Node, Clone, Debug, Default)]
#[node(id = "test.constrain_opt", group = Layout, role = Geometry)]
#[node(changes_dimensions)]
pub struct ConstrainOpt {
    /// Target width. None = unconstrained.
    #[param(range(0..=65535), default = 0, step = 1)]
    #[param(unit = "px", section = "Main")]
    #[kv("w")]
    pub w: Option<u32>,

    /// Target height. None = unconstrained.
    #[param(range(0..=65535), default = 0, step = 1)]
    #[param(unit = "px", section = "Main")]
    #[kv("h")]
    pub h: Option<u32>,

    /// Resampling filter. None = auto-select.
    #[param(default = "lanczos")]
    #[param(section = "Hints")]
    #[kv("filter")]
    pub filter: Option<String>,

    /// Sharpening percentage. None = no sharpening.
    #[param(range(0.0..=100.0), default = 0.0, step = 0.1)]
    #[param(unit = "%", section = "Hints")]
    #[kv("sharpen")]
    pub sharpen: Option<f32>,

    /// Whether to process in linear light. None = auto.
    #[param(default = true)]
    #[param(section = "Hints")]
    pub linear: Option<bool>,
}

#[test]
fn optional_schema_marks_optional() {
    let schema = CONSTRAIN_OPT_NODE.schema();
    assert_eq!(schema.id, "test.constrain_opt");
    assert_eq!(schema.params.len(), 5);
    for p in schema.params {
        assert!(p.optional, "param {} should be optional", p.name);
    }
}

#[test]
fn optional_defaults_are_none() {
    let node = CONSTRAIN_OPT_NODE.create_default().unwrap();
    assert_eq!(node.get_param("w"), Some(ParamValue::None));
    assert_eq!(node.get_param("h"), Some(ParamValue::None));
    assert_eq!(node.get_param("filter"), Some(ParamValue::None));
    assert_eq!(node.get_param("sharpen"), Some(ParamValue::None));
    assert_eq!(node.get_param("linear"), Some(ParamValue::None));
}

#[test]
fn optional_set_and_get() {
    let mut params = ParamMap::new();
    params.insert("w".into(), ParamValue::U32(800));
    params.insert("filter".into(), ParamValue::Str("lanczos".into()));
    params.insert("sharpen".into(), ParamValue::F32(15.0));
    // h and linear left as None

    let node = CONSTRAIN_OPT_NODE.create(&params).unwrap();
    assert_eq!(node.get_param("w"), Some(ParamValue::U32(800)));
    assert_eq!(node.get_param("h"), Some(ParamValue::None));
    assert_eq!(
        node.get_param("filter"),
        Some(ParamValue::Str("lanczos".into()))
    );
    assert_eq!(node.get_param("sharpen"), Some(ParamValue::F32(15.0)));
    assert_eq!(node.get_param("linear"), Some(ParamValue::None));
}

#[test]
fn optional_set_then_clear_with_none() {
    let mut node = ConstrainOpt {
        w: Some(800),
        h: Some(600),
        filter: Some("lanczos".into()),
        sharpen: Some(15.0),
        linear: Some(true),
    };

    // Clear w with ParamValue::None
    assert!(node.set_param("w", ParamValue::None));
    assert_eq!(node.w, None);
    assert_eq!(node.get_param("w"), Some(ParamValue::None));

    // Clear filter
    assert!(node.set_param("filter", ParamValue::None));
    assert_eq!(node.filter, None);

    // h should still be set
    assert_eq!(node.h, Some(600));
}

#[test]
fn optional_downcast() {
    let mut params = ParamMap::new();
    params.insert("w".into(), ParamValue::U32(1920));
    params.insert("sharpen".into(), ParamValue::F32(10.0));

    let node = CONSTRAIN_OPT_NODE.create(&params).unwrap();
    let c = node.as_any().downcast_ref::<ConstrainOpt>().unwrap();
    assert_eq!(c.w, Some(1920));
    assert_eq!(c.h, None);
    assert_eq!(c.filter, None);
    assert_eq!(c.sharpen, Some(10.0));
    assert_eq!(c.linear, None);
}

#[test]
fn optional_to_params_round_trip() {
    let original = ConstrainOpt {
        w: Some(800),
        h: None,
        filter: Some("ginseng".into()),
        sharpen: None,
        linear: Some(false),
    };
    let params = original.to_params();

    // Verify the ParamMap
    assert_eq!(params.get("w"), Some(&ParamValue::U32(800)));
    assert_eq!(params.get("h"), Some(&ParamValue::None));
    assert_eq!(
        params.get("filter"),
        Some(&ParamValue::Str("ginseng".into()))
    );
    assert_eq!(params.get("sharpen"), Some(&ParamValue::None));
    assert_eq!(params.get("linear"), Some(&ParamValue::Bool(false)));

    // Round-trip through create
    let node = CONSTRAIN_OPT_NODE.create(&params).unwrap();
    let restored = node.as_any().downcast_ref::<ConstrainOpt>().unwrap();
    assert_eq!(restored.w, Some(800));
    assert_eq!(restored.h, None);
    assert_eq!(restored.filter.as_deref(), Some("ginseng"));
    assert_eq!(restored.sharpen, None);
    assert_eq!(restored.linear, Some(false));
}

#[test]
fn optional_from_kv() {
    let mut kv = KvPairs::from_querystring("w=1024&sharpen=20.0");
    let node = CONSTRAIN_OPT_NODE.from_kv(&mut kv).unwrap().unwrap();
    assert_eq!(node.get_param("w"), Some(ParamValue::U32(1024)));
    assert_eq!(node.get_param("h"), Some(ParamValue::None));
    assert_eq!(node.get_param("sharpen"), Some(ParamValue::F32(20.0)));
    assert_eq!(node.get_param("filter"), Some(ParamValue::None));
    assert_eq!(kv.unconsumed().count(), 0);
}

#[test]
fn optional_from_kv_no_match() {
    let mut kv = KvPairs::from_querystring("quality=85");
    let result = CONSTRAIN_OPT_NODE.from_kv(&mut kv).unwrap();
    assert!(result.is_none());
}

#[test]
fn non_optional_params_not_marked_optional() {
    let schema = EXPOSURE_NODE.schema();
    for p in schema.params {
        assert!(!p.optional, "param {} should not be optional", p.name);
    }
}

// ─── Json param tests ───

/// A nested struct used as a Json param (like imageflow's ResampleHints).
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, Default, PartialEq)]
pub struct Hints {
    pub down_filter: Option<String>,
    pub up_filter: Option<String>,
    pub sharpen_percent: Option<f32>,
}

/// A tagged union (like imageflow's ConstraintGravity).
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Gravity {
    Center,
    Percentage { x: f32, y: f32 },
}

impl Default for Gravity {
    fn default() -> Self {
        Self::Center
    }
}

/// Node with Json params for testing nested/complex types.
#[derive(Node, Clone, Debug, Default)]
#[node(id = "test.json_node", group = Geometry, role = Geometry)]
pub struct JsonNode {
    /// Target width.
    #[param(range(0..=65535), default = 0)]
    #[kv("w")]
    pub w: Option<u32>,

    /// Resample hints (nested object).
    #[param(
        json_schema = r#"{"type":"object","properties":{"down_filter":{"type":"string"},"up_filter":{"type":"string"},"sharpen_percent":{"type":"number"}}}"#
    )]
    pub hints: Option<Hints>,

    /// Gravity (tagged union).
    #[param(
        json_schema = r#"{"oneOf":[{"const":"center"},{"type":"object","properties":{"percentage":{"type":"object","properties":{"x":{"type":"number"},"y":{"type":"number"}},"required":["x","y"]}}}]}"#
    )]
    pub gravity: Option<Gravity>,
}

#[test]
fn json_param_schema() {
    let schema = JSON_NODE_NODE.schema();
    assert_eq!(schema.id, "test.json_node");
    assert_eq!(schema.params.len(), 3);

    // w is a normal optional u32
    assert!(matches!(schema.params[0].kind, ParamKind::U32 { .. }));
    assert!(schema.params[0].optional);

    // hints is a Json param
    assert!(matches!(schema.params[1].kind, ParamKind::Json { .. }));
    assert!(schema.params[1].optional);

    // gravity is a Json param
    assert!(matches!(schema.params[2].kind, ParamKind::Json { .. }));
    assert!(schema.params[2].optional);
}

#[test]
fn json_param_defaults_are_none() {
    let node = JSON_NODE_NODE.create_default().unwrap();
    assert_eq!(node.get_param("w"), Some(ParamValue::None));
    assert_eq!(node.get_param("hints"), Some(ParamValue::None));
    assert_eq!(node.get_param("gravity"), Some(ParamValue::None));
}

#[test]
fn json_param_round_trip() {
    let original = JsonNode {
        w: Some(800),
        hints: Some(Hints {
            down_filter: Some("lanczos".into()),
            up_filter: None,
            sharpen_percent: Some(15.0),
        }),
        gravity: Some(Gravity::Percentage { x: 0.33, y: 0.0 }),
    };

    let params = original.to_params();

    // w is normal
    assert_eq!(params.get("w"), Some(&ParamValue::U32(800)));

    // hints is JSON text
    let hints_json = params.get("hints").unwrap().as_json_str().unwrap();
    let hints_parsed: Hints = serde_json::from_str(hints_json).unwrap();
    assert_eq!(hints_parsed.down_filter.as_deref(), Some("lanczos"));
    assert_eq!(hints_parsed.sharpen_percent, Some(15.0));

    // gravity is JSON text
    let gravity_json = params.get("gravity").unwrap().as_json_str().unwrap();
    let gravity_parsed: Gravity = serde_json::from_str(gravity_json).unwrap();
    assert_eq!(gravity_parsed, Gravity::Percentage { x: 0.33, y: 0.0 });

    // Round-trip through create
    let node = JSON_NODE_NODE.create(&params).unwrap();
    let restored = node.as_any().downcast_ref::<JsonNode>().unwrap();
    assert_eq!(restored.w, Some(800));
    assert_eq!(
        restored.hints.as_ref().unwrap().down_filter.as_deref(),
        Some("lanczos")
    );
    assert_eq!(
        restored.gravity,
        Some(Gravity::Percentage { x: 0.33, y: 0.0 })
    );
}

#[test]
fn json_param_set_and_clear() {
    let mut node = JsonNode::default();
    let boxed: &mut dyn NodeInstance = &mut node;

    // Set hints via JSON text
    let hints_json = r#"{"down_filter":"ginseng","sharpen_percent":10.0}"#;
    assert!(boxed.set_param("hints", ParamValue::Json(hints_json.into())));
    assert!(node.hints.is_some());
    assert_eq!(
        node.hints.as_ref().unwrap().down_filter.as_deref(),
        Some("ginseng")
    );

    // Clear with None
    let boxed: &mut dyn NodeInstance = &mut node;
    assert!(boxed.set_param("hints", ParamValue::None));
    assert_eq!(node.hints, None);
}

#[test]
fn json_param_downcast() {
    let mut params = ParamMap::new();
    params.insert("w".into(), ParamValue::U32(1920));
    params.insert(
        "gravity".into(),
        ParamValue::Json(r#"{"percentage":{"x":0.5,"y":0.0}}"#.into()),
    );

    let node = JSON_NODE_NODE.create(&params).unwrap();
    let n = node.as_any().downcast_ref::<JsonNode>().unwrap();
    assert_eq!(n.w, Some(1920));
    assert_eq!(n.gravity, Some(Gravity::Percentage { x: 0.5, y: 0.0 }));
    assert_eq!(n.hints, None);
}

#[test]
fn json_param_kv_skips_json_fields() {
    // JSON params don't parse from querystrings — only w matches
    let mut kv = KvPairs::from_querystring("w=400");
    let node = JSON_NODE_NODE.from_kv(&mut kv).unwrap().unwrap();
    assert_eq!(node.get_param("w"), Some(ParamValue::U32(400)));
    assert_eq!(node.get_param("hints"), Some(ParamValue::None));
    assert_eq!(node.get_param("gravity"), Some(ParamValue::None));
}

// ─── Whole-node serde tests ───

/// Node with json_key, json_name, json_alias, deny_unknown_fields.
#[derive(Node, Clone, Debug, Default)]
#[node(id = "test.imageflow_constrain", group = Layout, role = Resize)]
#[node(json_key = "constrain", deny_unknown_fields)]
#[node(changes_dimensions)]
pub struct ImageflowConstrain {
    /// Target width.
    #[param(range(0..=65535), default = 0)]
    pub w: Option<u32>,

    /// Target height.
    #[param(range(0..=65535), default = 0)]
    pub h: Option<u32>,

    /// Constraint mode.
    #[param(default = "within")]
    pub mode: String,

    /// Sharpening (renamed for imageflow compat).
    #[param(range(0.0..=100.0), default = 0.0, step = 1.0)]
    #[param(json_name = "sharpen_percent")]
    #[param(json_alias = "sharpen_pct")]
    pub sharpen: Option<f32>,

    /// Resample hints (nested object).
    #[param(
        json_schema = r#"{"type":"object","properties":{"down_filter":{"type":"string"},"up_filter":{"type":"string"}}}"#
    )]
    pub hints: Option<Hints>,
}

#[test]
fn json_key_in_schema() {
    let schema = IMAGEFLOW_CONSTRAIN_NODE.schema();
    assert_eq!(schema.json_key, "constrain");
    assert_eq!(schema.effective_json_key(), "constrain");
    assert!(schema.deny_unknown_fields);
}

#[test]
fn json_name_in_param_desc() {
    let schema = IMAGEFLOW_CONSTRAIN_NODE.schema();
    let sharpen = schema.params.iter().find(|p| p.name == "sharpen").unwrap();
    assert_eq!(sharpen.json_name, "sharpen_percent");
    assert_eq!(sharpen.effective_json_name(), "sharpen_percent");
    assert!(sharpen.json_aliases.contains(&"sharpen_pct"));
    assert!(sharpen.matches_json_key("sharpen_percent"));
    assert!(sharpen.matches_json_key("sharpen_pct"));
    assert!(sharpen.matches_json_key("sharpen")); // field name also works
    assert!(!sharpen.matches_json_key("sharpness"));
}

#[test]
fn json_key_empty_defaults_to_id() {
    let schema = EXPOSURE_NODE.schema();
    assert_eq!(schema.json_key, "");
    assert_eq!(schema.effective_json_key(), "test.exposure");
}

#[test]
fn whole_node_from_json() {
    let mut registry = NodeRegistry::new();
    registry.register(&IMAGEFLOW_CONSTRAIN_NODE);

    let json: serde_json::Value = serde_json::json!({
        "constrain": {
            "mode": "fit_crop",
            "w": 800,
            "h": 600,
            "sharpen_percent": 15.0
        }
    });

    let node = registry.node_from_json(&json).unwrap();
    assert_eq!(node.schema().id, "test.imageflow_constrain");
    assert_eq!(node.get_param("w"), Some(ParamValue::U32(800)));
    assert_eq!(node.get_param("h"), Some(ParamValue::U32(600)));
    assert_eq!(
        node.get_param("mode"),
        Some(ParamValue::Str("fit_crop".into()))
    );
    assert_eq!(node.get_param("sharpen"), Some(ParamValue::F32(15.0)));
}

#[test]
fn whole_node_from_json_with_alias() {
    let mut registry = NodeRegistry::new();
    registry.register(&IMAGEFLOW_CONSTRAIN_NODE);

    // Use the alias "sharpen_pct" instead of "sharpen_percent"
    let json: serde_json::Value = serde_json::json!({
        "constrain": {
            "w": 400,
            "sharpen_pct": 20.0
        }
    });

    let node = registry.node_from_json(&json).unwrap();
    assert_eq!(node.get_param("sharpen"), Some(ParamValue::F32(20.0)));
}

#[test]
fn whole_node_from_json_with_nested_json_param() {
    let mut registry = NodeRegistry::new();
    registry.register(&IMAGEFLOW_CONSTRAIN_NODE);

    let json: serde_json::Value = serde_json::json!({
        "constrain": {
            "w": 800,
            "hints": {
                "down_filter": "lanczos",
                "up_filter": "ginseng"
            }
        }
    });

    let node = registry.node_from_json(&json).unwrap();
    let c = node.as_any().downcast_ref::<ImageflowConstrain>().unwrap();
    assert_eq!(c.w, Some(800));
    let hints = c.hints.as_ref().unwrap();
    assert_eq!(hints.down_filter.as_deref(), Some("lanczos"));
    assert_eq!(hints.up_filter.as_deref(), Some("ginseng"));
}

#[test]
fn whole_node_to_json_skips_none() {
    let mut registry = NodeRegistry::new();
    registry.register(&IMAGEFLOW_CONSTRAIN_NODE);

    let node = ImageflowConstrain {
        w: Some(800),
        h: None,
        mode: String::from("fit"),
        sharpen: None,
        hints: None,
    };

    let json = registry.node_to_json(&node);
    let inner = json.get("constrain").unwrap();

    // w is present
    assert_eq!(inner.get("w").unwrap(), 800);
    // mode is present (non-optional, always serialized)
    assert_eq!(inner.get("mode").unwrap(), "fit");
    // h, sharpen_percent, hints are absent (None → skipped)
    assert!(inner.get("h").is_none());
    assert!(inner.get("sharpen_percent").is_none());
    assert!(inner.get("hints").is_none());
}

#[test]
fn whole_node_to_json_uses_json_name() {
    let mut registry = NodeRegistry::new();
    registry.register(&IMAGEFLOW_CONSTRAIN_NODE);

    let node = ImageflowConstrain {
        w: Some(800),
        h: None,
        mode: String::from("fit"),
        sharpen: Some(15.0),
        hints: None,
    };

    let json = registry.node_to_json(&node);
    let inner = json.get("constrain").unwrap();

    // The JSON key should be "sharpen_percent" (json_name), not "sharpen" (field name)
    assert!(inner.get("sharpen_percent").is_some());
    assert!(inner.get("sharpen").is_none());
    assert_eq!(inner.get("sharpen_percent").unwrap(), 15.0);
}

#[test]
fn whole_node_to_json_embeds_nested_json() {
    let mut registry = NodeRegistry::new();
    registry.register(&IMAGEFLOW_CONSTRAIN_NODE);

    let node = ImageflowConstrain {
        w: Some(800),
        h: None,
        mode: String::from("fit"),
        sharpen: None,
        hints: Some(Hints {
            down_filter: Some("lanczos".into()),
            up_filter: None,
            sharpen_percent: Some(10.0),
        }),
    };

    let json = registry.node_to_json(&node);
    let hints = json.get("constrain").unwrap().get("hints").unwrap();
    assert_eq!(hints.get("down_filter").unwrap(), "lanczos");
    assert_eq!(hints.get("sharpen_percent").unwrap(), 10.0);
}

#[test]
fn deny_unknown_fields_rejects_unknown() {
    let mut registry = NodeRegistry::new();
    registry.register(&IMAGEFLOW_CONSTRAIN_NODE);

    let json: serde_json::Value = serde_json::json!({
        "constrain": {
            "w": 800,
            "unknown_field": "bad"
        }
    });

    let result = registry.node_from_json(&json);
    let err = result.err().expect("should be an error");
    let err_str = err.to_string();
    assert!(
        err_str.contains("unknown_field"),
        "error should mention the field: {err_str}"
    );
}

#[test]
fn without_deny_unknown_fields_ignores_unknown() {
    let mut registry = NodeRegistry::new();
    registry.register(&JSON_NODE_NODE); // JSON_NODE_NODE does NOT have deny_unknown_fields

    let json: serde_json::Value = serde_json::json!({
        "test.json_node": {
            "w": 400,
            "extra_field": "ignored"
        }
    });

    let node = registry.node_from_json(&json).unwrap();
    assert_eq!(node.get_param("w"), Some(ParamValue::U32(400)));
}

#[test]
fn whole_node_round_trip() {
    let mut registry = NodeRegistry::new();
    registry.register(&IMAGEFLOW_CONSTRAIN_NODE);

    let original = ImageflowConstrain {
        w: Some(1920),
        h: Some(1080),
        mode: String::from("fit_crop"),
        sharpen: Some(15.0),
        hints: Some(Hints {
            down_filter: Some("lanczos".into()),
            up_filter: Some("ginseng".into()),
            sharpen_percent: None,
        }),
    };

    // Serialize
    let json = registry.node_to_json(&original);

    // Deserialize
    let restored_boxed = registry.node_from_json(&json).unwrap();
    let restored = restored_boxed
        .as_any()
        .downcast_ref::<ImageflowConstrain>()
        .unwrap();

    assert_eq!(restored.w, Some(1920));
    assert_eq!(restored.h, Some(1080));
    assert_eq!(restored.mode, "fit_crop");
    assert_eq!(restored.sharpen, Some(15.0));
    assert_eq!(
        restored.hints.as_ref().unwrap().down_filter.as_deref(),
        Some("lanczos")
    );
}

#[test]
fn pipeline_from_json() {
    let mut registry = NodeRegistry::new();
    registry.register(&IMAGEFLOW_CONSTRAIN_NODE);
    registry.register(&EXPOSURE_NODE);

    let json: serde_json::Value = serde_json::json!([
        {"constrain": {"w": 800, "mode": "fit"}},
        {"test.exposure": {"stops": 1.5}}
    ]);

    let nodes = registry.pipeline_from_json(&json).unwrap();
    assert_eq!(nodes.len(), 2);
    assert_eq!(nodes[0].schema().id, "test.imageflow_constrain");
    assert_eq!(nodes[1].schema().id, "test.exposure");
    assert_eq!(nodes[0].get_param("w"), Some(ParamValue::U32(800)));
    assert_eq!(nodes[1].get_param("stops"), Some(ParamValue::F32(1.5)));
}

#[test]
fn pipeline_round_trip() {
    let mut registry = NodeRegistry::new();
    registry.register(&IMAGEFLOW_CONSTRAIN_NODE);
    registry.register(&EXPOSURE_NODE);

    let nodes: Vec<Box<dyn NodeInstance>> = vec![
        Box::new(ImageflowConstrain {
            w: Some(800),
            h: None,
            mode: String::from("fit"),
            sharpen: None,
            hints: None,
        }),
        Box::new(Exposure { stops: 1.5 }),
    ];

    let json = registry.pipeline_to_json(&nodes);
    let restored = registry.pipeline_from_json(&json).unwrap();

    assert_eq!(restored.len(), 2);
    assert_eq!(restored[0].get_param("w"), Some(ParamValue::U32(800)));
    assert_eq!(restored[1].get_param("stops"), Some(ParamValue::F32(1.5)));
}

#[test]
fn node_from_json_null_means_none() {
    let mut registry = NodeRegistry::new();
    registry.register(&IMAGEFLOW_CONSTRAIN_NODE);

    let json: serde_json::Value = serde_json::json!({
        "constrain": {
            "w": 800,
            "sharpen_percent": null
        }
    });

    let node = registry.node_from_json(&json).unwrap();
    assert_eq!(node.get_param("sharpen"), Some(ParamValue::None));
}

#[test]
fn node_from_json_missing_key_error() {
    let mut registry = NodeRegistry::new();
    registry.register(&IMAGEFLOW_CONSTRAIN_NODE);

    let json: serde_json::Value = serde_json::json!({
        "nonexistent": {"w": 800}
    });

    assert!(registry.node_from_json(&json).err().is_some());
}

// ─── Sub-struct and enum NodeParams tests ───

/// Sub-struct: derives Node WITHOUT #[node(id)] → only NodeParams generated.
#[derive(Node, Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SubHints {
    /// Downscale filter name.
    pub down_filter: Option<String>,

    /// Sharpening amount.
    #[param(range(0.0..=100.0), default = 0.0, step = 1.0)]
    #[param(unit = "%")]
    pub sharpen_percent: Option<f32>,
}

/// Tagged enum: derives Node → NodeParams with TaggedVariant descriptors.
#[derive(Node, Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubGravity {
    /// Center of the image.
    Center,
    /// Percentage-based position.
    Percentage {
        /// Horizontal position (0.0 = left, 1.0 = right).
        #[param(range(0.0..=1.0), default = 0.5, step = 0.01)]
        x: f32,
        /// Vertical position (0.0 = top, 1.0 = bottom).
        #[param(range(0.0..=1.0), default = 0.5, step = 0.01)]
        y: f32,
    },
}

impl Default for SubGravity {
    fn default() -> Self {
        Self::Center
    }
}

/// Full node using sub-struct and enum fields.
#[derive(Node, Clone, Debug, Default)]
#[node(id = "test.recursive_node", group = Layout, role = Resize)]
#[node(json_key = "recursive")]
pub struct RecursiveNode {
    pub w: Option<u32>,
    pub hints: Option<SubHints>,
    pub gravity: Option<SubGravity>,
}

#[test]
fn sub_struct_node_params() {
    // SubHints should implement NodeParams
    let kind = SubHints::PARAM_KIND;
    match &kind {
        ParamKind::Object { params } => {
            assert_eq!(params.len(), 2);
            assert_eq!(params[0].name, "down_filter");
            assert!(params[0].optional);
            assert_eq!(params[1].name, "sharpen_percent");
            assert!(params[1].optional);
            // Check range on sharpen_percent
            match &params[1].kind {
                ParamKind::Float { min, max, .. } => {
                    assert_eq!(*min, 0.0);
                    assert_eq!(*max, 100.0);
                }
                other => panic!("expected Float, got {:?}", other),
            }
        }
        other => panic!("expected Object, got {:?}", other),
    }
}

#[test]
fn enum_node_params() {
    // SubGravity should implement NodeParams with TaggedUnion
    let kind = SubGravity::PARAM_KIND;
    match &kind {
        ParamKind::TaggedUnion { variants } => {
            assert_eq!(variants.len(), 2);

            // Unit variant: center
            assert_eq!(variants[0].tag, "center");
            assert_eq!(variants[0].label, "Center");
            assert!(variants[0].params.is_empty());

            // Struct variant: percentage with x, y
            assert_eq!(variants[1].tag, "percentage");
            assert_eq!(variants[1].label, "Percentage");
            assert_eq!(variants[1].params.len(), 2);
            assert_eq!(variants[1].params[0].name, "x");
            assert_eq!(variants[1].params[1].name, "y");
            // Check range on x
            match &variants[1].params[0].kind {
                ParamKind::Float { min, max, .. } => {
                    assert_eq!(*min, 0.0);
                    assert_eq!(*max, 1.0);
                }
                other => panic!("expected Float for x, got {:?}", other),
            }
        }
        other => panic!("expected TaggedUnion, got {:?}", other),
    }
}

#[test]
fn recursive_node_schema_has_object_and_tagged_union_params() {
    let schema = RECURSIVE_NODE_NODE.schema();
    assert_eq!(schema.id, "test.recursive_node");
    assert_eq!(schema.params.len(), 3);

    // w: normal optional u32
    assert!(matches!(schema.params[0].kind, ParamKind::U32 { .. }));
    assert!(schema.params[0].optional);

    // hints: Object (sub-struct)
    match &schema.params[1].kind {
        ParamKind::Object { params } => {
            assert_eq!(params.len(), 2);
            assert_eq!(params[0].name, "down_filter");
        }
        other => panic!("expected Object for hints, got {:?}", other),
    }
    assert!(schema.params[1].optional);

    // gravity: TaggedUnion (enum)
    match &schema.params[2].kind {
        ParamKind::TaggedUnion { variants } => {
            assert_eq!(variants.len(), 2);
            assert_eq!(variants[0].tag, "center");
            assert_eq!(variants[1].tag, "percentage");
        }
        other => panic!("expected TaggedUnion for gravity, got {:?}", other),
    }
    assert!(schema.params[2].optional);
}

#[test]
fn recursive_node_json_round_trip() {
    let mut registry = NodeRegistry::new();
    registry.register(&RECURSIVE_NODE_NODE);

    let json: serde_json::Value = serde_json::json!({
        "recursive": {
            "w": 800,
            "hints": {"down_filter": "lanczos", "sharpen_percent": 15.0},
            "gravity": {"percentage": {"x": 0.33, "y": 0.0}}
        }
    });

    let node = registry.node_from_json(&json).unwrap();
    let r = node.as_any().downcast_ref::<RecursiveNode>().unwrap();
    assert_eq!(r.w, Some(800));

    // Check hints round-trip
    let hints = r.hints.as_ref().unwrap();
    assert_eq!(hints.down_filter.as_deref(), Some("lanczos"));
    assert_eq!(hints.sharpen_percent, Some(15.0));

    // Check gravity round-trip
    assert_eq!(r.gravity, Some(SubGravity::Percentage { x: 0.33, y: 0.0 }));

    // Serialize back and verify structure
    let out = registry.node_to_json(node.as_ref());
    let inner = out.get("recursive").unwrap();
    assert_eq!(inner.get("w").unwrap(), 800);
    // hints should be a nested object (not string-escaped)
    assert!(inner.get("hints").unwrap().is_object());
    assert_eq!(
        inner.get("hints").unwrap().get("down_filter").unwrap(),
        "lanczos"
    );
}
