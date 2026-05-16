use hecs::Entity;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AccountType { Transactions, Savings, Dividends, BondLoans }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BankAccount {
    pub owner: Entity,
    pub bank: Entity,
    pub account_type: AccountType,
    pub balance_pv: u64,
    pub overdraft_allowed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixedRateBond {
    pub issuer: Entity,
    pub owner: Entity,
    pub face_value_pv: u64,
    pub coupon_rate: f64,
    pub maturity_tick: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Share {
    pub issuer: Entity,
    pub owner: Entity,
}
