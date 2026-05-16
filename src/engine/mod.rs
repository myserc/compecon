use hecs::World;
use crate::arithmodynamics::{self, ArithmodynamicNode};
use crate::economy::{Inventory, NeoclassicalBrain, HouseholdState, FactoryState, GoodType};
use rayon::prelude::*;
use crossbeam::channel;
use serde::{Serialize, Deserialize};

pub mod market;
pub mod dashboard;

pub mod intents {
    use hecs::Entity;
    use crate::economy::GoodType;

    #[derive(Debug, Clone)]
    pub struct SellIntent {
        pub seller: Entity,
        pub good: GoodType,
        pub amount: f64,
        pub price_pv: u64,
    }

    #[derive(Debug, Clone)]
    pub struct BuyIntent {
        pub buyer: Entity,
        pub good: GoodType,
        pub max_amount: f64,
        pub max_price_pv: u64,
    }

    #[derive(Debug, Clone)]
    pub struct ProductionIntent {
        pub factory: Entity,
        pub inputs: [(GoodType, f64); 2],
        pub output: (GoodType, f64),
    }

    #[derive(Debug, Clone)]
    pub struct ConsumptionIntent {
        pub household: Entity,
        pub utility: f64,
        pub consumed: Vec<(GoodType, f64)>,
    }
}

use self::intents::{SellIntent, BuyIntent, ProductionIntent, ConsumptionIntent};

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct MacroStats {
    pub total_pv: u64,
    pub total_entropy: i64,
    pub agent_count: usize,
    pub m0: u64,
    pub total_utility: f64,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct TimeSystem {
    pub tick: u64,
    pub hour: u32,
    pub day: u32,
    pub month: u32,
    pub year: u32,
}

impl TimeSystem {
    pub fn advance(&mut self) {
        self.tick += 1;
        self.hour = (self.tick % 24) as u32;
        let days = self.tick / 24;
        self.day = (days % 30) as u32;
        let months = days / 30;
        self.month = (months % 12) as u32;
        self.year = (months / 12) as u32;
    }
}

pub struct Simulation {
    pub world: World,
    pub time: TimeSystem,
    pub market: market::Market,
    pub stats: MacroStats,
}

impl Simulation {
    pub fn new() -> Self {
        Self {
            world: World::new(),
            time: TimeSystem::default(),
            market: market::Market::new(),
            stats: MacroStats::default(),
        }
    }

    pub fn tick(&mut self) {
        self.time.advance();
        self.market.clear();

        // Phase 1: Evaluation & Intent (Parallel)
        let (tx_sell, rx_sell) = channel::unbounded::<SellIntent>();
        let (tx_buy, rx_buy) = channel::unbounded::<BuyIntent>();
        let (tx_prod, rx_prod) = channel::unbounded::<ProductionIntent>();
        let (tx_cons, rx_cons) = channel::unbounded::<ConsumptionIntent>();

        let mut factory_evals = Vec::new();
        for (entity, (node, inventory, factory_state, prod_fn, pricing)) in self.world.query::<(&ArithmodynamicNode, &Inventory, &FactoryState, &crate::economy::ProductionFunction, &crate::economy::PricingStrategy)>().iter() {
            factory_evals.push((entity, node.clone(), inventory.clone(), factory_state.clone(), prod_fn.clone(), pricing.clone()));
        }

        let mut household_evals = Vec::new();
        for (entity, (node, inventory, h_state, util_fn)) in self.world.query::<(&ArithmodynamicNode, &Inventory, &HouseholdState, &crate::economy::UtilityFunction)>().iter() {
            household_evals.push((entity, node.clone(), inventory.clone(), h_state.clone(), util_fn.clone()));
        }

        let market_ref = &self.market;

        factory_evals.par_iter().for_each_with((tx_sell.clone(), tx_buy.clone(), tx_prod.clone()), |(tx_s, tx_b, tx_p), (entity, _node, inv, f_state, prod_fn, pricing)| {
            let mut inputs_val = [0.0; crate::economy::GOOD_TYPE_COUNT];
            for i in 0..crate::economy::GOOD_TYPE_COUNT {
                inputs_val[i] = inv.goods[i];
            }
            let output = crate::math::cobb_douglas(&inputs_val, &prod_fn.exponents, prod_fn.coefficient);

            if output > 0.0 {
                tx_p.send(ProductionIntent {
                    factory: *entity,
                    inputs: [(GoodType::LABOURHOUR, inv.get(GoodType::LABOURHOUR)), (GoodType::MACHINE, 0.0)], // Simplified
                    output: (f_state.produced_good, output),
                }).unwrap();
            }

            tx_s.send(SellIntent {
                seller: *entity,
                good: f_state.produced_good,
                amount: inv.get(f_state.produced_good),
                price_pv: pricing.current_price as u64,
            }).unwrap();

            let budget = 100.0;
            let mut prices = [10.0; crate::economy::GOOD_TYPE_COUNT];
            for good in GoodType::all() {
                if let Some(p) = market_ref.get_marginal_price(good) {
                    prices[good as usize] = p as f64;
                }
            }
            let optimal_inputs = crate::math::optimize_cobb_douglas_fixed_prices(&prod_fn.exponents, &prices, budget);
            for (i, &amount) in optimal_inputs.iter().enumerate() {
                if amount > 0.0 {
                    tx_b.send(BuyIntent {
                        buyer: *entity,
                        good: GoodType::all()[i],
                        max_amount: amount,
                        max_price_pv: (prices[i] * 1.2) as u64,
                    }).unwrap();
                }
            }
        });

        household_evals.par_iter().for_each_with((tx_sell.clone(), tx_buy.clone(), tx_cons.clone()), |(tx_s, tx_b, tx_c), (entity, _node, inv, _h_state, util_fn)| {
            tx_s.send(SellIntent {
                seller: *entity,
                good: GoodType::LABOURHOUR,
                amount: 8.0,
                price_pv: 10,
            }).unwrap();

            let mut inputs_val = [0.0; crate::economy::GOOD_TYPE_COUNT];
            for i in 0..crate::economy::GOOD_TYPE_COUNT {
                inputs_val[i] = inv.goods[i];
            }
            let utility = crate::math::cobb_douglas(&inputs_val, &util_fn.exponents, 1.0);
            if utility > 0.0 {
                let mut consumed = Vec::new();
                for g in GoodType::all() {
                    if inv.get(g) > 0.0 && g != GoodType::LABOURHOUR {
                        consumed.push((g, inv.get(g)));
                    }
                }
                tx_c.send(ConsumptionIntent {
                    household: *entity,
                    utility,
                    consumed,
                }).unwrap();
            }

            let budget = 50.0;
            let mut prices = [10.0; crate::economy::GOOD_TYPE_COUNT];
            for good in GoodType::all() {
                if let Some(p) = market_ref.get_marginal_price(good) {
                    prices[good as usize] = p as f64;
                }
            }
            let optimal_bundle = crate::math::optimize_cobb_douglas_fixed_prices(&util_fn.exponents, &prices, budget);
            for (i, &amount) in optimal_bundle.iter().enumerate() {
                if amount > 0.0 && GoodType::all()[i] != GoodType::LABOURHOUR {
                    tx_b.send(BuyIntent {
                        buyer: *entity,
                        good: GoodType::all()[i],
                        max_amount: amount,
                        max_price_pv: (prices[i] * 1.2) as u64,
                    }).unwrap();
                }
            }
        });

        // Phase 2: Materialization (Sequential)
        for (_entity, (node, _brain)) in self.world.query_mut::<(&mut ArithmodynamicNode, &NeoclassicalBrain)>() {
            if node.active_book_counts == 0 && node.vault_books > 0 {
                node.vault_books -= 1;
                node.active_book_counts = arithmodynamics::TOTAL_BOOK_COUNTS;
            }

            if node.active_book_counts > 0 {
                node.active_book_counts -= 1;
                node.counts += 1;
                node.prime_value = arithmodynamics::get_prime_value_for_counts(node.counts);
                node.entropy_delta += 1;
            }

            while node.prime_value >= arithmodynamics::MINT_SCARCITY {
                node.vault_books += 1;
                let old_counts = node.counts;
                node.prime_value -= arithmodynamics::MINT_SCARCITY;
                node.counts = arithmodynamics::get_counts_for_prime_value(node.prime_value);
                node.entropy_delta += (node.counts as i64) - (old_counts as i64);
            }
        }

        while let Ok(sell) = rx_sell.try_recv() {
            let book = &mut self.market.books[sell.good as usize];
            book.sell_orders.entry(sell.price_pv).or_default().push(crate::engine::market::SellOrder {
                seller: sell.seller,
                amount: sell.amount,
                price_pv: sell.price_pv,
            });
        }

        while let Ok(buy) = rx_buy.try_recv() {
            let book = &mut self.market.books[buy.good as usize];
            let mut remaining_amount = buy.max_amount;

            for (&price, orders) in book.sell_orders.iter_mut() {
                if price > buy.max_price_pv { break; }

                for order in orders.iter_mut() {
                    let fill = f64::min(remaining_amount, order.amount);
                    if fill > 0.0 {
                        let pv_total = (fill * price as f64) as u64;
                        if let Ok(mut buyer_node) = self.world.get::<&mut ArithmodynamicNode>(buy.buyer) {
                            if buyer_node.prime_value >= pv_total {
                                buyer_node.prime_value -= pv_total;
                                if let Ok(mut seller_node) = self.world.get::<&mut ArithmodynamicNode>(order.seller) {
                                    seller_node.prime_value += pv_total;

                                    if let Ok(mut buyer_inv) = self.world.get::<&mut Inventory>(buy.buyer) {
                                        buyer_inv.add(buy.good, fill);
                                    }
                                    if let Ok(mut seller_inv) = self.world.get::<&mut Inventory>(order.seller) {
                                        seller_inv.add(buy.good, -fill);
                                    }

                                    remaining_amount -= fill;
                                    order.amount -= fill;
                                } else {
                                    buyer_node.prime_value += pv_total; // Rollback
                                }
                            }
                        }
                    }
                    if remaining_amount <= 0.0 { break; }
                }
                if remaining_amount <= 0.0 { break; }
            }
            for (_, orders) in book.sell_orders.iter_mut() {
                orders.retain(|o| o.amount > 0.0);
            }
            book.sell_orders.retain(|_, v| !v.is_empty());
        }

        let mut total_utility = 0.0;
        while let Ok(prod) = rx_prod.try_recv() {
            if let Ok(mut inv) = self.world.get::<&mut Inventory>(prod.factory) {
                for (good, amount) in prod.inputs {
                    inv.add(good, -amount);
                }
                inv.add(prod.output.0, prod.output.1);
            }
        }

        while let Ok(cons) = rx_cons.try_recv() {
            if let Ok(mut inv) = self.world.get::<&mut Inventory>(cons.household) {
                for (good, amount) in cons.consumed {
                    inv.add(good, -amount);
                }
                total_utility += cons.utility;
                if let Ok(mut h_state) = self.world.get::<&mut HouseholdState>(cons.household) {
                    h_state.ticks_since_utility_met = 0;
                }
            }
        }

        // Phase 3: Macro Reduction & Demographics
        let mut to_despawn = Vec::new();
        let mut to_spawn = Vec::new();
        let mut current_stats = MacroStats::default();

        for (entity, (h_state, node)) in self.world.query_mut::<(&mut HouseholdState, &ArithmodynamicNode)>() {
            h_state.ticks_since_utility_met += 1;
            if h_state.ticks_since_utility_met >= 60 {
                to_despawn.push(entity);
            } else if h_state.ticks_since_utility_met == 1 {
                 if node.prime_value > 2000 {
                     to_spawn.push(true);
                 }
            }
        }

        for (_, (node, _inv)) in self.world.query::<(&ArithmodynamicNode, &Inventory)>().iter() {
            current_stats.total_pv += node.prime_value;
            current_stats.total_entropy += node.entropy_delta;
            current_stats.agent_count += 1;
            current_stats.m0 += node.prime_value;
        }
        current_stats.total_utility = total_utility;
        self.stats = current_stats;

        for entity in to_despawn {
            let _ = self.world.despawn(entity);
        }
        for _ in to_spawn {
            self.spawn_household(500);
        }
    }

    pub fn spawn_household(&mut self, initial_pv: u64) {
        let mut utility_exponents = [0.0; crate::economy::GOOD_TYPE_COUNT];
        utility_exponents[GoodType::FOOD as usize] = 1.0;

        self.world.spawn((
            ArithmodynamicNode::new(initial_pv),
            Inventory::new(),
            NeoclassicalBrain {
                brain_type: crate::economy::BrainType::Household,
            },
            crate::economy::UtilityFunction {
                exponents: utility_exponents,
            },
            HouseholdState {
                ticks_since_utility_met: 0,
                age_ticks: 0,
            },
        ));
    }

    pub fn spawn_factory(&mut self, initial_pv: u64, produced_good: crate::economy::GoodType) {
        let mut production_exponents = [0.0; crate::economy::GOOD_TYPE_COUNT];
        production_exponents[GoodType::LABOURHOUR as usize] = 1.0;

        self.world.spawn((
            ArithmodynamicNode::new(initial_pv),
            Inventory::new(),
            NeoclassicalBrain {
                brain_type: crate::economy::BrainType::Factory,
            },
            crate::economy::ProductionFunction {
                coefficient: 1.0,
                exponents: production_exponents,
            },
            crate::economy::PricingStrategy::new(10.0),
            FactoryState { produced_good },
        ));
    }
}
