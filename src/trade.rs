use rust_decimal::Decimal;
use uuid::Uuid;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    pub id: Uuid,
    pub price: Decimal,
    pub quantity: Decimal,
    pub buyer_id: Uuid,
    pub seller_id: Uuid,
    pub buy_order_id: Option<Uuid>,
    pub sell_order_id: Option<Uuid>,
}

impl Trade {
    pub fn new(
        price: Decimal,
        quantity: Decimal,
        buyer_id: Uuid,
        seller_id: Uuid,
        buy_order_id: Option<Uuid>,
        sell_order_id: Option<Uuid>,
    ) -> Self {
        Trade {
            id: Uuid::new_v4(),
            price,
            quantity,
            buyer_id,
            seller_id,
            buy_order_id,
            sell_order_id,
        }
    }
}
