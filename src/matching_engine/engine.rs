use rust_decimal::prelude::*;
use rust_decimal_macros::dec;
use std::collections::HashMap;
use uuid::Uuid;

use crate::matching_engine::orderbook::{Order, Orderbook, BidOrAsk};
use crate::trade::Trade;
use crate::user::User;

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub struct TradingPair {
    pub base: String,
    pub quote: String,
}

impl TradingPair {
    pub fn new(base: String, quote: String) -> TradingPair {
        TradingPair { base, quote }
    }
    
    pub fn to_string(&self) -> String {
        format!("{}_{}", self.base, self.quote)
    }
}

pub struct Exchange {
    orderbooks: HashMap<TradingPair, Orderbook>,
    users: HashMap<Uuid, User>,
    trades: Vec<Trade>,
}

impl Exchange {
    pub fn new() -> Exchange {
        Exchange {
            orderbooks: HashMap::new(),
            users: HashMap::new(),
            trades: Vec::new(),
        }
    }

    

    pub fn reflect_locked_funds(&mut self, user_id: &Uuid, asset: &str, cost: Decimal) {
        if let Some(user) = self.users.get_mut(user_id) {
            let balance = user.balances.entry(asset.to_string()).or_insert_with(crate::user::Balance::new);
            balance.available -= cost;
            balance.locked += cost;
        }
    }

    

    pub fn add_limit_order_to_book(&mut self, pair: TradingPair, price: Decimal, order: Order) -> Result<(), String> {
        match self.orderbooks.get_mut(&pair) {
            Some(orderbook) => {
                orderbook.add_limit_order(price, order);
                Ok(())
            }
            None => Err(format!("the orderbook for the given trading pair ({}) does not exist", pair.to_string())),
        }
    }
    
    pub fn add_new_market(&mut self, pair: TradingPair) {
        self.orderbooks.insert(pair.clone(), Orderbook::new());
        println!("opening new orderbook for market {}", pair.to_string());
    }

    

    pub fn has_market(&self, pair: &TradingPair) -> bool {
        self.orderbooks.contains_key(pair)
    }
    
    pub fn get_orderbook_depth(&self, pair: &TradingPair) -> Option<(Vec<(Decimal, Decimal)>, Vec<(Decimal, Decimal)>)> {
        self.orderbooks.get(pair).map(|ob| ob.get_depth())
    }
    
    pub fn add_user(&mut self, user: User) {
        self.users.insert(user.id, user.clone());
    }
    
    pub fn get_user(&self, id: &Uuid) -> Option<&User> {
        self.users.get(id)
    }
    
    pub fn place_limit_order(&mut self, pair: TradingPair, price: Decimal, order: Order) -> Result<(), String> {
        

        let cost = match order.bid_or_ask {
            BidOrAsk::Bid => price * order.size,
            BidOrAsk::Ask => order.size,
        };
        
        let asset = match order.bid_or_ask {
            BidOrAsk::Bid => &pair.quote,
            BidOrAsk::Ask => &pair.base,
        };
        
        let user = self.users.get_mut(&order.user_id)
            .ok_or_else(|| "User not found".to_string())?;
            
        user.lock_funds(asset, cost)?;
        
        

        match self.orderbooks.get_mut(&pair) {
            Some(orderbook) => {
                orderbook.add_limit_order(price, order);
                println!("placed limit order at price level {}", price);
                Ok(())
            }
            None => Err(format!(
                "the orderbook for the given trading pair ({}) does not exist",
                pair.to_string()
            )),
        }
    }
    
    pub fn place_market_order(&mut self, pair: TradingPair, mut order: Order) -> Result<Vec<Trade>, String> {
        let locked_cost = if matches!(order.bid_or_ask, BidOrAsk::Ask) {
             let user = self.users.get_mut(&order.user_id).ok_or("User not found")?;
             user.lock_funds(&pair.base, order.size)?;
             order.size
        } else {
             let orderbook = self.orderbooks.get(&pair).ok_or("Orderbook not found")?;
             let cost = orderbook.estimate_market_buy_cost(order.size);
             if cost.is_zero() && !order.size.is_zero() {
                 return Err("Insufficient liquidity to fulfill market buy".to_string());
             }
             let user = self.users.get_mut(&order.user_id).ok_or("User not found")?;
             user.lock_funds(&pair.quote, cost)?;
             cost
        };
        
        let trades = match self.orderbooks.get_mut(&pair) {
            Some(orderbook) => orderbook.fill_market_order(&mut order),
            None => return Err("Orderbook not found".to_string())
        };
        
        let mut spent_cost = dec!(0.0);
        

        for trade in &trades {
            spent_cost += match order.bid_or_ask {
                BidOrAsk::Ask => trade.quantity,
                BidOrAsk::Bid => trade.price * trade.quantity,
            };
            self.settle_trade(&pair, trade)?;
        }
        
        let refund = locked_cost - spent_cost;
        if refund > dec!(0.0) {
             let asset = match order.bid_or_ask {
                 BidOrAsk::Ask => &pair.base,
                 BidOrAsk::Bid => &pair.quote,
             };
             let user = self.users.get_mut(&order.user_id).unwrap();
             user.balances.get_mut(asset).unwrap().unlock(refund).unwrap();
        }
        
        

        self.trades.extend(trades.clone());
        
        Ok(trades)
    }
    
    pub fn cancel_order(&mut self, pair: &TradingPair, order_id: Uuid) -> Result<Order, String> {
        let orderbook = self.orderbooks.get_mut(pair).ok_or("Orderbook not found")?;
        
        if let Some((limit_price, order)) = orderbook.cancel_order(order_id) {
            let cost = match order.bid_or_ask {
                BidOrAsk::Ask => order.size,
                BidOrAsk::Bid => order.size * limit_price,
            };
            let asset = match order.bid_or_ask {
                BidOrAsk::Ask => &pair.base,
                BidOrAsk::Bid => &pair.quote,
            };
            if let Some(user) = self.users.get_mut(&order.user_id) {
                if let Some(balance) = user.balances.get_mut(asset) {
                    let _ = balance.unlock(cost);
                }
            }
            Ok(order)
        } else {
            Err("Order not found".to_string())
        }
    }
    
    fn settle_trade(&mut self, pair: &TradingPair, trade: &Trade) -> Result<(), String> {
        let total_cost = trade.price * trade.quantity;
        
        

        if let Some(buyer) = self.users.get_mut(&trade.buyer_id) {
            

            let quote_balance = buyer.balances.entry(pair.quote.clone()).or_insert_with(crate::user::Balance::new);
            if quote_balance.settle_lock(total_cost).is_err() {
                

                

                if quote_balance.available >= total_cost {
                    quote_balance.available -= total_cost;
                } else {
                    return Err(format!("Buyer {} has insufficient funds for settlement", trade.buyer_id));
                }
            }
            buyer.deposit(&pair.base, trade.quantity);
        }
        
        

        if let Some(seller) = self.users.get_mut(&trade.seller_id) {
            let base_balance = seller.balances.entry(pair.base.clone()).or_insert_with(crate::user::Balance::new);
            if base_balance.settle_lock(trade.quantity).is_err() {
                 if base_balance.available >= trade.quantity {
                     base_balance.available -= trade.quantity;
                 } else {
                     return Err(format!("Seller {} has insufficient funds for settlement", trade.seller_id));
                 }
            }
            seller.deposit(&pair.quote, total_cost);
        }
        
        Ok(())
    }
}
