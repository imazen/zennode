//! RIAPI key-value parsing with consumption tracking.
//!
//! When processing a querystring like `w=800&quality=85&exposure=1.5`,
//! multiple node definitions can each consume their relevant keys.
//! After all nodes have parsed, unconsumed keys generate warnings.

use alloc::string::String;
use alloc::vec::Vec;

/// RIAPI key-value pairs with consumption tracking.
pub struct KvPairs {
    entries: Vec<KvEntry>,
    warnings: Vec<KvWarning>,
}

struct KvEntry {
    key: String,
    value: String,
    consumed_by: Option<&'static str>,
}

/// A warning generated during KV parsing.
#[derive(Clone, Debug)]
pub struct KvWarning {
    /// The key that caused the warning.
    pub key: String,
    /// Warning category.
    pub kind: KvWarningKind,
    /// Human-readable message.
    pub message: String,
}

/// Categories of KV parsing warnings.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KvWarningKind {
    /// Key not recognized by any registered node.
    UnrecognizedKey,
    /// Key recognized but value could not be parsed.
    InvalidValue,
    /// Key is deprecated; use an alternative.
    DeprecatedKey,
    /// Same key appeared more than once (last value wins).
    DuplicateKey,
}

impl KvPairs {
    /// Parse from a URL querystring (e.g., `"w=800&quality=85&exposure=1.5"`).
    pub fn from_querystring(qs: &str) -> Self {
        let mut entries = Vec::new();
        let mut seen = alloc::collections::BTreeSet::new();
        let mut warnings = Vec::new();

        for part in qs.split('&') {
            if part.is_empty() {
                continue;
            }
            let (key, value) = match part.split_once('=') {
                Some((k, v)) => (k, v),
                None => (part, ""),
            };
            let key_lower = key.to_lowercase();
            if !seen.insert(key_lower.clone()) {
                warnings.push(KvWarning {
                    key: key_lower.clone(),
                    kind: KvWarningKind::DuplicateKey,
                    message: alloc::format!("duplicate key '{key_lower}', using last value"),
                });
                // Remove old entry, keep the new one (last wins)
                entries.retain(|e: &KvEntry| e.key != key_lower);
            }
            entries.push(KvEntry {
                key: key_lower,
                value: percent_decode(value),
                consumed_by: None,
            });
        }

        Self { entries, warnings }
    }

    /// Create from an iterator of key-value pairs.
    pub fn from_pairs(pairs: impl Iterator<Item = (String, String)>) -> Self {
        let entries = pairs
            .map(|(key, value)| KvEntry {
                key,
                value,
                consumed_by: None,
            })
            .collect();
        Self {
            entries,
            warnings: Vec::new(),
        }
    }

    /// Get the string value for a key, marking it as consumed.
    ///
    /// Returns `None` if the key is absent or already consumed.
    pub fn take(&mut self, key: &str, consumer: &'static str) -> Option<&str> {
        for entry in &mut self.entries {
            if entry.key == key && entry.consumed_by.is_none() {
                entry.consumed_by = Some(consumer);
                return Some(&entry.value);
            }
        }
        None
    }

    /// Take the value as an owned `String`, marking consumed.
    pub fn take_owned(&mut self, key: &str, consumer: &'static str) -> Option<String> {
        for entry in &mut self.entries {
            if entry.key == key && entry.consumed_by.is_none() {
                entry.consumed_by = Some(consumer);
                return Some(entry.value.clone());
            }
        }
        None
    }

    /// Get and parse as `f32`, marking consumed if present.
    pub fn take_f32(&mut self, key: &str, consumer: &'static str) -> Option<f32> {
        let val_str = self.take_owned(key, consumer)?;
        match val_str.parse::<f32>() {
            Ok(v) => Some(v),
            Err(_) => {
                self.warn(
                    key,
                    KvWarningKind::InvalidValue,
                    alloc::format!("cannot parse '{val_str}' as number for key '{key}'"),
                );
                None
            }
        }
    }

    /// Get and parse as `i32`, marking consumed if present.
    pub fn take_i32(&mut self, key: &str, consumer: &'static str) -> Option<i32> {
        let val_str = self.take_owned(key, consumer)?;
        match val_str.parse::<i32>() {
            Ok(v) => Some(v),
            Err(_) => {
                self.warn(
                    key,
                    KvWarningKind::InvalidValue,
                    alloc::format!("cannot parse '{val_str}' as integer for key '{key}'"),
                );
                None
            }
        }
    }

    /// Get and parse as `u32`, marking consumed if present.
    pub fn take_u32(&mut self, key: &str, consumer: &'static str) -> Option<u32> {
        let val_str = self.take_owned(key, consumer)?;
        match val_str.parse::<u32>() {
            Ok(v) => Some(v),
            Err(_) => {
                self.warn(
                    key,
                    KvWarningKind::InvalidValue,
                    alloc::format!("cannot parse '{val_str}' as unsigned integer for key '{key}'"),
                );
                None
            }
        }
    }

    /// Get and parse as `bool`, marking consumed if present.
    ///
    /// Accepts `"true"`, `"1"`, `"yes"` as true; `"false"`, `"0"`, `"no"` as false.
    pub fn take_bool(&mut self, key: &str, consumer: &'static str) -> Option<bool> {
        let val_str = self.take_owned(key, consumer)?;
        match val_str.to_lowercase().as_str() {
            "true" | "1" | "yes" => Some(true),
            "false" | "0" | "no" => Some(false),
            _ => {
                self.warn(
                    key,
                    KvWarningKind::InvalidValue,
                    alloc::format!("cannot parse '{val_str}' as boolean for key '{key}'"),
                );
                None
            }
        }
    }

    /// Peek at a value without consuming it.
    pub fn peek(&self, key: &str) -> Option<&str> {
        self.entries
            .iter()
            .find(|e| e.key == key && e.consumed_by.is_none())
            .map(|e| e.value.as_str())
    }

    /// Iterate over all unconsumed key-value pairs.
    pub fn unconsumed(&self) -> impl Iterator<Item = (&str, &str)> {
        self.entries
            .iter()
            .filter(|e| e.consumed_by.is_none())
            .map(|e| (e.key.as_str(), e.value.as_str()))
    }

    /// All warnings accumulated during parsing.
    pub fn warnings(&self) -> &[KvWarning] {
        &self.warnings
    }

    /// Add a warning manually.
    pub fn warn(
        &mut self,
        key: impl Into<String>,
        kind: KvWarningKind,
        message: impl Into<String>,
    ) {
        self.warnings.push(KvWarning {
            key: key.into(),
            kind,
            message: message.into(),
        });
    }
}

/// Minimal percent-decoding for querystring values.
fn percent_decode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.bytes();
    while let Some(b) = chars.next() {
        if b == b'+' {
            result.push(' ');
        } else if b == b'%' {
            let hi = chars.next().and_then(from_hex);
            let lo = chars.next().and_then(from_hex);
            if let (Some(h), Some(l)) = (hi, lo) {
                result.push((h << 4 | l) as char);
            } else {
                result.push('%');
            }
        } else {
            result.push(b as char);
        }
    }
    result
}

fn from_hex(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_querystring() {
        let mut kv = KvPairs::from_querystring("w=800&h=600&quality=85");
        assert_eq!(kv.take_u32("w", "test"), Some(800));
        assert_eq!(kv.take_u32("h", "test"), Some(600));
        assert_eq!(kv.take_i32("quality", "test"), Some(85));
        assert_eq!(kv.unconsumed().count(), 0);
    }

    #[test]
    fn unconsumed_keys() {
        let mut kv = KvPairs::from_querystring("w=800&unknown=foo&h=600");
        kv.take_u32("w", "test");
        kv.take_u32("h", "test");
        let unconsumed: Vec<_> = kv.unconsumed().collect();
        assert_eq!(unconsumed, vec![("unknown", "foo")]);
    }

    #[test]
    fn duplicate_keys_last_wins() {
        let mut kv = KvPairs::from_querystring("w=100&w=200");
        assert_eq!(kv.take_u32("w", "test"), Some(200));
        assert!(
            kv.warnings()
                .iter()
                .any(|w| w.kind == KvWarningKind::DuplicateKey)
        );
    }

    #[test]
    fn bool_parsing() {
        let mut kv = KvPairs::from_querystring("a=true&b=0&c=yes&d=NO");
        assert_eq!(kv.take_bool("a", "t"), Some(true));
        assert_eq!(kv.take_bool("b", "t"), Some(false));
        assert_eq!(kv.take_bool("c", "t"), Some(true));
        assert_eq!(kv.take_bool("d", "t"), Some(false));
    }

    #[test]
    fn percent_decoding() {
        let mut kv = KvPairs::from_querystring("name=hello+world&path=%2Ffoo%2Fbar");
        assert_eq!(kv.take("name", "t"), Some("hello world"));
        assert_eq!(kv.take("path", "t"), Some("/foo/bar"));
    }

    #[test]
    fn consumed_not_returned_again() {
        let mut kv = KvPairs::from_querystring("w=800");
        assert_eq!(kv.take_u32("w", "first"), Some(800));
        assert_eq!(kv.take_u32("w", "second"), None);
    }

    #[test]
    fn case_insensitive_keys() {
        let mut kv = KvPairs::from_querystring("Quality=85&WIDTH=800");
        assert_eq!(kv.take_i32("quality", "t"), Some(85));
        assert_eq!(kv.take_u32("width", "t"), Some(800));
    }
}
