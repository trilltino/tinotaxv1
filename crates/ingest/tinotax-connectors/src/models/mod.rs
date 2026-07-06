//! Provider response models shared by connector and normalisation code.
//!
//! These structs are intentionally close to API JSON. Domain-level meaning is
//! assigned later in `tinotax-normalise`.
pub mod blockscout;
pub mod nearblocks;

/// Providers are inconsistent about numbers-vs-strings; accept either.
pub fn value_as_u64(value: Option<&serde_json::Value>) -> Option<u64> {
    match value? {
        serde_json::Value::Number(n) => n.as_u64(),
        serde_json::Value::String(s) => s.parse().ok(),
        _ => None,
    }
}

/// Exact textual form of a JSON number or string — never routed through f64.
/// (serde_json's `arbitrary_precision` feature preserves the original digits.)
pub fn value_as_raw_string(value: Option<&serde_json::Value>) -> Option<String> {
    match value? {
        serde_json::Value::Number(n) => Some(n.to_string()),
        serde_json::Value::String(s) => Some(s.clone()),
        _ => None,
    }
}
