//! Deterministic normalised event ID generation.
//!
//! IDs are derived from stable source fields rather than runtime state so
//! review decisions can be re-applied after regeneration from raw cache.
/// Deterministic event identity: re-running the import over the same raw
/// data yields the same ID, so reviews and downstream state survive reruns.
///
/// `event_id = blake3(chain | wallet | tx_hash | log_index | movement_index | asset | amount | direction)`
#[allow(clippy::too_many_arguments)]
pub fn event_id(
    chain: &str,
    wallet: &str,
    tx_hash: &str,
    log_index: Option<u64>,
    movement_index: Option<u64>,
    asset: &str,
    amount: &str,
    direction: &str,
) -> String {
    let material = format!(
        "{chain}|{wallet}|{tx_hash}|{}|{}|{asset}|{amount}|{direction}",
        log_index.map(|v| v.to_string()).unwrap_or_default(),
        movement_index.map(|v| v.to_string()).unwrap_or_default(),
    );
    blake3::hash(material.as_bytes()).to_hex().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_deterministic_and_input_sensitive() {
        let a = event_id("near", "w", "tx", None, Some(0), "NEAR", "1", "in");
        let b = event_id("near", "w", "tx", None, Some(0), "NEAR", "1", "in");
        let c = event_id("near", "w", "tx", None, Some(1), "NEAR", "1", "in");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
