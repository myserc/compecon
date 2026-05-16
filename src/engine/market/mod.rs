use std::collections::BTreeMap;
use hecs::Entity;
use crate::economy::{GoodType, GOOD_TYPE_COUNT};

#[derive(Debug, Clone)]
pub struct SellOrder {
    pub seller: Entity,
    pub amount: f64,
    pub price_pv: u64,
}

#[derive(Default, Debug, Clone)]
pub struct OrderBook {
    pub sell_orders: BTreeMap<u64, Vec<SellOrder>>,
}

#[derive(Debug, Clone)]
pub struct Market {
    pub books: Vec<OrderBook>,
}

impl Market {
    pub fn new() -> Self {
        let mut books = Vec::with_capacity(GOOD_TYPE_COUNT);
        for _ in 0..GOOD_TYPE_COUNT {
            books.push(OrderBook::default());
        }
        Self { books }
    }

    pub fn get_marginal_price(&self, good: GoodType) -> Option<u64> {
        let book = &self.books[good as usize];
        book.sell_orders.keys().next().copied()
    }

    pub fn clear(&mut self) {
        for book in &mut self.books {
            book.sell_orders.clear();
        }
    }

    pub fn get_marginal_price_at_volume(&self, good: GoodType, volume: f64) -> f64 {
        let book = &self.books[good as usize];
        let mut remaining = volume;
        let mut last_price = 0.0;

        if book.sell_orders.is_empty() {
            return 0.0;
        }

        for (&price, orders) in &book.sell_orders {
            last_price = price as f64;
            for order in orders {
                remaining -= order.amount;
                if remaining <= 1e-9 {
                    return price as f64;
                }
            }
        }
        last_price
    }
}
