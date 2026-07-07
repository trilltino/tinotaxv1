//! Blockscout-style EVM normalisation: native transfers (top-level and
//! internal call frames), gas fees, token transfers, and review-flagged
//! contract calls. No DeFi decoding in v1.

use std::collections::HashSet;

use anyhow::{Context, Result};
use rust_decimal::Decimal;
use tinotax_connectors::models::blockscout::{
    InternalTransaction, Page, TokenTransfer, Transaction,
};
use tinotax_connectors::models::value_as_u64;
use tinotax_core::{
    Chain, Confidence, Direction, EventType, NormalisedEvent, ScaledAmount, SourceKind, SourceRef,
    WalletSource,
};
use tinotax_store::{EndpointCache, ProjectPaths};

use crate::classify::classify_contract_call;
use crate::event_id::event_id;
use crate::{Batch, RejectedItem};

pub fn normalise_evm_wallet(
    paths: &ProjectPaths,
    project_id: &str,
    wallet: &WalletSource,
    chain: &Chain,
    batch: &mut Batch,
) -> Result<()> {
    // Token transfers first: transaction classification needs to know which
    // tx hashes already have decoded movements.
    let token_tx_hashes = normalise_token_transfers(paths, project_id, wallet, batch)?;
    normalise_transactions(paths, project_id, wallet, chain, &token_tx_hashes, batch)?;
    normalise_internal_transactions(paths, project_id, wallet, chain, batch)?;
    Ok(())
}

/// `normalise_transactions` claims movement indices 0..=2 per tx hash;
/// internal call frames get their own space above this base so event IDs of
/// the same transaction can never collide.
const INTERNAL_MOVEMENT_BASE: u64 = 10;

/// Native value carried by internal call frames. The transactions endpoint
/// only reports a transaction's own `value`, so ETH paid out by contracts
/// (bridge payouts, DEX swap outputs, WETH unwraps) is visible nowhere else.
/// Gas is not re-counted here: it stays on the top-level transaction.
fn normalise_internal_transactions(
    paths: &ProjectPaths,
    project_id: &str,
    wallet: &WalletSource,
    chain: &Chain,
    batch: &mut Batch,
) -> Result<()> {
    let wallet_addr = wallet.address.to_ascii_lowercase();
    let native_symbol = chain.native_symbol().to_string();
    let native_decimals = chain.native_decimals();

    for (page_num, rel_path, page) in read_pages(paths, wallet, "internal_transactions")? {
        for (idx, raw_item) in page.items.iter().enumerate() {
            let json_path = format!("items[{idx}]");
            let frame: InternalTransaction = match serde_json::from_value(raw_item.clone()) {
                Ok(f) => f,
                Err(err) => {
                    batch.rejected.push(RejectedItem {
                        chain: wallet.chain.clone(),
                        wallet: wallet.address.clone(),
                        raw_file: rel_path.clone(),
                        json_path,
                        reason: format!("unparseable internal transaction: {err}"),
                        raw: raw_item.clone(),
                    });
                    continue;
                }
            };

            // Reverted frames move no value; zero-value frames carry no
            // taxable movement.
            let failed = frame.error.is_some() || frame.success == Some(false);
            let raw_value = frame.value.clone().unwrap_or_default();
            if failed || raw_value.is_empty() || raw_value == "0" {
                continue;
            }

            let Some(tx_hash) = frame.transaction_hash.clone() else {
                batch.rejected.push(RejectedItem {
                    chain: wallet.chain.clone(),
                    wallet: wallet.address.clone(),
                    raw_file: rel_path.clone(),
                    json_path,
                    reason: "internal transaction without transaction hash".to_string(),
                    raw: raw_item.clone(),
                });
                continue;
            };
            let tx_hash = tx_hash.to_ascii_lowercase();

            let mut reasons: Vec<String> = Vec::new();
            let from = frame
                .from
                .as_ref()
                .and_then(|a| a.hash.as_ref())
                .map(|h| h.to_ascii_lowercase());
            let to = frame
                .to
                .as_ref()
                .and_then(|a| a.hash.as_ref())
                .map(|h| h.to_ascii_lowercase());
            let direction = direction_relative_to(&wallet_addr, from.as_deref(), to.as_deref());
            if direction == Direction::Unknown {
                reasons.push("internal_transfer_direction_unknown".to_string());
            }

            let amount = match ScaledAmount::from_raw(&raw_value, native_decimals) {
                Ok(scaled) => {
                    if !scaled.exact {
                        reasons.push("amount_precision_truncated".to_string());
                    }
                    scaled.value
                }
                Err(err) => {
                    batch.rejected.push(RejectedItem {
                        chain: wallet.chain.clone(),
                        wallet: wallet.address.clone(),
                        raw_file: rel_path.clone(),
                        json_path,
                        reason: format!("unparseable internal value: {err}"),
                        raw: raw_item.clone(),
                    });
                    continue;
                }
            };

            let timestamp = frame.timestamp.clone().unwrap_or_else(|| {
                reasons.push("missing_timestamp".to_string());
                String::new()
            });

            let movement_index = match value_as_u64(frame.index.as_ref()) {
                Some(index) => INTERNAL_MOVEMENT_BASE + index,
                None => {
                    reasons.push("missing_internal_call_index".to_string());
                    INTERNAL_MOVEMENT_BASE + idx as u64
                }
            };

            let id = event_id(
                &wallet.chain,
                &wallet_addr,
                &tx_hash,
                None,
                Some(movement_index),
                &native_symbol,
                &raw_value,
                direction.as_str(),
            );

            let needs_review = !reasons.is_empty();
            batch.events.push(NormalisedEvent {
                event_id: id,
                project_id: project_id.to_string(),
                source_id: wallet.id.clone(),
                source_kind: SourceKind::Wallet,
                chain: wallet.chain.clone(),
                wallet: wallet.address.clone(),
                timestamp,
                block_number: value_as_u64(frame.block_number.as_ref()),
                tx_hash,
                event_type: EventType::NativeTransfer,
                direction,
                asset_symbol: native_symbol.clone(),
                asset_contract: None,
                amount,
                raw_amount: Some(raw_value),
                token_decimals: u8::try_from(native_decimals).ok(),
                from_address: from.clone(),
                to_address: to.clone(),
                fee_asset: None,
                fee_amount: None,
                counterparty: counterparty_of(&wallet_addr, from.as_deref(), to.as_deref()),
                method: None,
                confidence: if needs_review {
                    Confidence::Medium
                } else {
                    Confidence::High
                },
                needs_review,
                review_reasons: reasons,
                source_ref: SourceRef {
                    raw_file: rel_path.clone(),
                    raw_page: Some(page_num),
                    json_path: Some(json_path),
                    log_index: None,
                    movement_index: Some(movement_index),
                },
            });
        }
    }

    Ok(())
}

fn read_pages(
    paths: &ProjectPaths,
    wallet: &WalletSource,
    endpoint: &str,
) -> Result<Vec<(u64, String, Page)>> {
    let cache = EndpointCache::open(paths, &wallet.chain, &wallet.address, endpoint)?;
    let mut pages = Vec::new();
    for (page_num, path) in cache.list_pages()? {
        let text = std::fs::read_to_string(&path).with_context(|| format!("reading {path}"))?;
        let page: Page = serde_json::from_str(&text).with_context(|| format!("parsing {path}"))?;
        // Keep both the page number and project-relative path so each emitted
        // event can be traced back to exact raw evidence.
        pages.push((page_num, paths.relative(&path), page));
    }
    Ok(pages)
}

fn normalise_token_transfers(
    paths: &ProjectPaths,
    project_id: &str,
    wallet: &WalletSource,
    batch: &mut Batch,
) -> Result<HashSet<String>> {
    let wallet_addr = wallet.address.to_ascii_lowercase();
    let mut tx_hashes = HashSet::new();

    for (page_num, rel_path, page) in read_pages(paths, wallet, "token_transfers")? {
        for (idx, raw_item) in page.items.iter().enumerate() {
            let json_path = format!("items[{idx}]");
            let transfer: TokenTransfer = match serde_json::from_value(raw_item.clone()) {
                Ok(t) => t,
                Err(err) => {
                    batch.rejected.push(RejectedItem {
                        chain: wallet.chain.clone(),
                        wallet: wallet.address.clone(),
                        raw_file: rel_path.clone(),
                        json_path,
                        reason: format!("unparseable token transfer: {err}"),
                        raw: raw_item.clone(),
                    });
                    continue;
                }
            };

            let Some(tx_hash) = transfer.transaction_hash.clone() else {
                batch.rejected.push(RejectedItem {
                    chain: wallet.chain.clone(),
                    wallet: wallet.address.clone(),
                    raw_file: rel_path.clone(),
                    json_path,
                    reason: "token transfer without transaction hash".to_string(),
                    raw: raw_item.clone(),
                });
                continue;
            };
            tx_hashes.insert(tx_hash.to_ascii_lowercase());

            let mut reasons: Vec<String> = Vec::new();

            let from = transfer
                .from
                .as_ref()
                .and_then(|a| a.hash.as_ref())
                .map(|h| h.to_ascii_lowercase());
            let to = transfer
                .to
                .as_ref()
                .and_then(|a| a.hash.as_ref())
                .map(|h| h.to_ascii_lowercase());
            let direction = direction_relative_to(&wallet_addr, from.as_deref(), to.as_deref());
            if direction == Direction::Unknown {
                // Direction can be unknown when provider data is incomplete or
                // neither side matches the wallet; keep the event reviewable.
                reasons.push("token_transfer_direction_unknown".to_string());
            }

            let token = transfer.token.as_ref();
            let symbol = token.and_then(|t| t.symbol.clone()).unwrap_or_else(|| {
                reasons.push("missing_token_symbol".to_string());
                "UNKNOWN".to_string()
            });
            let contract = token.and_then(|t| t.address.clone());

            let decimals: Option<u32> = transfer
                .total
                .as_ref()
                .and_then(|t| t.decimals.as_ref())
                .or_else(|| token.and_then(|t| t.decimals.as_ref()))
                .and_then(|d| d.parse().ok());

            let token_type = token.and_then(|t| t.token_type.as_deref());
            let raw_value = match transfer.total.as_ref().and_then(|t| t.value.clone()) {
                Some(value) => value,
                None if matches!(token_type, Some("ERC-721")) => {
                    let token_id = transfer
                        .total
                        .as_ref()
                        .and_then(|t| t.token_id.clone())
                        .unwrap_or_else(|| "unknown".to_string());
                    reasons.push("nft_transfer_quantity_assumed_one".to_string());
                    reasons.push(format!("nft_token_id:{token_id}"));
                    "1".to_string()
                }
                None => {
                    batch.rejected.push(RejectedItem {
                        chain: wallet.chain.clone(),
                        wallet: wallet.address.clone(),
                        raw_file: rel_path.clone(),
                        json_path,
                        reason: "token transfer without total.value".to_string(),
                        raw: raw_item.clone(),
                    });
                    continue;
                }
            };

            let scale = decimals.unwrap_or_else(|| {
                reasons.push("missing_token_decimals".to_string());
                0
            });
            let amount = match ScaledAmount::from_raw(&raw_value, scale) {
                Ok(scaled) => {
                    if !scaled.exact {
                        reasons.push("amount_precision_truncated".to_string());
                    }
                    scaled.value
                }
                Err(err) => {
                    batch.rejected.push(RejectedItem {
                        chain: wallet.chain.clone(),
                        wallet: wallet.address.clone(),
                        raw_file: rel_path.clone(),
                        json_path,
                        reason: format!("unparseable token amount: {err}"),
                        raw: raw_item.clone(),
                    });
                    continue;
                }
            };

            let timestamp = transfer.timestamp.clone().unwrap_or_else(|| {
                reasons.push("missing_timestamp".to_string());
                String::new()
            });
            let log_index = value_as_u64(transfer.log_index.as_ref());

            let id = event_id(
                &wallet.chain,
                &wallet_addr,
                &tx_hash,
                log_index,
                None,
                &symbol,
                &raw_value,
                direction.as_str(),
            );

            let needs_review = !reasons.is_empty();
            batch.events.push(NormalisedEvent {
                event_id: id,
                project_id: project_id.to_string(),
                source_id: wallet.id.clone(),
                source_kind: SourceKind::Wallet,
                chain: wallet.chain.clone(),
                wallet: wallet.address.clone(),
                timestamp,
                block_number: value_as_u64(transfer.block_number.as_ref()),
                tx_hash,
                event_type: EventType::TokenTransfer,
                direction,
                asset_symbol: symbol,
                asset_contract: contract,
                amount,
                raw_amount: Some(raw_value),
                token_decimals: decimals.and_then(|d| u8::try_from(d).ok()),
                from_address: from.clone(),
                to_address: to.clone(),
                fee_asset: None,
                fee_amount: None,
                counterparty: counterparty_of(&wallet_addr, from.as_deref(), to.as_deref()),
                method: None,
                confidence: if needs_review {
                    Confidence::Medium
                } else {
                    Confidence::High
                },
                needs_review,
                review_reasons: reasons,
                source_ref: SourceRef {
                    raw_file: rel_path.clone(),
                    raw_page: Some(page_num),
                    json_path: Some(json_path),
                    log_index,
                    movement_index: None,
                },
            });
        }
    }

    Ok(tx_hashes)
}

fn normalise_transactions(
    paths: &ProjectPaths,
    project_id: &str,
    wallet: &WalletSource,
    chain: &Chain,
    token_tx_hashes: &HashSet<String>,
    batch: &mut Batch,
) -> Result<()> {
    let wallet_addr = wallet.address.to_ascii_lowercase();
    let native_symbol = chain.native_symbol().to_string();
    let native_decimals = chain.native_decimals();

    for (page_num, rel_path, page) in read_pages(paths, wallet, "transactions")? {
        for (idx, raw_item) in page.items.iter().enumerate() {
            let json_path = format!("items[{idx}]");
            let tx: Transaction = match serde_json::from_value(raw_item.clone()) {
                Ok(t) => t,
                Err(err) => {
                    batch.rejected.push(RejectedItem {
                        chain: wallet.chain.clone(),
                        wallet: wallet.address.clone(),
                        raw_file: rel_path.clone(),
                        json_path,
                        reason: format!("unparseable transaction: {err}"),
                        raw: raw_item.clone(),
                    });
                    continue;
                }
            };

            let tx_hash = tx.hash.to_ascii_lowercase();
            let from = tx
                .from
                .as_ref()
                .and_then(|a| a.hash.as_ref())
                .map(|h| h.to_ascii_lowercase());
            let to = tx
                .to
                .as_ref()
                .and_then(|a| a.hash.as_ref())
                .map(|h| h.to_ascii_lowercase());
            let timestamp = tx.timestamp.clone().unwrap_or_default();
            let block_number = value_as_u64(tx.block_number.as_ref());
            let is_sender = from.as_deref() == Some(wallet_addr.as_str());
            let failed = matches!(tx.status.as_deref(), Some(s) if !s.eq_ignore_ascii_case("ok"));

            let base = |event_type: EventType,
                        direction: Direction,
                        amount: Decimal,
                        raw_amount: Option<String>,
                        movement_index: u64,
                        confidence: Confidence,
                        needs_review: bool,
                        reasons: Vec<String>,
                        id: String| NormalisedEvent {
                event_id: id,
                project_id: project_id.to_string(),
                source_id: wallet.id.clone(),
                source_kind: SourceKind::Wallet,
                chain: wallet.chain.clone(),
                wallet: wallet.address.clone(),
                timestamp: timestamp.clone(),
                block_number,
                tx_hash: tx_hash.clone(),
                event_type,
                direction,
                asset_symbol: native_symbol.clone(),
                asset_contract: None,
                amount,
                raw_amount,
                token_decimals: u8::try_from(native_decimals).ok(),
                from_address: from.clone(),
                to_address: to.clone(),
                fee_asset: None,
                fee_amount: None,
                counterparty: counterparty_of(&wallet_addr, from.as_deref(), to.as_deref()),
                method: tx.method.clone(),
                confidence,
                needs_review,
                review_reasons: reasons,
                source_ref: SourceRef {
                    raw_file: rel_path.clone(),
                    raw_page: Some(page_num),
                    json_path: Some(json_path.clone()),
                    log_index: None,
                    movement_index: Some(movement_index),
                },
            };

            // Movement 0: the native value transfer (skipped for failed txs —
            // a reverted tx moves no value, only gas).
            let raw_value = tx.value.clone().unwrap_or_default();
            let has_value = !raw_value.is_empty() && raw_value != "0";
            if has_value && !failed {
                let mut reasons = Vec::new();
                let direction = direction_relative_to(&wallet_addr, from.as_deref(), to.as_deref());
                if direction == Direction::Unknown {
                    reasons.push("native_transfer_direction_unknown".to_string());
                }
                match ScaledAmount::from_raw(&raw_value, native_decimals) {
                    Ok(scaled) => {
                        if !scaled.exact {
                            reasons.push("amount_precision_truncated".to_string());
                        }
                        let needs_review = !reasons.is_empty();
                        let id = event_id(
                            &wallet.chain,
                            &wallet_addr,
                            &tx_hash,
                            None,
                            Some(0),
                            &native_symbol,
                            &raw_value,
                            direction.as_str(),
                        );
                        batch.events.push(base(
                            EventType::NativeTransfer,
                            direction,
                            scaled.value,
                            Some(raw_value.clone()),
                            0,
                            if needs_review {
                                Confidence::Medium
                            } else {
                                Confidence::High
                            },
                            needs_review,
                            reasons,
                            id,
                        ));
                    }
                    Err(err) => {
                        batch.rejected.push(RejectedItem {
                            chain: wallet.chain.clone(),
                            wallet: wallet.address.clone(),
                            raw_file: rel_path.clone(),
                            json_path: json_path.clone(),
                            reason: format!("unparseable native value: {err}"),
                            raw: raw_item.clone(),
                        });
                    }
                }
            }

            // Movement 1: gas fee — paid by the sender even when the tx failed.
            if is_sender {
                if let Some(raw_fee) = tx.fee.as_ref().and_then(|f| f.value.clone()) {
                    if !raw_fee.is_empty() && raw_fee != "0" {
                        if let Ok(scaled) = ScaledAmount::from_raw(&raw_fee, native_decimals) {
                            let mut reasons = Vec::new();
                            if failed {
                                reasons.push(format!(
                                    "fee_for_failed_tx:{}",
                                    tx.status.clone().unwrap_or_default()
                                ));
                            }
                            let id = event_id(
                                &wallet.chain,
                                &wallet_addr,
                                &tx_hash,
                                None,
                                Some(1),
                                &native_symbol,
                                &raw_fee,
                                Direction::Out.as_str(),
                            );
                            batch.events.push(base(
                                EventType::Fee,
                                Direction::Out,
                                scaled.value,
                                Some(raw_fee),
                                1,
                                Confidence::High,
                                false,
                                reasons,
                                id,
                            ));
                        }
                    }
                }
            }

            // Movement 2: opaque contract interaction — only when there was no
            // native value and no token transfer already captured for this tx.
            let has_token_movement = token_tx_hashes.contains(&tx_hash);
            if !has_value && !has_token_movement && !failed {
                // Opaque calls are preserved as zero-amount review rows so
                // unknown DeFi or approvals do not vanish from the audit trail.
                let class = classify_contract_call(tx.method.as_deref());
                let direction = if is_sender {
                    Direction::Out
                } else {
                    Direction::In
                };
                let id = event_id(
                    &wallet.chain,
                    &wallet_addr,
                    &tx_hash,
                    None,
                    Some(2),
                    &native_symbol,
                    "0",
                    direction.as_str(),
                );
                batch.events.push(base(
                    class.event_type,
                    direction,
                    Decimal::ZERO,
                    None,
                    2,
                    class.confidence,
                    class.needs_review,
                    class.reasons,
                    id,
                ));
            }
        }
    }

    Ok(())
}

fn direction_relative_to(wallet: &str, from: Option<&str>, to: Option<&str>) -> Direction {
    let is_from = from == Some(wallet);
    let is_to = to == Some(wallet);
    match (is_from, is_to) {
        (true, true) => Direction::SelfTransfer,
        (true, false) => Direction::Out,
        (false, true) => Direction::In,
        (false, false) => Direction::Unknown,
    }
}

fn counterparty_of(wallet: &str, from: Option<&str>, to: Option<&str>) -> Option<String> {
    if from == Some(wallet) {
        to.map(str::to_string)
    } else if to == Some(wallet) {
        from.map(str::to_string)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use camino::Utf8PathBuf;
    use pretty_assertions::assert_eq;

    const WALLET: &str = "0x1b4399A7c97ae092fB4CCDc1598b2767ECB79652";
    const BRIDGE: &str = "0x9552a0a6624A23B848060AE5901659CDDa1f83f8";

    fn frame(value: &str, index: u64, to: &str, success: bool) -> serde_json::Value {
        serde_json::json!({
            "transaction_hash": "0xf35fc2c6a7233ab411e60761456aa95e4a4b2f90c1a51fa0fc021c3113e1faf0",
            "index": index,
            "block_number": 11_989_120,
            "timestamp": "2025-02-04T22:10:31.000000Z",
            "from": { "hash": BRIDGE },
            "to": { "hash": to },
            "value": value,
            "success": success,
            "error": if success { serde_json::Value::Null } else { "Reverted".into() },
            "type": "call"
        })
    }

    #[test]
    fn internal_frames_become_native_transfers() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let root = Utf8PathBuf::from_path_buf(tmp.path().to_path_buf())
            .map_err(|p| anyhow::anyhow!("non-UTF8 path {}", p.display()))?;
        let paths = ProjectPaths::new(root);
        paths.init()?;

        let wallet = WalletSource {
            id: "lisk_main".to_string(),
            name: "test".to_string(),
            chain: "lisk-evm".to_string(),
            address: WALLET.to_string(),
            provider: "lisk_blockscout".to_string(),
        };
        let cache =
            EndpointCache::open(&paths, &wallet.chain, &wallet.address, "internal_transactions")?;
        cache.write_page(
            1,
            &serde_json::json!({ "items": [
                frame("914837044695905424", 6, WALLET, true), // inflow: kept
                frame("0", 7, WALLET, true),                  // zero value: skipped
                frame("5", 8, WALLET, false),                 // reverted: skipped
            ]}),
        )?;

        let mut batch = Batch::default();
        normalise_evm_wallet(&paths, "proj", &wallet, &Chain::LiskEvm, &mut batch)?;

        assert_eq!(batch.rejected.len(), 0);
        assert_eq!(batch.events.len(), 1);
        let event = &batch.events[0];
        assert_eq!(event.event_type, EventType::NativeTransfer);
        assert_eq!(event.direction, Direction::In);
        assert_eq!(event.asset_symbol, "ETH");
        assert_eq!(event.amount.to_string(), "0.914837044695905424");
        assert_eq!(event.counterparty.as_deref(), Some(&*BRIDGE.to_ascii_lowercase()));
        assert_eq!(
            event.source_ref.movement_index,
            Some(INTERNAL_MOVEMENT_BASE + 6)
        );
        assert!(event.fee_amount.is_none(), "gas stays on the top-level tx");
        assert!(!event.needs_review);

        // Same raw data must yield the same id so review decisions survive.
        let mut second = Batch::default();
        normalise_evm_wallet(&paths, "proj", &wallet, &Chain::LiskEvm, &mut second)?;
        assert_eq!(second.events[0].event_id, event.event_id);
        Ok(())
    }
}
