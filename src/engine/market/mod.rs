use std::collections::BTreeMap;
use hecs::Entity;
use ordered_float::OrderedFloat;
use crate::economy::{GoodType, GOOD_TYPE_COUNT};

#[derive(Debug, Clone)]
pub struct SellOrder {
    pub seller: Entity,
    pub amount: f64,
    pub price_f64: f64,
}

pub struct Transaction {
    pub seller: Entity,
    pub amount: f64,
    pub price: f64,
}

#[derive(Default, Debug, Clone)]
pub struct MarketBook {
    pub asks: BTreeMap<OrderedFloat<f64>, Vec<SellOrder>>,
}

impl MarketBook {
    /// Port of Java's findBestFulfillmentSet / execute_buy
    pub fn execute_buy(&mut self, _buyer: Entity, max_budget_pv: u64, max_price: f64, max_amount: f64) -> (f64, u64, Vec<Transaction>) {
        let mut budget_left_f64 = max_budget_pv as f64;
        let mut amount_acquired = 0.0;
        let mut total_spent_f64 = 0.0;
        let mut transactions = Vec::new();

        // Iterate orders lowest price first
        let prices: Vec<OrderedFloat<f64>> = self.asks.keys().cloned().collect();

        for price in prices {
            if *price > max_price || budget_left_f64 <= 0.0 || amount_acquired >= max_amount {
                break;
            }

            if let Some(orders) = self.asks.get_mut(&price) {
                for order in orders.iter_mut() {
                    let remaining_needed = max_amount - amount_acquired;
                    let can_afford = budget_left_f64 / *price;

                    let fill = f64::min(remaining_needed, f64::min(order.amount, can_afford));

                    if fill > 1e-9 {
                        order.amount -= fill;
                        amount_acquired += fill;
                        let cost = fill * (*price);
                        budget_left_f64 -= cost;
                        total_spent_f64 += cost;

                        transactions.push(Transaction {
                            seller: order.seller,
                            amount: fill,
                            price: *price,
                        });
                    }

                    if amount_acquired >= max_amount || budget_left_f64 <= 0.0 {
                        break;
                    }
                }
                orders.retain(|o| o.amount > 1e-9);
            }

            if self.asks.get(&price).map_or(false, |v| v.is_empty()) {
                self.asks.remove(&price);
            }

            if amount_acquired >= max_amount || budget_left_f64 <= 0.0 {
                break;
            }
        }

        (amount_acquired, total_spent_f64.round() as u64, transactions)
    }
}

#[derive(Debug, Clone)]
pub struct Market {
    pub books: Vec<MarketBook>,
}

impl Market {
    pub fn new() -> Self {
        let mut books = Vec::with_capacity(GOOD_TYPE_COUNT);
        for _ in 0..GOOD_TYPE_COUNT {
            books.push(MarketBook::default());
        }
        Self { books }
    }

    pub fn get_marginal_price(&self, good: GoodType) -> Option<f64> {
        let book = &self.books[good as usize];
        book.asks.keys().next().map(|p| **p)
    }

    pub fn clear(&mut self) {
        for book in &mut self.books {
            book.asks.clear();
        }
    }

    pub fn get_marginal_price_at_volume(&self, good: GoodType, volume: f64) -> f64 {
        let book = &self.books[good as usize];
        let mut remaining = volume;
        let mut last_price = 0.0;

        if book.asks.is_empty() {
            return 0.0;
        }

        for (price, orders) in &book.asks {
            last_price = **price;
            for order in orders {
                remaining -= order.amount;
                if remaining <= 1e-9 {
                    return **price;
                }
            }
        }
        last_price
    }
}
