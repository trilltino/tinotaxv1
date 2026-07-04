use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("invalid raw amount {raw:?}: {reason}")]
    InvalidAmount { raw: String, reason: String },

    #[error("unknown chain {0:?}")]
    UnknownChain(String),

    #[error("unknown review action {0:?}")]
    UnknownReviewAction(String),
}
