use serde::{Serialize, Deserialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GoodType {
    LABOURHOUR,
    WHEAT,
    COAL,
    IRON,
    BREAD,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Inventory {
    pub goods: HashMap<GoodType, f64>,
}

impl Inventory {
    pub fn new() -> Self {
        Self { goods: HashMap::new() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BrainType {
    Household,
    Factory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeoclassicalBrain {
    pub brain_type: BrainType,
    pub price_expectations: HashMap<GoodType, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HouseholdState {
    pub ticks_since_utility_met: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactoryState {
    pub produced_good: GoodType,
}
