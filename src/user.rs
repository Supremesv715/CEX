use std::collections::HashMap;
use rust_decimal::Decimal;
use uuid::Uuid;
use rust_decimal_macros::dec;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Balance {
    pub available: Decimal,
    pub locked: Decimal,
}

impl Balance {
    pub fn new() -> Self {
        Balance {
            available: dec!(0.0),
            locked: dec!(0.0),
        }
    }

    pub fn deposit(&mut self, amount: Decimal) {
        self.available += amount;
    }

    pub fn lock(&mut self, amount: Decimal) -> Result<(), String> {
        if self.available >= amount {
            self.available -= amount;
            self.locked += amount;
            Ok(())
        } else {
            Err("Insufficient available balance".to_string())
        }
    }

    pub fn unlock(&mut self, amount: Decimal) -> Result<(), String> {
        if self.locked >= amount {
            self.locked -= amount;
            self.available += amount;
            Ok(())
        } else {
            Err("Insufficient locked balance".to_string())
        }
    }
    
    pub fn settle_lock(&mut self, amount: Decimal) -> Result<(), String> {
        if self.locked >= amount {
            self.locked -= amount;
            Ok(())
        } else {
            Err("Insufficient locked balance to settle".to_string())
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub balances: HashMap<String, Balance>,
}

impl User {
    pub fn new() -> Self {
        User {
            id: Uuid::new_v4(),
            balances: HashMap::new(),
        }
    }

    pub fn deposit(&mut self, asset: &str, amount: Decimal) {
        let balance = self.balances.entry(asset.to_string()).or_insert_with(Balance::new);
        balance.deposit(amount);
    }

    pub fn lock_funds(&mut self, asset: &str, amount: Decimal) -> Result<(), String> {
        if let Some(balance) = self.balances.get_mut(asset) {
            balance.lock(amount)
        } else {
            Err(format!("Balance not found for asset {}", asset))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lock_success() {
        let mut user = User::new();
        user.deposit("USD", dec!(100.0));
        assert!(user.lock_funds("USD", dec!(50.0)).is_ok());
        let balance = user.balances.get("USD").unwrap();
        assert_eq!(balance.available, dec!(50.0));
        assert_eq!(balance.locked, dec!(50.0));
    }

    #[test]
    fn test_lock_failure() {
        let mut user = User::new();
        user.deposit("USD", dec!(100.0));
        assert!(user.lock_funds("USD", dec!(150.0)).is_err());
    }
}
