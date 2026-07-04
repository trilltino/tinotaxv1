use serde::{Deserialize, Serialize};
use tinotax_core::{EventType, NormalisedEvent};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewDiagnostics {
    pub needs_review: u64,
    pub unknown_contract_calls: u64,
    pub possible_swaps: u64,
    pub possible_bridges: u64,
    pub possible_airdrops: u64,
    pub possible_staking_rewards: u64,
    pub missing_token_decimals: u64,
    pub direction_unknown: u64,
    pub failed_txs: u64,
}

pub fn compute(events: &[NormalisedEvent]) -> ReviewDiagnostics {
    let mut review = ReviewDiagnostics {
        needs_review: 0,
        unknown_contract_calls: 0,
        possible_swaps: 0,
        possible_bridges: 0,
        possible_airdrops: 0,
        possible_staking_rewards: 0,
        missing_token_decimals: 0,
        direction_unknown: 0,
        failed_txs: 0,
    };
    for event in events {
        if event.needs_review {
            review.needs_review += 1;
        }
        match event.event_type {
            EventType::ContractCall => review.unknown_contract_calls += 1,
            EventType::PossibleSwap => review.possible_swaps += 1,
            EventType::PossibleBridge => review.possible_bridges += 1,
            EventType::PossibleAirdrop => review.possible_airdrops += 1,
            EventType::PossibleStakingReward => review.possible_staking_rewards += 1,
            _ => {}
        }
        for reason in &event.review_reasons {
            if reason.starts_with("missing_token_decimals") {
                review.missing_token_decimals += 1;
            }
            if reason.contains("direction_unknown") {
                review.direction_unknown += 1;
            }
            if reason.starts_with("tx_failed") || reason.starts_with("fee_for_failed_tx") {
                review.failed_txs += 1;
            }
        }
    }
    review
}
