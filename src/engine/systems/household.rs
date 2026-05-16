use hecs::World;
use crate::arithmodynamics::ArithmodynamicNode;
use crate::economy::{Inventory, HouseholdState, UtilityFunction, GoodType};
use crate::engine::intents::{BuyIntent, SellIntent, ConsumptionIntent};
use crossbeam::channel::Sender;
use rayon::prelude::*;

pub fn household_system(
    world: &World,
    tx_buy: &Sender<BuyIntent>,
    tx_sell: &Sender<SellIntent>,
    tx_cons: &Sender<ConsumptionIntent>,
    market: &crate::engine::market::Market,
) {
    let mut evals = Vec::new();
    for (entity, (node, inv, h_state, util_fn)) in world.query::<(&ArithmodynamicNode, &Inventory, &HouseholdState, &UtilityFunction)>().iter() {
        evals.push((entity, node.clone(), inv.clone(), h_state.clone(), util_fn.clone()));
    }

    evals.par_iter().for_each_with((tx_buy.clone(), tx_sell.clone(), tx_cons.clone()), |(tx_b, tx_s, tx_c), (entity, node, inv, _h_state, util_fn)| {
        // 1. Supply Labour
        tx_s.send(SellIntent {
            seller: *entity,
            good: GoodType::LABOURHOUR,
            amount: 8.0,
            price_pv: 10, // Base wage expectation
        }).unwrap();

        // 2. Consumption Decision (Modigliani-ish)
        let budget = (node.prime_value as f64 * 0.1).max(5.0); // Spend 10% of savings

        let mut prices = [None; crate::economy::GOOD_TYPE_COUNT];
        for good in GoodType::all() {
            prices[good as usize] = market.get_marginal_price(good);
        }

        let optimal_bundle = crate::math::optimize::calculate_optimal_basket(budget, &prices, &util_fn.exponents);

        for (i, &amount) in optimal_bundle.iter().enumerate() {
            let good = GoodType::all()[i];
            if amount > 0.0 && good != GoodType::LABOURHOUR {
                tx_b.send(BuyIntent {
                    buyer: *entity,
                    good,
                    max_amount: amount,
                    max_price_pv: (prices[i].unwrap_or(10.0) * 1.5) as u64,
                }).unwrap();
            }
        }

        // 3. Consume existing inventory
        let mut consumed = Vec::new();
        let mut inputs_val = [0.0; crate::economy::GOOD_TYPE_COUNT];
        for g in GoodType::all() {
            let amount = inv.get(g);
            if amount > 0.0 && g != GoodType::LABOURHOUR {
                let to_consume = amount * 0.5; // Consume half of inventory
                consumed.push((g, to_consume));
                inputs_val[g as usize] = to_consume;
            }
        }

        let utility = crate::math::cobb_douglas(&inputs_val, &util_fn.exponents, 1.0);
        if utility > 0.0 {
            tx_c.send(ConsumptionIntent {
                household: *entity,
                utility,
                consumed,
            }).unwrap();
        }
    });
}
