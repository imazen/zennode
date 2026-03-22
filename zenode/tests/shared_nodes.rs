//! Tests for built-in shared nodes (Decode, QualityIntent).

extern crate alloc;

use zenode::nodes::*;
use zenode::*;

#[test]
fn decode_schema() {
    let schema = DECODE_NODE.schema();
    assert_eq!(schema.id, "zenode.decode");
    assert_eq!(schema.group, NodeGroup::Decode);
    assert_eq!(schema.phase, Phase::Decode);
    assert_eq!(schema.params.len(), 1);
    assert_eq!(schema.params[0].name, "io_id");
}

#[test]
fn decode_create_default() {
    let node = DECODE_NODE.create_default().unwrap();
    assert_eq!(node.get_param("io_id"), Some(ParamValue::I32(0)));
}

#[test]
fn decode_from_kv_no_keys() {
    let mut kv = KvPairs::from_querystring("w=800");
    let result = DECODE_NODE.from_kv(&mut kv).unwrap();
    assert!(result.is_none());
}

#[test]
fn quality_intent_schema() {
    let schema = QUALITY_INTENT_NODE.schema();
    assert_eq!(schema.id, "zenode.quality_intent");
    assert_eq!(schema.group, NodeGroup::Encode);
    assert_eq!(schema.phase, Phase::Encode);
    assert!(schema.tags.contains(&"quality"));
    assert!(schema.tags.contains(&"auto"));

    let param_names: Vec<&str> = schema.params.iter().map(|p| p.name).collect();
    assert!(param_names.contains(&"profile"));
    assert!(param_names.contains(&"format"));
    assert!(param_names.contains(&"dpr"));
    assert!(param_names.contains(&"lossless"));
    assert!(param_names.contains(&"allow_jpeg"));
    assert!(param_names.contains(&"allow_png"));
    assert!(param_names.contains(&"allow_gif"));
    assert!(param_names.contains(&"allow_webp"));
    assert!(param_names.contains(&"allow_avif"));
    assert!(param_names.contains(&"allow_jxl"));
    assert!(param_names.contains(&"allow_color_profiles"));
}

#[test]
fn quality_intent_defaults_match_web_safe() {
    let node = QUALITY_INTENT_NODE.create_default().unwrap();
    assert_eq!(node.get_param("profile"), Some(ParamValue::Str("high".into())));
    assert_eq!(node.get_param("format"), Some(ParamValue::Str(String::new())));
    assert_eq!(node.get_param("dpr"), Some(ParamValue::F32(1.0)));
    assert_eq!(node.get_param("lossless"), Some(ParamValue::Bool(false)));
    // web_safe baseline: jpeg/png/gif on, webp/avif/jxl off
    assert_eq!(node.get_param("allow_jpeg"), Some(ParamValue::Bool(true)));
    assert_eq!(node.get_param("allow_png"), Some(ParamValue::Bool(true)));
    assert_eq!(node.get_param("allow_gif"), Some(ParamValue::Bool(true)));
    assert_eq!(node.get_param("allow_webp"), Some(ParamValue::Bool(false)));
    assert_eq!(node.get_param("allow_avif"), Some(ParamValue::Bool(false)));
    assert_eq!(node.get_param("allow_jxl"), Some(ParamValue::Bool(false)));
    assert_eq!(node.get_param("allow_color_profiles"), Some(ParamValue::Bool(false)));
}

#[test]
fn quality_intent_from_kv_qp_with_accepts() {
    let mut kv = KvPairs::from_querystring("qp=medium&accept.webp=true&accept.avif=true");
    let node = QUALITY_INTENT_NODE.from_kv(&mut kv).unwrap().unwrap();
    assert_eq!(node.get_param("profile"), Some(ParamValue::Str("medium".into())));
    assert_eq!(node.get_param("allow_webp"), Some(ParamValue::Bool(true)));
    assert_eq!(node.get_param("allow_avif"), Some(ParamValue::Bool(true)));
    assert_eq!(node.get_param("allow_jxl"), Some(ParamValue::Bool(false)));
    assert_eq!(kv.unconsumed().count(), 0);
}

#[test]
fn quality_intent_from_kv_format_explicit() {
    let mut kv = KvPairs::from_querystring("format=webp&qp=good");
    let node = QUALITY_INTENT_NODE.from_kv(&mut kv).unwrap().unwrap();
    assert_eq!(node.get_param("format"), Some(ParamValue::Str("webp".into())));
    assert_eq!(node.get_param("profile"), Some(ParamValue::Str("good".into())));
    assert_eq!(kv.unconsumed().count(), 0);
}

#[test]
fn quality_intent_from_kv_dpr() {
    let mut kv = KvPairs::from_querystring("qp=high&dpr=2.0");
    let node = QUALITY_INTENT_NODE.from_kv(&mut kv).unwrap().unwrap();
    assert_eq!(node.get_param("profile"), Some(ParamValue::Str("high".into())));
    assert_eq!(node.get_param("dpr"), Some(ParamValue::F32(2.0)));
}

#[test]
fn quality_intent_from_kv_lossless() {
    let mut kv = KvPairs::from_querystring("qp=highest&lossless=true");
    let node = QUALITY_INTENT_NODE.from_kv(&mut kv).unwrap().unwrap();
    assert_eq!(node.get_param("lossless"), Some(ParamValue::Bool(true)));
}

#[test]
fn quality_intent_from_kv_no_match() {
    let mut kv = KvPairs::from_querystring("w=800&h=600");
    let result = QUALITY_INTENT_NODE.from_kv(&mut kv).unwrap();
    assert!(result.is_none());
}

#[test]
fn quality_intent_from_kv_format_only() {
    // format= alone should trigger the node (common in imageflow)
    let mut kv = KvPairs::from_querystring("format=jpeg");
    let node = QUALITY_INTENT_NODE.from_kv(&mut kv).unwrap().unwrap();
    assert_eq!(node.get_param("format"), Some(ParamValue::Str("jpeg".into())));
    // profile stays at default since qp= wasn't specified
    assert_eq!(node.get_param("profile"), Some(ParamValue::Str("high".into())));
}

#[test]
fn quality_intent_json_round_trip() {
    // Simulate JSON API: create from ParamMap (how JSON deserialization works)
    let mut params = ParamMap::new();
    params.insert("profile".into(), ParamValue::Str("medium".into()));
    params.insert("format".into(), ParamValue::Str("".into()));
    params.insert("dpr".into(), ParamValue::F32(2.0));
    params.insert("lossless".into(), ParamValue::Bool(false));
    params.insert("allow_webp".into(), ParamValue::Bool(true));
    params.insert("allow_avif".into(), ParamValue::Bool(true));
    params.insert("allow_jxl".into(), ParamValue::Bool(false));
    params.insert("allow_jpeg".into(), ParamValue::Bool(true));
    params.insert("allow_png".into(), ParamValue::Bool(true));
    params.insert("allow_gif".into(), ParamValue::Bool(true));
    params.insert("allow_color_profiles".into(), ParamValue::Bool(false));

    let node = QUALITY_INTENT_NODE.create(&params).unwrap();
    assert_eq!(node.get_param("profile"), Some(ParamValue::Str("medium".into())));
    assert_eq!(node.get_param("dpr"), Some(ParamValue::F32(2.0)));
    assert_eq!(node.get_param("allow_webp"), Some(ParamValue::Bool(true)));

    // Round-trip: to_params → create → verify
    let exported = node.to_params();
    let node2 = QUALITY_INTENT_NODE.create(&exported).unwrap();
    assert_eq!(node2.get_param("profile"), Some(ParamValue::Str("medium".into())));
    assert_eq!(node2.get_param("dpr"), Some(ParamValue::F32(2.0)));
    assert_eq!(node2.get_param("allow_webp"), Some(ParamValue::Bool(true)));
}

#[test]
fn quality_intent_kv_keys_coverage() {
    let schema = QUALITY_INTENT_NODE.schema();

    let profile_param = schema.params.iter().find(|p| p.name == "profile").unwrap();
    assert_eq!(profile_param.kv_keys, &["qp"]);

    let format_param = schema.params.iter().find(|p| p.name == "format").unwrap();
    assert_eq!(format_param.kv_keys, &["format"]);

    let dpr_param = schema.params.iter().find(|p| p.name == "dpr").unwrap();
    assert!(dpr_param.kv_keys.contains(&"qp.dpr"));
    assert!(dpr_param.kv_keys.contains(&"dpr"));
    assert!(dpr_param.kv_keys.contains(&"dppx"));
}

#[test]
fn registry_riapi_querystring() {
    let mut registry = NodeRegistry::new();
    registry.register(&DECODE_NODE);
    registry.register(&QUALITY_INTENT_NODE);

    let result = registry.from_querystring("qp=good&accept.webp=true&accept.jxl=true");
    assert_eq!(result.instances.len(), 1);
    let qi = &result.instances[0];
    assert_eq!(qi.schema().id, "zenode.quality_intent");
    assert_eq!(qi.get_param("profile"), Some(ParamValue::Str("good".into())));
    assert_eq!(qi.get_param("allow_webp"), Some(ParamValue::Bool(true)));
    assert_eq!(qi.get_param("allow_jxl"), Some(ParamValue::Bool(true)));
}

#[test]
fn quality_intent_downcast() {
    let node = QUALITY_INTENT_NODE.create_default().unwrap();
    let qi = node.as_any().downcast_ref::<QualityIntent>().unwrap();
    assert_eq!(qi.profile, "high");
    assert!(qi.allow_jpeg);
    assert!(!qi.allow_webp);
}
