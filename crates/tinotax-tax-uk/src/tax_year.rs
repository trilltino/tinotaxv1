//! UK tax years run 6 April – 5 April. Milestone 2.

/// Label like `2024-25`. Placeholder until the engine lands.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TaxYear(pub String);
