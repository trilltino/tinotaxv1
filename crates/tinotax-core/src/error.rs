use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("invalid raw amount {raw:?}: {reason}")]
    InvalidAmount { raw: String, reason: String },

    #[error("unknown chain {0:?}")]
    UnknownChain(String),

    #[error("unknown review action {0:?}")]
    UnknownReviewAction(String),

    #[error("unknown tax event type {0:?}")]
    UnknownTaxEventType(String),

    #[error("unknown price source {0:?}")]
    UnknownPriceSource(String),

    #[error("invalid timestamp {0:?} (expected RFC 3339)")]
    InvalidTimestamp(String),
}
