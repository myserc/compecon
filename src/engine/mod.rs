use hecs::World;
use crate::arithmodynamics::{self, ArithmodynamicNode};
use crate::economy::{Inventory, NeoclassicalBrain, HouseholdState, FactoryState, GoodType};
use crossbeam::channel;
use serde::{Serialize, Deserialize};

pub mod market;
pub mod dashboard;
pub mod systems;
pub mod statistics;

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
        pub inputs: Vec<(GoodType, f64)>,
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
    pub m1: u64,
    pub total_utility: f64,
    pub gini: f64,
    pub prices: std::collections::HashMap<String, f64>,
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
    pub control_rx: Option<crossbeam::channel::Receiver<dashboard::ControlCommand>>,
}

impl Simulation {
    pub fn new() -> Self {
        Self {
            world: World::new(),
            time: TimeSystem::default(),
            market: market::Market::new(),
            stats: MacroStats::default(),
            control_rx: None,
        }
    }

    pub fn tick(&mut self) {
        self.time.advance();

        if let Some(rx) = &self.control_rx {
            while let Ok(cmd) = rx.try_recv() {
                match cmd {
                    dashboard::ControlCommand::EconomicShock(intensity) => {
                        println!("Applying economic shock: intensity={}", intensity);
                        for (_entity, prod_fn) in self.world.query_mut::<&mut crate::economy::ProductionFunction>() {
                            prod_fn.coefficient *= 1.0 + intensity;
                        }
                    }
                    dashboard::ControlCommand::DeficitSpending(amount) => {
                        println!("Injecting deficit spending: amount={}", amount);
                        // Find the state entity and inject PV
                        for (_entity, (_s_state, node)) in self.world.query_mut::<(&crate::economy::StateState, &mut ArithmodynamicNode)>() {
                            node.prime_value += amount;
                        }
                    }
                }
            }
        }

        // Phase 1: Evaluation & Intent
        let (tx_sell, rx_sell) = channel::unbounded::<SellIntent>();
        let (tx_buy, rx_buy) = channel::unbounded::<BuyIntent>();
        let (tx_prod, rx_prod) = channel::unbounded::<ProductionIntent>();
        let (tx_cons, rx_cons) = channel::unbounded::<ConsumptionIntent>();

        systems::household::household_system(&self.world, &tx_buy, &tx_sell, &tx_cons, &self.market);
        systems::factory::factory_system(&self.world, &tx_buy, &tx_sell, &tx_prod, &self.market);
        systems::state::state_system(&self.world, &tx_buy);
        systems::trader::trader_system(&self.world, &tx_buy, &tx_sell, &self.market);
        systems::finance::bank_system(&mut self.world);

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
            if let Ok(mut pricing) = self.world.get::<&mut crate::economy::PricingStrategy>(sell.seller) {
                pricing.last_offered += sell.amount;
            }
            let book = &mut self.market.books[sell.good as usize];
            book.asks.entry(ordered_float::OrderedFloat(sell.price_pv as f64)).or_default().push(crate::engine::market::SellOrder {
                seller: sell.seller,
                amount: sell.amount,
                price_f64: sell.price_pv as f64,
            });
        }

        while let Ok(buy) = rx_buy.try_recv() {
            let budget_pv = (buy.max_price_pv as f64 * buy.max_amount).round() as u64;
            let buyer_pv = self.world.get::<&ArithmodynamicNode>(buy.buyer).map(|n| n.prime_value).unwrap_or(0);

            if buyer_pv < budget_pv {
                continue;
            }

            let book = &mut self.market.books[buy.good as usize];
            let (amount_acquired, spent_pv, transactions) = book.execute_buy(buy.buyer, budget_pv, buy.max_price_pv as f64, buy.max_amount);

            if amount_acquired > 0.0 {
                // To avoid multiple mutable borrows of ArithmodynamicNode at once,
                // we'll update the buyer and sellers separately.
                let mut success = false;
                if let Ok(mut buyer_node) = self.world.get::<&mut ArithmodynamicNode>(buy.buyer) {
                    if buyer_node.prime_value >= spent_pv {
                        buyer_node.prime_value -= spent_pv;
                        success = true;
                    }
                }

                if success {
                    let mut distributed_pv = 0;
                    let tx_len = transactions.len();
                    for (idx, tx) in transactions.iter().enumerate() {
                        let tx_pv = if idx == tx_len - 1 {
                            spent_pv - distributed_pv
                        } else {
                            (tx.amount * tx.price).round() as u64
                        };
                        distributed_pv += tx_pv;

                        if let Ok(mut seller_node) = self.world.get::<&mut ArithmodynamicNode>(tx.seller) {
                            seller_node.prime_value += tx_pv;
                        }
                        if let Ok(mut seller_inv) = self.world.get::<&mut Inventory>(tx.seller) {
                            seller_inv.add(buy.good, -tx.amount);
                        }
                        if let Ok(mut seller_pricing) = self.world.get::<&mut crate::economy::PricingStrategy>(tx.seller) {
                            seller_pricing.last_sold += tx.amount;
                        }
                    }

                    if let Ok(mut buyer_inv) = self.world.get::<&mut Inventory>(buy.buyer) {
                        buyer_inv.add(buy.good, amount_acquired);
                    }
                }
            }
        }

        let mut total_utility = 0.0;
        while let Ok(prod) = rx_prod.try_recv() {
            if let Ok(mut inv) = self.world.get::<&mut Inventory>(prod.factory) {
                let mut can_afford_inputs = true;
                for (good, amount) in &prod.inputs {
                    if inv.get(*good) < *amount {
                        can_afford_inputs = false;
                        break;
                    }
                }

                if can_afford_inputs {
                    for (good, amount) in prod.inputs {
                        inv.add(good, -amount);
                    }
                    inv.add(prod.output.0, prod.output.1);
                }
            }
        }

        while let Ok(cons) = rx_cons.try_recv() {
            if let Ok(mut inv) = self.world.get::<&mut Inventory>(cons.household) {
                let mut can_afford_consumed = true;
                for (good, amount) in &cons.consumed {
                    if inv.get(*good) < *amount {
                        can_afford_consumed = false;
                        break;
                    }
                }

                if can_afford_consumed {
                    for (good, amount) in cons.consumed {
                        inv.add(good, -amount);
                    }
                    total_utility += cons.utility;
                    if let Ok(mut h_state) = self.world.get::<&mut HouseholdState>(cons.household) {
                        h_state.ticks_since_utility_met = 0;
                    }
                }
            }
        }

        // Phase 3: Macro Reduction & Demographics
        let mut to_despawn = Vec::new();
        let mut to_spawn = Vec::new();

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

        for (_entity, pricing) in self.world.query_mut::<&mut crate::economy::PricingStrategy>() {
            pricing.adapt_price();
        }

        let detailed_stats = statistics::calculate_statistics(&self.world, &self.market);

        let mut current_stats = MacroStats::default();
        current_stats.m0 = detailed_stats.m0;
        current_stats.m1 = detailed_stats.m1;
        current_stats.gini = detailed_stats.gini;
        current_stats.total_utility = total_utility;
        current_stats.agent_count = self.world.len() as usize;

        for g in GoodType::all() {
            let name = format!("{:?}", g);
            let price = self.market.get_marginal_price(g).unwrap_or(0.0);
            current_stats.prices.insert(name, price);
        }

        self.stats = current_stats;

        for entity in to_despawn {
            let _ = self.world.despawn(entity);
        }
        for _ in to_spawn {
            self.spawn_household(500);
        }

        self.market.clear();
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

    pub fn spawn_trader(&mut self, initial_pv: u64, traded_good: crate::economy::GoodType) {
        self.world.spawn((
            ArithmodynamicNode::new(initial_pv),
            Inventory::new(),
            NeoclassicalBrain {
                brain_type: crate::economy::BrainType::Trader,
            },
            crate::economy::TraderState { traded_good },
        ));
    }

    pub fn spawn_state(&mut self, initial_pv: u64) {
        self.world.spawn((
            ArithmodynamicNode::new(initial_pv),
            Inventory::new(),
            NeoclassicalBrain {
                brain_type: crate::economy::BrainType::State,
            },
            crate::economy::StateState { tax_rate: 0.2 },
        ));
    }

    pub fn spawn_bank(&mut self, initial_pv: u64) {
        self.world.spawn((
            ArithmodynamicNode::new(initial_pv),
            Inventory::new(),
            NeoclassicalBrain {
                brain_type: crate::economy::BrainType::CreditBank,
            },
            crate::economy::CreditBankState {
                interest_rate: 0.05,
                reserves_pv: initial_pv,
            },
        ));
    }
}
