//! Error types for node operations.

use alloc::string::String;
use core::fmt;

/// Errors from node creation, parameter access, and registry operations.
#[non_exhaustive]
#[derive(Clone, Debug)]
pub enum NodeError {
    /// Unknown node ID.
    UnknownNode(String),
    /// Unknown parameter name.
    UnknownParam { node: &'static str, param: String },
    /// Parameter value type mismatch.
    TypeMismatch {
        node: &'static str,
        param: &'static str,
        expected: &'static str,
    },
    /// Parameter value out of valid range.
    OutOfRange {
        node: &'static str,
        param: &'static str,
        message: String,
    },
    /// Required parameter missing.
    MissingParam {
        node: &'static str,
        param: &'static str,
    },
    /// Invalid enum variant string.
    InvalidEnumVariant {
        node: &'static str,
        param: &'static str,
        value: String,
    },
    /// Generic error.
    Other(String),
}

impl fmt::Display for NodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownNode(id) => write!(f, "unknown node: {id}"),
            Self::UnknownParam { node, param } => {
                write!(f, "unknown parameter '{param}' on node '{node}'")
            }
            Self::TypeMismatch {
                node,
                param,
                expected,
            } => write!(
                f,
                "type mismatch for '{param}' on '{node}': expected {expected}"
            ),
            Self::OutOfRange {
                node,
                param,
                message,
            } => write!(f, "out of range for '{param}' on '{node}': {message}"),
            Self::MissingParam { node, param } => {
                write!(f, "missing required parameter '{param}' on '{node}'")
            }
            Self::InvalidEnumVariant { node, param, value } => {
                write!(f, "invalid variant '{value}' for '{param}' on '{node}'")
            }
            Self::Other(msg) => write!(f, "{msg}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for NodeError {}
