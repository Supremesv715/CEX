pub mod user;
pub mod trade;
pub mod matching_engine;
pub mod api;
pub mod db;
pub mod repo;
pub mod price_feed;

use matching_engine::engine::{TradingPair, Exchange};
use db::init_db;
use rust_decimal_macros::dec;
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;
use user::User;
use std::collections::HashMap;
use uuid::Uuid;
use crate::repo::{load_users, load_balances, load_open_orders};
use matching_engine::orderbook::{Order, BidOrAsk};
use rust_decimal::Decimal;
use std::sync::Arc as StdArc;
use crate::price_feed::{PriceCache, start_coingecko_poller, start_chainlink_poller};
use serde_json::Value as JsonValue;

#[tokio::main]
async fn main() {
    println!("--- Centralized Exchange Engine Initializing ---");

    dotenvy::dotenv().ok();

    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://postgres:password@localhost:5432/engine_dev".to_string()
    });

    println!("Using DATABASE_URL={}", database_url);

    let pool = init_db(&database_url).await.expect("failed to init db");

    let mut exchange = Exchange::new();
    let btc_usd = TradingPair::new("BTC".to_string(), "USD".to_string());
    exchange.add_new_market(btc_usd.clone());

    

    let users = load_users(&pool).await.unwrap_or_default();
    if users.is_empty() {
        

        let mut bob = User::new();
        bob.deposit("BTC", dec!(100.0));
        exchange.add_user(bob.clone());
        println!("Seeded Bob (Market Maker) with 100 BTC. ID: {}", bob.id);
    } else {

        

        let mut user_map: HashMap<Uuid, User> = HashMap::new();
        for u in &users {
            let user = User { id: u.id, balances: HashMap::new() };
            user_map.insert(u.id, user);
        }

        

        let balances = load_balances(&pool).await.unwrap_or_default();
        for b in &balances {
            if let Some(user) = user_map.get_mut(&b.user_id) {
                user.balances.insert(b.asset.clone(), crate::user::Balance { available: b.available, locked: b.locked });
            }
        }

        

        for (_id, user) in user_map.into_iter() {
            exchange.add_user(user);
        }

        

        let open_orders = load_open_orders(&pool).await.unwrap_or_default();
        for o in &open_orders {
            

            if o.order_type == "limit" {
                let bid_or_ask = if o.side == "buy" { BidOrAsk::Bid } else { BidOrAsk::Ask };
                let ord = Order { id: o.id, user_id: o.user_id, size: o.amount, bid_or_ask, time_in_force: crate::matching_engine::orderbook::TimeInForce::GTC };
                

                let price: Decimal = o.price.unwrap_or_else(|| Decimal::new(0, 0));

                

                let pair = if let Some(mkt) = &o.market {
                    

                    let parts: Vec<&str> = mkt.split('_').collect();
                    if parts.len() == 2 {
                        TradingPair::new(parts[0].to_string(), parts[1].to_string())
                    } else {
                        btc_usd.clone()
                    }
                } else {
                    btc_usd.clone()
                };

                

                if !exchange.has_market(&pair) {
                    exchange.add_new_market(pair.clone());
                }

                if let Err(e) = exchange.add_limit_order_to_book(pair, price, ord) {
                    println!("Failed to restore order {}: {}", o.id, e);
                }
            }
        }
        println!("Restored {} users and {} open orders from DB", users.len(), open_orders.len());
    }

    let state = Arc::new(Mutex::new(exchange));

    let (tx, _rx) = tokio::sync::broadcast::channel(1024);

    let price_cache: StdArc<PriceCache> = StdArc::new(PriceCache::new());
    let (price_tx, _price_rx) = tokio::sync::broadcast::channel::<JsonValue>(1024);

    
    start_coingecko_poller(price_cache.clone(), price_tx.clone(), 15, Some(pool.clone())).await;

    
    if let Ok(rpc) = std::env::var("CHAINLINK_RPC") {
        if let Ok(feeds_str) = std::env::var("CHAINLINK_FEEDS") {
            
            let mut feeds = Vec::new();
            for entry in feeds_str.split(',') {
                if let Some((k, v)) = entry.split_once(':') {
                    feeds.push((k.to_string(), v.to_string()));
                }
            }
            if !feeds.is_empty() {
                let feeds_len = feeds.len();
                start_chainlink_poller(price_cache.clone(), price_tx.clone(), rpc, feeds.clone(), 15, Some(pool.clone())).await;
                println!("Chainlink poller started with {} feeds", feeds_len);
            }
        }
    }

    let app_state = api::AppState { exchange: state.clone(), db: pool.clone(), tx, price_cache, price_tx };

    let app = api::router(app_state);

    let addr = "0.0.0.0:3000";
    println!("API Server running at http://{}", addr);

    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}