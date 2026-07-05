//! Source-reference metadata for audit trails.
//!
//! These types keep every event linked back to the raw file, provider, and
//! source-specific location that produced it.
use serde::{Deserialize, Serialize};

/// A wallet to ingest, as declared in the project config.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletSource {
    /// Stable config-declared id, e.g. `near_foxboss`. Used as `source_id` on events.
    pub id: String,
    pub name: String,
    pub chain: String,
    pub address: String,
    /// Key into the `[providers.*]` table of the config.
    pub provider: String,
}
