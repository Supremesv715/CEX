use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use rust_decimal::Decimal;
use rust_decimal::prelude::{ToPrimitive, FromPrimitive};
use chrono::{DateTime, Utc};
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tokio::sync::broadcast;
use crate::db::DbPool;
use crate::repo;

// ethers for Chainlink RPC
use ethers::providers::{Provider, Http};
use ethers::types::Address;
use ethers::contract::abigen;
use std::convert::TryFrom;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PriceInfo {
    pub base: String,
    pub quote: String,
    #[serde(with = "rust_decimal::serde::str")]
    pub price: Decimal,
    pub fetched_at: DateTime<Utc>,
    pub source: String,
    pub valid: bool,
}

pub type PriceCache = DashMap<String, PriceInfo>;

/// Start a simple CoinGecko poller for BTC, ETH, and SOL vs USD (one batched HTTP call per tick).
/// Updates the provided `cache` and broadcasts price JSON via `price_tx`.
pub async fn start_coingecko_poller(cache: Arc<PriceCache>, price_tx: broadcast::Sender<serde_json::Value>, interval_secs: u64, db: Option<DbPool>) {
    let mappings = vec![
        ("BTC_USD", "bitcoin", "usd"),
        ("ETH_USD", "ethereum", "usd"),
        ("SOL_USD", "solana", "usd"),
    ];

    let max_dev_pct: f64 = std::env::var("PRICE_MAX_DEVIATION_PERC").ok().and_then(|s| s.parse().ok()).unwrap_or(5.0);

    tokio::spawn(async move {
        let client = reqwest::Client::new();
        loop {
            let url = "https://api.coingecko.com/api/v3/simple/price?ids=bitcoin,ethereum,solana&vs_currencies=usd";
            match client
                .get(url)
                .header("User-Agent", "engine-cex/0.1 (price poller; contact: local dev)")
                .send()
                .await
            {
                Ok(resp) => {
                    let status = resp.status();
                    if !status.is_success() {
                        let text = resp.text().await.unwrap_or_default();
                        eprintln!(
                            "CoinGecko HTTP {} for {}: {}",
                            status,
                            url,
                            text.chars().take(200).collect::<String>()
                        );
                    } else if let Ok(body) = resp.json::<serde_json::Value>().await {
                        for (key, id, vs) in &mappings {
                            if let Some(price_val) = body.get(*id).and_then(|o| o.get(*vs)) {
                                if let Some(price_f) = price_val.as_f64() {
                                    let price_dec = Decimal::from_f64(price_f).unwrap_or_else(|| Decimal::new(0, 0));
                                    let parts: Vec<&str> = key.split('_').collect();
                                    let mut info = PriceInfo {
                                        base: parts[0].to_string(),
                                        quote: parts[1].to_string(),
                                        price: price_dec,
                                        fetched_at: Utc::now(),
                                        source: "coingecko".to_string(),
                                        valid: true,
                                    };

                                    if let Some(existing) = cache.get(&key.to_string()) {
                                        let other = existing.value();
                                        if other.source == "chainlink" {
                                            if let (Some(a), Some(b)) = (info.price.to_f64(), other.price.to_f64()) {
                                                let diff = (a - b).abs();
                                                let pct = if b != 0.0 { diff / b * 100.0 } else { 0.0 };
                                                if pct > max_dev_pct {
                                                    info.valid = false;
                                                }
                                            }
                                        }
                                    }

                                    cache.insert(key.to_string(), info.clone());
                                    if let Ok(js) = serde_json::to_value(&info) {
                                        let _ = price_tx.send(js);
                                    }
                                    if let Some(pool) = &db {
                                        if let Err(e) = repo::insert_price_snapshot(
                                            pool,
                                            &info.base,
                                            &info.quote,
                                            info.price,
                                            info.fetched_at,
                                            Some("coingecko"),
                                        )
                                        .await
                                        {
                                            eprintln!("price snapshot DB insert failed: {}", e);
                                        }
                                    }
                                } else {
                                    eprintln!("CoinGecko JSON missing numeric price for {}", key);
                                }
                            } else {
                                eprintln!("CoinGecko JSON missing {} / {} — body: {}", id, vs, body);
                            }
                        }
                    } else {
                        eprintln!("CoinGecko JSON parse failed for {}", url);
                    }
                }
                Err(e) => {
                    eprintln!("CoinGecko request failed: {}", e);
                }
            }
            sleep(Duration::from_secs(interval_secs)).await;
        }
    });
}

abigen!(AggregatorV3Interface, r#"[
  {"inputs":[],"name":"decimals","outputs":[{"internalType":"uint8","name":"","type":"uint8"}],"stateMutability":"view","type":"function"},
  {"inputs":[{"internalType":"uint80","name":"_roundId","type":"uint80"}],"name":"getRoundData","outputs":[{"internalType":"uint80","name":"roundId","type":"uint80"},{"internalType":"int256","name":"answer","type":"int256"},{"internalType":"uint256","name":"startedAt","type":"uint256"},{"internalType":"uint256","name":"updatedAt","type":"uint256"},{"internalType":"uint80","name":"answeredInRound","type":"uint80"}],"stateMutability":"view","type":"function"},
  {"inputs":[],"name":"latestRoundData","outputs":[{"internalType":"uint80","name":"roundId","type":"uint80"},{"internalType":"int256","name":"answer","type":"int256"},{"internalType":"uint256","name":"startedAt","type":"uint256"},{"internalType":"uint256","name":"updatedAt","type":"uint256"},{"internalType":"uint80","name":"answeredInRound","type":"uint80"}],"stateMutability":"view","type":"function"}
]"#);

/// Start a Chainlink poller that reads aggregator contracts and updates the cache.
/// `feeds` is a vec of tuples: ("PAIR_KEY", "0xADDRESS") where PAIR_KEY is like "BTC_USD".
pub async fn start_chainlink_poller(cache: Arc<PriceCache>, price_tx: broadcast::Sender<serde_json::Value>, rpc_url: String, feeds: Vec<(String, String)>, interval_secs: u64, db: Option<DbPool>) {
    // Run chainlink polling on a dedicated current-thread runtime to avoid requiring Send on ethers futures
    let rpc_clone = rpc_url.clone();
    let feeds_clone = feeds.clone();
    let cache_clone = cache.clone();
    let price_tx_clone = price_tx.clone();
    let db_clone = db.clone();
    let _jh = tokio::task::spawn_blocking(move || {
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().expect("failed to build rt");
        rt.block_on(async move {
            let provider = match Provider::<Http>::try_from(rpc_clone.clone()) {
                Ok(p) => p,
                Err(_) => return,
            };

            let max_dev_pct: f64 = std::env::var("PRICE_MAX_DEVIATION_PERC").ok().and_then(|s| s.parse().ok()).unwrap_or(5.0);

            loop {
                for (key, addr_str) in &feeds_clone {
                    if let Ok(addr) = addr_str.parse::<Address>() {
                        let contract = AggregatorV3Interface::new(addr, provider.clone().into());
                        // Read decimals and latestRoundData
                        let decimals_res = contract.decimals().await;
                        let latest_res = contract.latest_round_data().await;
                        if let (Ok(dec), Ok((_round, answer, _start, _updated_at, _ansround))) = (decimals_res, latest_res) {
                            if answer > ethers::types::I256::from(0) || answer < ethers::types::I256::from(0) {
                                let ans_i128: i128 = answer.as_i128();
                                let scale = 10i128.pow(dec as u32) as f64;
                                let price_f = (ans_i128 as f64) / scale;
                                if let Some(price_dec) = Decimal::from_f64(price_f) {
                                    let parts: Vec<&str> = key.split('_').collect();
                                    if parts.len() == 2 {
                                        let mut info = PriceInfo {
                                            base: parts[0].to_string(),
                                            quote: parts[1].to_string(),
                                            price: price_dec,
                                            fetched_at: Utc::now(),
                                            source: "chainlink".to_string(),
                                            valid: true,
                                        };

                                        // validation against existing coingecko price if present
                                        validate_price_against(&cache_clone, key, &mut info, max_dev_pct);

                                        cache_clone.insert(key.clone(), info.clone());
                                        if let Ok(js) = serde_json::to_value(&info) {
                                            let _ = price_tx_clone.send(js);
                                        }
                                        if let Some(pool) = &db_clone {
                                            let _ = repo::insert_price_snapshot(pool, &info.base, &info.quote, info.price, info.fetched_at, Some("chainlink"));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                std::thread::sleep(std::time::Duration::from_secs(interval_secs));
            }
        })
    });
}

pub fn get_price(cache: &PriceCache, base: &str, quote: &str) -> Option<PriceInfo> {
    let key = format!("{}_{}", base, quote);
    cache.get(&key).map(|v| v.clone())
}

/// Validate `info` against an existing cached price (if present).
/// Sets `info.valid = false` when deviation exceeds `max_dev_pct`.
pub fn validate_price_against(cache: &PriceCache, key: &str, info: &mut PriceInfo, max_dev_pct: f64) {
    if let Some(existing) = cache.get(key) {
        let other = existing.value();
        if other.source != info.source {
            if let (Some(a), Some(b)) = (info.price.to_f64(), other.price.to_f64()) {
                let diff = (a - b).abs();
                let pct = if b != 0.0 { diff / b * 100.0 } else { 0.0 };
                if pct > max_dev_pct {
                    info.valid = false;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dashmap::DashMap;
    use rust_decimal::Decimal;
    use rust_decimal::prelude::FromPrimitive;

    #[test]
    fn validation_marks_invalid_when_deviation_high() {
        let cache: PriceCache = DashMap::new();
        let key = "BTC_USD".to_string();
        let existing = PriceInfo { base: "BTC".into(), quote: "USD".into(), price: Decimal::from_f64(100.0).unwrap(), fetched_at: Utc::now(), source: "chainlink".into(), valid: true };
        cache.insert(key.clone(), existing);

        let mut incoming = PriceInfo { base: "BTC".into(), quote: "USD".into(), price: Decimal::from_f64(200.0).unwrap(), fetched_at: Utc::now(), source: "coingecko".into(), valid: true };
        validate_price_against(&cache, &key, &mut incoming, 5.0);
        assert!(!incoming.valid, "incoming should be invalid due to large deviation");
    }

    #[test]
    fn validation_keeps_valid_when_close() {
        let cache: PriceCache = DashMap::new();
        let key = "BTC_USD".to_string();
        let existing = PriceInfo { base: "BTC".into(), quote: "USD".into(), price: Decimal::from_f64(100.0).unwrap(), fetched_at: Utc::now(), source: "chainlink".into(), valid: true };
        cache.insert(key.clone(), existing);

        let mut incoming = PriceInfo { base: "BTC".into(), quote: "USD".into(), price: Decimal::from_f64(101.0).unwrap(), fetched_at: Utc::now(), source: "coingecko".into(), valid: true };
        validate_price_against(&cache, &key, &mut incoming, 5.0);
        assert!(incoming.valid, "incoming should remain valid when within threshold");
    }
}
