//! Conservative machine classification hints.
//!
//! Classifiers should prefer reviewable uncertainty over aggressive tax
//! assumptions. Human review can later promote or correct the suggested type.
use tinotax_core::{Confidence, EventType};

/// Result of the deliberately shallow v1 classifier. We do not decode DeFi;
/// we bucket by method-name hints and route everything uncertain to review.
#[derive(Debug, Clone)]
pub struct Classification {
    pub event_type: EventType,
    pub confidence: Confidence,
    pub needs_review: bool,
    pub reasons: Vec<String>,
}

/// Classify a contract interaction with no decoded value movement.
pub fn classify_contract_call(method: Option<&str>) -> Classification {
    let Some(method) = method else {
        return Classification {
            event_type: EventType::ContractCall,
            confidence: Confidence::Low,
            needs_review: true,
            reasons: vec!["contract_call_without_decoded_movement".to_string()],
        };
    };
    let lower = method.to_ascii_lowercase();

    if lower.contains("swap") {
        Classification {
            event_type: EventType::PossibleSwap,
            confidence: Confidence::Medium,
            needs_review: true,
            reasons: vec![format!("method_suggests_swap:{method}")],
        }
    } else if lower.contains("bridge") {
        Classification {
            event_type: EventType::PossibleBridge,
            confidence: Confidence::Medium,
            needs_review: true,
            reasons: vec![format!("method_suggests_bridge:{method}")],
        }
    } else if lower.contains("claim") {
        Classification {
            event_type: EventType::PossibleAirdrop,
            confidence: Confidence::Low,
            needs_review: true,
            reasons: vec![format!("method_suggests_claim:{method}")],
        }
    } else if lower.contains("stake") || lower.contains("unstake") {
        Classification {
            event_type: EventType::PossibleStakingReward,
            confidence: Confidence::Low,
            needs_review: true,
            reasons: vec![format!("method_suggests_staking:{method}")],
        }
    } else {
        Classification {
            event_type: EventType::ContractCall,
            confidence: Confidence::Low,
            needs_review: true,
            reasons: vec![format!("unclassified_contract_call:{method}")],
        }
    }
}
