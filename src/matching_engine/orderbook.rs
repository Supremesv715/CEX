use std::collections::HashMap;
use rust_decimal::prelude::*;
use rust_decimal_macros::dec;
use uuid::Uuid;
use serde::{Serialize, Deserialize};
use crate::trade::Trade;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum BidOrAsk{
    Bid,
    Ask,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum TimeInForce {
    GTC, 

    IOC, 

    FOK, 

}

impl Default for TimeInForce {
    fn default() -> Self {
        TimeInForce::GTC
    }
}

#[derive(Debug)]
pub struct Orderbook{
    asks: HashMap<Decimal, Limit>,
    bids: HashMap<Decimal, Limit>,
}

impl Orderbook{
    pub fn new()->Orderbook{
        Orderbook { 
            asks:HashMap::new(),
            bids:HashMap::new(),
         }
    }
    
    pub fn fill_market_order(&mut self, market_order: &mut Order) -> Vec<Trade> {
        let mut trades = Vec::new();
        
        if market_order.time_in_force == TimeInForce::FOK {
            let mut available_vol = dec!(0.0);
            match market_order.bid_or_ask {
                BidOrAsk::Bid => {
                    for limit in self.asks.values() {
                        available_vol += limit.total_volume();
                    }
                },
                BidOrAsk::Ask => {
                    for limit in self.bids.values() {
                        available_vol += limit.total_volume();
                    }
                }
            }
            if available_vol < market_order.size {
                return trades;
            }
        }
        
        {
            let limits=match market_order.bid_or_ask{
                BidOrAsk::Bid=>self.ask_limits(),
                BidOrAsk::Ask=>self.bids_limits()
            };
            
            for limit_order in limits{
                trades.extend(limit_order.fill_order(market_order));
                if market_order.is_filled(){
                    break;
                }
            }
        }
        
        

        self.asks.retain(|_, limit| !limit.orders.is_empty());
        self.bids.retain(|_, limit| !limit.orders.is_empty());
        
        trades
    }
    
    pub fn cancel_order(&mut self, order_id: Uuid) -> Option<(Decimal, Order)> {
        for limit in self.bids.values_mut() {
            if let Some(pos) = limit.orders.iter().position(|o| o.id == order_id) {
                let o = limit.orders.remove(pos);
                return Some((limit.price, o));
            }
        }
        for limit in self.asks.values_mut() {
            if let Some(pos) = limit.orders.iter().position(|o| o.id == order_id) {
                let o = limit.orders.remove(pos);
                return Some((limit.price, o));
            }
        }
        None
    }
        
    pub fn get_depth(&self) -> (Vec<(Decimal, Decimal)>, Vec<(Decimal, Decimal)>) {
        let mut bids_depth: Vec<(Decimal, Decimal)> = self.bids.iter().map(|(&price, limit)| (price, limit.total_volume())).collect();
        bids_depth.sort_by(|a, b| b.0.cmp(&a.0));

        let mut asks_depth: Vec<(Decimal, Decimal)> = self.asks.iter().map(|(&price, limit)| (price, limit.total_volume())).collect();
        asks_depth.sort_by(|a, b| a.0.cmp(&b.0));
        
        (bids_depth, asks_depth)
    }

    pub fn ask_limits(&mut self)-> Vec<&mut Limit>{
        let mut limits =self.asks.values_mut().collect::<Vec<&mut Limit>>();
        limits.sort_by(|a, b| a.price.cmp(&b.price));
        limits
    }
    
    pub fn estimate_market_buy_cost(&self, mut size: Decimal) -> Decimal {
        let mut total_cost = dec!(0.0);
        let mut limits: Vec<&Limit> = self.asks.values().collect();
        limits.sort_by(|a, b| a.price.cmp(&b.price));
        
        for limit in limits {
            let limit_vol = limit.total_volume();
            if size >= limit_vol {
                total_cost += limit_vol * limit.price;
                size -= limit_vol;
            } else {
                total_cost += size * limit.price;
                size = dec!(0.0);
            }
            if size.is_zero() {
                break;
            }
        }
        total_cost
    }
    
    pub fn bids_limits(&mut self)-> Vec<&mut Limit>{
        let mut limits=self.bids.values_mut().collect::<Vec<&mut Limit>>();
        limits.sort_by(|a, b| b.price.cmp(&a.price));
        limits
    }
    
    pub fn add_limit_order(&mut self, price: Decimal, order: Order) {
        match order.bid_or_ask{
            BidOrAsk::Bid=>match self.bids.get_mut(&price){
                Some(limit)=> limit.add_orders(order),
                None=>{
                    let mut limit=Limit::new(price);
                    limit.add_orders(order);
                    self.bids.insert(price, limit);
                }
            },
            BidOrAsk::Ask=>match self.asks.get_mut(&price){
                Some(limit)=> limit.add_orders(order),
                None=>{
                    let mut limit=Limit::new(price);
                    limit.add_orders(order);
                    self.asks.insert(price, limit);
                }
            },
        }
    }
}

#[derive(Debug)]
pub struct Limit{
    price:Decimal,
    pub orders:Vec<Order>,
}

impl Limit {
    pub fn new(price:Decimal)-> Limit{
        Limit {
            price,
            orders:Vec::new(),
        }
    }
    
    pub fn total_volume(&self)-> Decimal{
        self.orders.iter().map(|order| order.size).sum()
    }
    
    fn fill_order(&mut self, market_order: &mut Order) -> Vec<Trade> {
        let mut trades = Vec::new();
        
        for limit_order in self.orders.iter_mut(){
            let match_size = if market_order.size >= limit_order.size {
                limit_order.size
            } else {
                market_order.size
            };
            
            let (buyer_id, seller_id) = match market_order.bid_or_ask {
                BidOrAsk::Bid => (market_order.user_id, limit_order.user_id),
                BidOrAsk::Ask => (limit_order.user_id, market_order.user_id),
            };
            
            

            let buy_ord_id = if market_order.bid_or_ask == BidOrAsk::Bid {
                Some(market_order.id)
            } else {
                Some(limit_order.id)
            };
            let sell_ord_id = if market_order.bid_or_ask == BidOrAsk::Ask {
                Some(market_order.id)
            } else {
                Some(limit_order.id)
            };

            trades.push(Trade::new(
                self.price,
                match_size,
                buyer_id,
                seller_id,
                buy_ord_id,
                sell_ord_id,
            ));
            
            market_order.size -= match_size;
            limit_order.size -= match_size;
            
            if market_order.is_filled(){
                break;
            }
        }
        
        

        self.orders.retain(|o| !o.is_filled());
        
        trades
    }
    
    pub fn add_orders(&mut self,order:Order){
        self.orders.push(order);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub id: Uuid,
    pub user_id: Uuid,
    pub size: Decimal,
    pub bid_or_ask: BidOrAsk,
    pub time_in_force: TimeInForce,
}

impl Order {
    pub fn new(bid_or_ask: BidOrAsk, size: Decimal, user_id: Uuid) -> Order {
        Order {
            id: Uuid::new_v4(),
            user_id,
            bid_or_ask,
            size,
            time_in_force: TimeInForce::GTC,
        }
    }
    
    pub fn with_tif(mut self, tif: TimeInForce) -> Self {
        self.time_in_force = tif;
        self
    }
    
    pub fn is_filled(&self) -> bool {
        self.size.is_zero()
    }
}

#[cfg(test)]
pub mod tests{
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn orderbook_fill_market_order_asks(){
        let mut orderbook=Orderbook::new();
        let u1 = Uuid::new_v4();
        let u2 = Uuid::new_v4();
        orderbook.add_limit_order(dec!(500), Order::new(BidOrAsk::Ask, dec!(10.0), u1));
        orderbook.add_limit_order(dec!(100), Order::new(BidOrAsk::Ask, dec!(10.0), u1));
        orderbook.add_limit_order(dec!(200), Order::new(BidOrAsk::Ask, dec!(10.0), u1));
        orderbook.add_limit_order(dec!(300), Order::new(BidOrAsk::Ask, dec!(10.0), u1));
        let mut market_order=Order::new(BidOrAsk::Bid, dec!(10.0), u2);
        
        let trades = orderbook.fill_market_order(&mut market_order);
        let ask_limits=orderbook.ask_limits();
        
        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].price, dec!(100));
        assert_eq!(market_order.is_filled(), true);
        
        

        assert_eq!(ask_limits.len(), 3);
    }
    
    #[test]
    fn limit_total_volume(){
        let price=dec!(10000);
        let mut limit=Limit::new(price);
        let u = Uuid::new_v4();

        let buy_limit_order_a=Order::new(BidOrAsk::Bid, dec!(100.0), u);
        let buy_limit_order_b=Order::new(BidOrAsk::Bid, dec!(100.0), u);
        
        limit.add_orders(buy_limit_order_a);
        limit.add_orders(buy_limit_order_b);
        assert_eq!(limit.total_volume(), dec!(200.0));
    }
    
    #[test]
    fn limit_order_multi_fill(){
        let price=dec!(10000.0);
        let mut limit=Limit::new(price);
        let u1 = Uuid::new_v4();
        let u2 = Uuid::new_v4();

        let buy_limit_order_a=Order::new(BidOrAsk::Bid, dec!(100.0), u1);
        let buy_limit_order_b=Order::new(BidOrAsk::Bid, dec!(100.0), u1);
        limit.add_orders(buy_limit_order_a);
        limit.add_orders(buy_limit_order_b);
        
        let mut market_sell_order=Order::new(BidOrAsk::Ask, dec!(199.0), u2);
        let trades = limit.fill_order(&mut market_sell_order);
        
        assert_eq!(market_sell_order.is_filled(), true);
        assert_eq!(trades.len(), 2);
        assert_eq!(limit.orders.len(), 1);
        assert_eq!(limit.orders[0].is_filled(), false);
        assert_eq!(limit.orders[0].size, dec!(1.0));
    }

    #[test]
    fn limit_order_single_fill(){
        let price=dec!(10000.0);
        let mut limit=Limit::new(price);
        let u1 = Uuid::new_v4();
        let u2 = Uuid::new_v4();

        let buy_limit_order=Order::new(BidOrAsk::Bid, dec!(100.0), u1);
        limit.add_orders(buy_limit_order);
        
        let mut market_sell_order=Order::new(BidOrAsk::Ask, dec!(99.0), u2);
        let trades = limit.fill_order(&mut market_sell_order);
        
        assert_eq!(market_sell_order.is_filled(), true);
        assert_eq!(trades.len(), 1);
        assert_eq!(limit.orders[0].size, dec!(1.0));
    }
    
    #[test]
    fn cancel_order() {
        let mut orderbook = Orderbook::new();
        let u1 = Uuid::new_v4();
        let order = Order::new(BidOrAsk::Bid, dec!(10.0), u1);
        let oid = order.id;
        orderbook.add_limit_order(dec!(100), order);
        let cancelled = orderbook.cancel_order(oid);
        assert!(cancelled.is_some());
        assert_eq!(cancelled.unwrap().1.id, oid);
        assert!(orderbook.cancel_order(oid).is_none());
    }
}
