//! NEAR normalisation from NearBlocks account activity. The public txns
//! endpoint gives signer/receiver/actions/aggregate deposit — enough for
//! native transfers and review-flagged function calls, not full receipts.

use anyhow::{Context, Result};
use rust_decimal::Decimal;
use tinotax_connectors::models::nearblocks::{Txn, TxnsPage};
use tinotax_connectors::models::{value_as_raw_string, value_as_u64};
use tinotax_core::{
    Chain, Confidence, Direction, EventType, NormalisedEvent, ScaledAmount, SourceKind, SourceRef,
    WalletSource,
};
use tinotax_store::{EndpointCache, ProjectPaths};

use crate::classify::classify_contract_call;
use crate::event_id::event_id;
use crate::{Batch, RejectedItem};

pub fn normalise_near_wallet(
    paths: &ProjectPaths,
    project_id: &str,
    wallet: &WalletSource,
    batch: &mut Batch,
) -> Result<()> {
    let chain = Chain::Near;
    let wallet_id = wallet.address.to_ascii_lowercase();
    let native_symbol = chain.native_symbol().to_string();
    let native_decimals = chain.native_decimals();

    let cache = EndpointCache::open(paths, &wallet.chain, &wallet.address, "transactions")?;
    let mut fee_warning_emitted = false;

    for (page_num, path) in cache.list_pages()? {
        let text = std::fs::read_to_string(&path).with_context(|| format!("reading {path}"))?;
        let page: TxnsPage =
            serde_json::from_str(&text).with_context(|| format!("parsing {path}"))?;
        let rel_path = paths.relative(&path);

        for (idx, raw_item) in page.txns.iter().enumerate() {
            let json_path = format!("txns[{idx}]");
            let txn: Txn = match serde_json::from_value(raw_item.clone()) {
                Ok(t) => t,
                Err(err) => {
                    batch.rejected.push(RejectedItem {
                        chain: wallet.chain.clone(),
                        wallet: wallet.address.clone(),
                        raw_file: rel_path.clone(),
                        json_path,
                        reason: format!("unparseable NEAR txn: {err}"),
                        raw: raw_item.clone(),
                    });
                    continue;
                }
            };

            let mut reasons: Vec<String> = Vec::new();

            let signer = txn
                .signer_account_id
                .clone()
                .or_else(|| txn.predecessor_account_id.clone())
                .map(|s| s.to_ascii_lowercase());
            let receiver = txn
                .receiver_account_id
                .clone()
                .map(|s| s.to_ascii_lowercase());
            let direction = match (
                signer.as_deref() == Some(wallet_id.as_str()),
                receiver.as_deref() == Some(wallet_id.as_str()),
            ) {
                (true, true) => Direction::SelfTransfer,
                (true, false) => Direction::Out,
                (false, true) => Direction::In,
                (false, false) => {
                    reasons.push("near_txn_direction_unknown".to_string());
                    Direction::Unknown
                }
            };

            let timestamp = near_timestamp(txn.block_timestamp.as_ref()).unwrap_or_else(|| {
                reasons.push("missing_timestamp".to_string());
                String::new()
            });

            if matches!(txn.outcomes.as_ref().and_then(|o| o.status), Some(false)) {
                reasons.push("tx_failed".to_string());
            }

            let first_action = txn.actions.as_ref().and_then(|a| a.first());
            let action_kind = first_action.and_then(|a| a.action.clone());
            let method = first_action.and_then(|a| a.method.clone());

            let raw_deposit = value_as_raw_string(
                txn.actions_agg
                    .as_ref()
                    .and_then(|agg| agg.deposit.as_ref()),
            );
            // NearBlocks exposes an aggregate deposit for the transaction, so
            // v1 treats the visible native value as one movement and leaves
            // deeper receipt/action reconstruction to later enrichment.
            let (amount, raw_amount) = match raw_deposit {
                Some(raw) if raw != "0" => match ScaledAmount::from_raw(&raw, native_decimals) {
                    Ok(scaled) => {
                        if !scaled.exact {
                            reasons.push("amount_precision_truncated".to_string());
                        }
                        (scaled.value, Some(raw))
                    }
                    Err(_) => {
                        // NearBlocks sometimes aggregates deposits into
                        // scientific notation; keep the raw text and flag it.
                        reasons.push(format!("unparseable_deposit:{raw}"));
                        (Decimal::ZERO, Some(raw))
                    }
                },
                _ => (Decimal::ZERO, None),
            };

            let is_plain_transfer = matches!(action_kind.as_deref(), Some(kind) if kind.eq_ignore_ascii_case("transfer"))
                && method.is_none();

            let (event_type, confidence) = if is_plain_transfer && amount > Decimal::ZERO {
                (EventType::NativeTransfer, Confidence::High)
            } else if amount > Decimal::ZERO {
                // Function call with an attached deposit: value moved, but
                // the intent needs a human (or a later decoder).
                let class = classify_contract_call(method.as_deref());
                reasons.extend(class.reasons);
                (class.event_type, Confidence::Medium)
            } else {
                let class = classify_contract_call(method.as_deref());
                reasons.extend(class.reasons);
                (class.event_type, class.confidence)
            };

            // The txns endpoint does not expose gas fees; that needs the
            // receipts endpoint (milestone 2). Warn once per wallet.
            if !fee_warning_emitted && direction == Direction::Out {
                batch.warn(
                    &wallet.chain,
                    &wallet.address,
                    "NEAR gas fees are not available from the txns endpoint; \
                     fee extraction needs the receipts endpoint (planned)",
                );
                fee_warning_emitted = true;
            }

            let tx_hash = txn.transaction_hash.clone();
            let id = event_id(
                &wallet.chain,
                &wallet_id,
                &tx_hash,
                None,
                Some(0),
                &native_symbol,
                raw_amount.as_deref().unwrap_or("0"),
                direction.as_str(),
            );

            let needs_review = !reasons.is_empty() || event_type != EventType::NativeTransfer;
            batch.events.push(NormalisedEvent {
                event_id: id,
                project_id: project_id.to_string(),
                source_id: wallet.id.clone(),
                source_kind: SourceKind::Wallet,
                chain: wallet.chain.clone(),
                wallet: wallet.address.clone(),
                timestamp,
                block_number: value_as_u64(
                    txn.block.as_ref().and_then(|b| b.block_height.as_ref()),
                ),
                tx_hash,
                event_type,
                direction,
                asset_symbol: native_symbol.clone(),
                asset_contract: None,
                amount,
                raw_amount,
                token_decimals: u8::try_from(native_decimals).ok(),
                from_address: signer.clone(),
                to_address: receiver.clone(),
                fee_asset: None,
                fee_amount: None,
                counterparty: match direction {
                    Direction::Out => receiver,
                    Direction::In => signer,
                    _ => None,
                },
                method,
                confidence,
                needs_review,
                review_reasons: reasons,
                source_ref: SourceRef {
                    raw_file: rel_path.clone(),
                    raw_page: Some(page_num),
                    json_path: Some(json_path),
                    log_index: None,
                    movement_index: Some(0),
                },
            });
        }
    }

    Ok(())
}

/// NearBlocks block timestamps are nanoseconds since epoch.
fn near_timestamp(value: Option<&serde_json::Value>) -> Option<String> {
    let raw = value_as_raw_string(value)?;
    let nanos: i128 = raw.parse().ok()?;
    jiff::Timestamp::from_nanosecond(nanos)
        .ok()
        .map(|t| t.to_string())
}
