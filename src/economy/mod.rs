use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GoodType {
    CLOTHING = 0,
    COAL = 1,
    COTTON = 2,
    FOOD = 3,
    IRON = 4,
    KILOWATT = 5,
    LABOURHOUR = 6,
    MACHINE = 7,
    REALESTATE = 8,
    WHEAT = 9,
}

pub const GOOD_TYPE_COUNT: usize = 10;

impl GoodType {
    pub fn all() -> [GoodType; GOOD_TYPE_COUNT] {
        [
            GoodType::CLOTHING,
            GoodType::COAL,
            GoodType::COTTON,
            GoodType::FOOD,
            GoodType::IRON,
            GoodType::KILOWATT,
            GoodType::LABOURHOUR,
            GoodType::MACHINE,
            GoodType::REALESTATE,
            GoodType::WHEAT,
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Inventory {
    pub goods: [f64; GOOD_TYPE_COUNT],
}

impl Inventory {
    pub fn new() -> Self {
        Self { goods: [0.0; GOOD_TYPE_COUNT] }
    }

    pub fn get(&self, good: GoodType) -> f64 {
        self.goods[good as usize]
    }

    pub fn set(&mut self, good: GoodType, amount: f64) {
        self.goods[good as usize] = amount;
    }

    pub fn add(&mut self, good: GoodType, amount: f64) {
        self.goods[good as usize] += amount;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BrainType {
    Household,
    Factory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductionFunction {
    pub coefficient: f64,
    pub exponents: [f64; GOOD_TYPE_COUNT],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UtilityFunction {
    pub exponents: [f64; GOOD_TYPE_COUNT],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingStrategy {
    pub current_price: f64,
    pub price_change_increment: f64,
    pub last_offered: f64,
    pub last_sold: f64,
    pub history_prices: [f64; 3],
    pub history_offered: [f64; 3],
    pub history_sold: [f64; 3],
}

impl PricingStrategy {
    pub fn new(initial_price: f64) -> Self {
        Self {
            current_price: initial_price,
            price_change_increment: 0.05,
            last_offered: 0.0,
            last_sold: 0.0,
            history_prices: [initial_price; 3],
            history_offered: [0.0; 3],
            history_sold: [0.0; 3],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeoclassicalBrain {
    pub brain_type: BrainType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HouseholdState {
    pub ticks_since_utility_met: u32,
    pub age_ticks: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactoryState {
    pub produced_good: GoodType,
}
