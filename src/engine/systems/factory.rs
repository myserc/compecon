use hecs::World;
use crate::arithmodynamics::ArithmodynamicNode;
use crate::economy::{Inventory, FactoryState, ProductionFunction, PricingStrategy, GoodType};
use crate::engine::intents::{BuyIntent, SellIntent, ProductionIntent};
use crossbeam::channel::Sender;
use rayon::prelude::*;

pub fn factory_system(
    world: &World,
    tx_buy: &Sender<BuyIntent>,
    tx_sell: &Sender<SellIntent>,
    tx_prod: &Sender<ProductionIntent>,
    market: &crate::engine::market::Market,
) {
    let mut evals = Vec::new();
    for (entity, (node, inv, f_state, prod_fn, pricing)) in world.query::<(&ArithmodynamicNode, &Inventory, &FactoryState, &ProductionFunction, &PricingStrategy)>().iter() {
        evals.push((entity, node.clone(), inv.clone(), f_state.clone(), prod_fn.clone(), pricing.clone()));
    }

    evals.par_iter().for_each_with((tx_buy.clone(), tx_sell.clone(), tx_prod.clone()), |(tx_b, tx_s, tx_p), (entity, node, inv, f_state, prod_fn, pricing)| {
        // 1. Production Optimization (MC = MR)
        let budget = (node.prime_value as f64 * 0.5).max(50.0);

        let mut prices = [None; crate::economy::GOOD_TYPE_COUNT];
        for good in GoodType::all() {
            prices[good as usize] = market.get_marginal_price(good);
        }

        let optimal_inputs = crate::math::optimize::calculate_optimal_basket(budget, &prices, &prod_fn.exponents);

        for (i, &amount) in optimal_inputs.iter().enumerate() {
            let good = GoodType::all()[i];
            if amount > 0.0 {
                tx_b.send(BuyIntent {
                    buyer: *entity,
                    good,
                    max_amount: amount,
                    max_price_pv: (prices[i].unwrap_or(10.0) * 1.2) as u64,
                }).unwrap();
            }
        }

        // 2. Production if inputs available
        let mut can_produce = true;
        let mut inputs_to_use = Vec::new();
        for (i, &alpha) in prod_fn.exponents.iter().enumerate() {
            if alpha > 0.0 {
                let good = GoodType::all()[i];
                let available = inv.get(good);
                if available < 1.0 { // Minimum threshold
                    can_produce = false;
                    break;
                }
                inputs_to_use.push((good, available * 0.8)); // Use 80% of available
            }
        }

        if can_produce && !inputs_to_use.is_empty() {
            let mut inputs_val = [0.0; crate::economy::GOOD_TYPE_COUNT];
            for (g, a) in &inputs_to_use {
                inputs_val[*g as usize] = *a;
            }
            let output = crate::math::cobb_douglas(&inputs_val, &prod_fn.exponents, prod_fn.coefficient);

            tx_p.send(ProductionIntent {
                factory: *entity,
                inputs: inputs_to_use,
                output: (f_state.produced_good, output),
            }).unwrap();
        }

        // 3. Sell produced goods
        let stock = inv.get(f_state.produced_good);
        if stock > 0.0 {
            tx_s.send(SellIntent {
                seller: *entity,
                good: f_state.produced_good,
                amount: stock,
                price_pv: pricing.current_price as u64,
            }).unwrap();
        }
    });
}
