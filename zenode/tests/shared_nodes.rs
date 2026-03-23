//! Tests for built-in shared nodes (Decode).
//!
//! QualityIntent tests have moved to zencodecs (zenode_defs module).

use zenode::nodes::*;
use zenode::*;

#[test]
fn decode_schema() {
    let schema = DECODE_NODE.schema();
    assert_eq!(schema.id, "zenode.decode");
    assert_eq!(schema.group, NodeGroup::Decode);
    assert_eq!(schema.role, NodeRole::Decode);
    let names: Vec<&str> = schema.params.iter().map(|p| p.name).collect();
    assert!(names.contains(&"io_id"));
    assert!(names.contains(&"hdr_mode"));
    assert!(names.contains(&"color_intent"));
    assert!(names.contains(&"min_size"));
}

#[test]
fn decode_create_default() {
    let node = DECODE_NODE.create_default().unwrap();
    assert_eq!(node.get_param("io_id"), Some(ParamValue::I32(0)));
    assert_eq!(
        node.get_param("hdr_mode"),
        Some(ParamValue::Str("sdr_only".into()))
    );
    assert_eq!(
        node.get_param("color_intent"),
        Some(ParamValue::Str("preserve".into()))
    );
    assert_eq!(node.get_param("min_size"), Some(ParamValue::U32(0)));
}

#[test]
fn decode_hdr_reconstruct() {
    let mut params = ParamMap::new();
    params.insert("hdr_mode".into(), ParamValue::Str("hdr_reconstruct".into()));
    let node = DECODE_NODE.create(&params).unwrap();
    assert_eq!(
        node.get_param("hdr_mode"),
        Some(ParamValue::Str("hdr_reconstruct".into()))
    );
}

#[test]
fn decode_color_intent_srgb() {
    let mut params = ParamMap::new();
    params.insert("color_intent".into(), ParamValue::Str("srgb".into()));
    let node = DECODE_NODE.create(&params).unwrap();
    assert_eq!(
        node.get_param("color_intent"),
        Some(ParamValue::Str("srgb".into()))
    );
}

#[test]
fn decode_min_size_hint() {
    let mut params = ParamMap::new();
    params.insert("min_size".into(), ParamValue::U32(800));
    let node = DECODE_NODE.create(&params).unwrap();
    assert_eq!(node.get_param("min_size"), Some(ParamValue::U32(800)));
}

#[test]
fn decode_from_kv_no_keys() {
    let mut kv = KvPairs::from_querystring("w=800");
    let result = DECODE_NODE.from_kv(&mut kv).unwrap();
    assert!(result.is_none());
}

#[test]
fn decode_downcast() {
    let node = DECODE_NODE.create_default().unwrap();
    let d = node.as_any().downcast_ref::<Decode>().unwrap();
    assert_eq!(d.hdr_mode, "sdr_only");
    assert_eq!(d.color_intent, "preserve");
    assert_eq!(d.min_size, 0);
}
