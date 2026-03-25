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
