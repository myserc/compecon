use crate::arithmodynamics::ArithmodynamicNode;
use crate::economy::{Inventory, BankAccount};
use hecs::World;

pub struct DetailedStats {
    pub m0: u64,
    pub m1: u64,
    pub total_inventory_value: f64,
    pub gini: f64,
}

pub fn calculate_statistics(world: &World, market: &crate::engine::market::Market) -> DetailedStats {
    let mut m0 = 0;
    let mut m1 = 0;
    let mut total_inventory_value = 0.0;
    let mut wealths = Vec::new();

    for (_entity, node) in world.query::<&ArithmodynamicNode>().iter() {
        m0 += node.prime_value;
    }

    for (_entity, account) in world.query::<&BankAccount>().iter() {
        m1 += account.balance_pv;
    }

    for (_entity, (node, inv)) in world.query::<(&ArithmodynamicNode, &Inventory)>().iter() {
        let mut inv_val = 0.0;
        for g in crate::economy::GoodType::all() {
            let price = market.get_marginal_price(g).unwrap_or(0.0);
            inv_val += inv.get(g) * price;
        }
        total_inventory_value += inv_val;
        wealths.push(node.prime_value as f64 + inv_val);
    }

    let gini = if !wealths.is_empty() {
        wealths.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let n = wealths.len() as f64;
        let sum_wealth: f64 = wealths.iter().sum();
        if sum_wealth > 0.0 {
            let mut sum_diff = 0.0;
            for (i, w) in wealths.iter().enumerate() {
                sum_diff += (i as f64 + 1.0) * w;
            }
            (2.0 * sum_diff) / (n * sum_wealth) - (n + 1.0) / n
        } else {
            0.0
        }
    } else {
        0.0
    };

    DetailedStats {
        m0,
        m1,
        total_inventory_value,
        gini,
    }
}
