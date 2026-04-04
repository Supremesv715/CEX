use axum::{
    extract::{State, Path, ws::{WebSocketUpgrade, WebSocket, Message as WsMsg}},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post, delete},
    Json, Router,
};
use futures_util::{StreamExt, SinkExt};
use tokio::sync::broadcast;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::matching_engine::engine::{Exchange, TradingPair};
use crate::matching_engine::orderbook::{Order, BidOrAsk, TimeInForce};
use crate::user::User;
use crate::db::DbPool;
use crate::repo;
use crate::trade::Trade as TradeModel;

#[derive(Clone, Serialize)]
pub enum WsMessage {
    Trade(TradeModel),
    OrderPlaced(Order),
    OrderCancelled { order_id: Uuid },
}

#[derive(Clone)]
pub struct AppState {
    pub exchange: Arc<Mutex<Exchange>>,
    pub db: DbPool,
    pub tx: broadcast::Sender<WsMessage>,
}

#[derive(Deserialize)]
pub struct CreateUserRequest {
    pub initial_funds: Decimal,
}

#[derive(Serialize)]
pub struct CreateUserResponse {
    pub user_id: Uuid,
}

#[derive(Deserialize)]
pub struct PlaceOrderRequest {
    pub user_id: Uuid,
    pub base: String,
    pub quote: String,
    pub price: Decimal,
    pub size: Decimal,
    pub bid_or_ask: BidOrAsk,
    pub order_type: String, 

    pub time_in_force: Option<String>,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/v1/users", post(create_user))
        .route("/api/v1/users/:user_id/balances", get(get_balances))
        .route("/api/v1/orders", post(place_order))
        .route("/api/v1/users/:user_id/orders", get(list_open_orders))
        .route("/api/v1/orders/:order_id", delete(cancel_order))
        .route("/api/v1/market/:base/:quote/orderbook", get(get_orderbook))
        .route("/api/v1/ws", get(ws_handler))
        .with_state(state)
}

async fn create_user(
    State(state): State<AppState>,
    Json(payload): Json<CreateUserRequest>,
) -> impl IntoResponse {
    let mut exchange = state.exchange.lock().unwrap();
    let mut user = User::new();
    user.deposit("USD", payload.initial_funds);
    user.deposit("BTC", Decimal::from(10));
    let user_id = user.id;
    exchange.add_user(user.clone());

    

    let db = state.db.clone();
    let uid = user_id;
    let initial = payload.initial_funds;
    tokio::spawn(async move {
        let _ = sqlx::query("INSERT INTO users (id, username, metadata) VALUES ($1, $2, $3)")
            .bind(uid)
            .bind(uid.to_string())
            .bind(serde_json::Value::Null)
            .execute(&db)
            .await;

        let _ = sqlx::query("INSERT INTO balances (user_id, asset, available, locked) VALUES ($1, $2, $3, $4)")
            .bind(uid)
            .bind("USD")
            .bind(initial)
            .bind(0_i32)
            .execute(&db)
            .await;
            
        let _ = sqlx::query("INSERT INTO balances (user_id, asset, available, locked) VALUES ($1, $2, $3, $4)")
            .bind(uid)
            .bind("BTC")
            .bind(Decimal::from(10))
            .bind(0_i32)
            .execute(&db)
            .await;
    });

    (StatusCode::CREATED, Json(CreateUserResponse { user_id }))
}

async fn get_balances(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
) -> impl IntoResponse {
    let exchange = state.exchange.lock().unwrap();
    if let Some(user) = exchange.get_user(&user_id) {
        (StatusCode::OK, Json(user.balances.clone())).into_response()
    } else {
        (StatusCode::NOT_FOUND, "User not found").into_response()
    }
}

async fn place_order(
    State(state): State<AppState>,
    Json(payload): Json<PlaceOrderRequest>,
) -> impl IntoResponse {
    let pair = TradingPair::new(payload.base, payload.quote);
    let mut order = Order::new(payload.bid_or_ask, payload.size, payload.user_id);
    if let Some(tif) = &payload.time_in_force {
        let tif_enum = match tif.as_str() {
            "IOC" => TimeInForce::IOC,
            "FOK" => TimeInForce::FOK,
            _ => TimeInForce::GTC,
        };
        order = order.with_tif(tif_enum);
    }

    if payload.order_type == "limit" {
        

        let cost = match order.bid_or_ask {
            BidOrAsk::Bid => payload.price * order.size,
            BidOrAsk::Ask => order.size,
        };

        let asset = match order.bid_or_ask {
            BidOrAsk::Bid => &pair.quote,
            BidOrAsk::Ask => &pair.base,
        };

        

        match repo::lock_balance(&state.db, order.user_id, asset, cost).await {
            Ok((new_avail, new_locked)) => {
                

                if let Err(_e) = repo::insert_order(&state.db, order.id, order.user_id, if matches!(order.bid_or_ask, BidOrAsk::Bid) { "buy" } else { "sell" }, "limit", "open", Some(payload.price), payload.size).await {
                    

                    let _ = repo::upsert_balance(&state.db, order.user_id, asset, new_avail + cost, new_locked - cost).await;
                    return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to persist order").into_response();
                }

                

                let _ = repo::insert_ledger(&state.db, order.user_id, asset, -cost, new_avail + cost, new_avail, "place_limit_order", Some(order.id)).await;

                

                let mut exchange = state.exchange.lock().unwrap();
                exchange.reflect_locked_funds(&order.user_id, asset, cost);
                match exchange.add_limit_order_to_book(pair, payload.price, order.clone()) {
                    Ok(_) => {
                        let _ = state.tx.send(WsMessage::OrderPlaced(order));
                        (StatusCode::OK, "Limit order placed successfully").into_response()
                    },
                    Err(e) => (StatusCode::BAD_REQUEST, e).into_response(),
                }
            }
            Err(e) => (StatusCode::BAD_REQUEST, e).into_response(),
        }
    } else if payload.order_type == "market" {
        let trades_result = {
            let mut exchange = state.exchange.lock().unwrap();
            exchange.place_market_order(pair.clone(), order.clone())
        };

        match trades_result {
            Ok(trades) => {
                let db = state.db.clone();
                let trades_to_persist = trades.clone();
                let tx = state.tx.clone();
                tokio::spawn(async move {
                    for t in trades_to_persist {
                        

                        let _ = repo::insert_trade(&db, t.id, t.buy_order_id, t.sell_order_id, t.price, t.quantity).await;
                        
                        let _ = tx.send(WsMessage::Trade(t.clone()));

                        

                        if let Some(buy_ord) = t.buy_order_id {
                            let _ = repo::increment_order_filled(&db, buy_ord, t.quantity).await;
                        }
                        if let Some(sell_ord) = t.sell_order_id {
                            let _ = repo::increment_order_filled(&db, sell_ord, t.quantity).await;
                        }
                    }
                });
                (StatusCode::OK, Json(trades)).into_response()
            }
            Err(e) => (StatusCode::BAD_REQUEST, e).into_response(),
        }
    } else {
        (StatusCode::BAD_REQUEST, "Invalid order type").into_response()
    }
}

async fn list_open_orders(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
) -> impl IntoResponse {
    match repo::load_open_orders(&state.db).await {
        Ok(orders) => {
            let user_orders: Vec<_> = orders.into_iter().filter(|o| o.user_id == user_id).collect();
            (StatusCode::OK, Json(user_orders)).into_response()
        }
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Failed to load orders").into_response(),
    }
}

async fn cancel_order(
    State(state): State<AppState>,
    Path(order_id): Path<Uuid>,
) -> impl IntoResponse {
    

    let order_row = match repo::get_order(&state.db, order_id).await {
        Ok(Some(o)) => o,
        Ok(None) => return (StatusCode::NOT_FOUND, "Order not found in db").into_response(),
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "DB Error").into_response(),
    };

    let market_str = order_row.market.unwrap_or_else(|| "BTC_USD".to_string());
    let parts: Vec<&str> = market_str.split('_').collect();
    let pair = TradingPair::new(parts[0].to_string(), parts[1].to_string());

    let mut exchange = state.exchange.lock().unwrap();
    match exchange.cancel_order(&pair, order_id) {
        Ok(_) => {
            

            let db = state.db.clone();
            let tx = state.tx.clone();
            tokio::spawn(async move {
                let _ = repo::update_order_status(&db, order_id, "cancelled").await;
                let _ = tx.send(WsMessage::OrderCancelled { order_id });
            });
            (StatusCode::OK, "Order cancelled").into_response()
        }
        Err(e) => (StatusCode::BAD_REQUEST, e).into_response(),
    }
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    let (mut sender, _) = socket.split();
    let mut rx = state.tx.subscribe();

    tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if let Ok(json) = serde_json::to_string(&msg) {
                if sender.send(WsMsg::Text(json)).await.is_err() {
                    break;
                }
            }
        }
    });
}

#[derive(Serialize)]
pub struct OrderbookResponse {
    pub bids: Vec<(Decimal, Decimal)>,
    pub asks: Vec<(Decimal, Decimal)>,
}

async fn get_orderbook(
    State(state): State<AppState>,
    Path((base, quote)): Path<(String, String)>,
) -> impl IntoResponse {
    let pair = TradingPair::new(base, quote);
    let exchange = state.exchange.lock().unwrap();
    if let Some((bids, asks)) = exchange.get_orderbook_depth(&pair) {
        (StatusCode::OK, Json(OrderbookResponse { bids, asks })).into_response()
    } else {
        (StatusCode::NOT_FOUND, "Orderbook not found").into_response()
    }
}

