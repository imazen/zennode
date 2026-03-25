//! Integration tests for `#[derive(NodeEnum)]`.

extern crate alloc;

use core::str::FromStr;

use zennode::*;

/// Edge detection algorithm.
#[derive(NodeEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum EdgeMode {
    /// Sobel operator for directional gradient detection.
    Sobel,
    /// Laplacian for isotropic second-derivative detection.
    Laplacian,
    /// Canny edge detector with non-maximum suppression.
    #[variant(label = "Canny (Best)")]
    Canny,
    /// Roberts cross operator.
    #[variant(alias = "roberts_cross")]
    Roberts,
}

#[test]
fn enum_variants_metadata() {
    let variants = EdgeMode::zennode_variants();
    assert_eq!(variants.len(), 4);
    assert_eq!(variants[0].name, "sobel");
    assert_eq!(variants[0].label, "Sobel");
    assert_eq!(variants[1].name, "laplacian");
    assert_eq!(variants[2].name, "canny");
    assert_eq!(variants[2].label, "Canny (Best)");
    assert_eq!(variants[3].name, "roberts");
}

#[test]
fn enum_display() {
    assert_eq!(EdgeMode::Sobel.to_string(), "sobel");
    assert_eq!(EdgeMode::Canny.to_string(), "canny");
}

#[test]
fn enum_from_str() {
    assert_eq!(EdgeMode::from_str("sobel").unwrap(), EdgeMode::Sobel);
    assert_eq!(EdgeMode::from_str("canny").unwrap(), EdgeMode::Canny);
    assert_eq!(
        EdgeMode::from_str("roberts_cross").unwrap(),
        EdgeMode::Roberts
    );
    assert!(EdgeMode::from_str("nonexistent").is_err());
}

#[test]
fn enum_zennode_name() {
    assert_eq!(EdgeMode::Sobel.zennode_name(), "sobel");
    assert_eq!(EdgeMode::Laplacian.zennode_name(), "laplacian");
    assert_eq!(EdgeMode::Canny.zennode_name(), "canny");
    assert_eq!(EdgeMode::Roberts.zennode_name(), "roberts");
}

#[test]
fn enum_doc_comments() {
    let variants = EdgeMode::zennode_variants();
    assert!(variants[0].description.contains("Sobel operator"));
    assert!(variants[2].description.contains("Canny edge detector"));
}
