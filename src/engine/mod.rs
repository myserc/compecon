use hecs::{World, Entity};
use crate::arithmodynamics::{self, ArithmodynamicNode};
use crate::economy::{Inventory, NeoclassicalBrain, HouseholdState, FactoryState, BrainType, GoodType};
use rayon::prelude::*;
use crossbeam::channel;

pub mod intents {
    use hecs::Entity;
    use crate::economy::GoodType;

    #[derive(Debug, Clone)]
    pub struct TransferIntent {
        pub from: Entity,
        pub to: Entity,
        pub good: Option<GoodType>,
        pub amount_f64: f64,
        pub pv_amount: u64,
    }
}

use self::intents::TransferIntent;

#[derive(Default, Debug)]
pub struct MacroStats {
    pub total_pv: u64,
    pub total_entropy: i64,
    pub agent_count: usize,
}

pub struct Simulation {
    pub world: World,
    pub tick: u64,
    pub stats: MacroStats,
}

impl Simulation {
    pub fn new() -> Self {
        Self {
            world: World::new(),
            tick: 0,
            stats: MacroStats::default(),
        }
    }

    pub fn tick(&mut self) {
        self.tick += 1;

        // Phase 1: Evaluation & Intent (Parallel)
        let (tx, rx) = channel::unbounded::<TransferIntent>();

        let mut evaluations = Vec::new();
        for (entity, (node, inventory, brain)) in self.world.query::<(&ArithmodynamicNode, &Inventory, &NeoclassicalBrain)>().iter() {
            evaluations.push((entity, node.clone(), inventory.clone(), brain.clone()));
        }

        let mut factories = Vec::new();
        let mut households = Vec::new();
        for (entity, brain) in self.world.query::<&NeoclassicalBrain>().iter() {
            match brain.brain_type {
                BrainType::Factory => factories.push(entity),
                BrainType::Household => households.push(entity),
            }
        }

        evaluations.par_iter().for_each_with((tx, factories, households), |(tx, factories, households), (entity, _node, _inv, brain)| {
            match brain.brain_type {
                BrainType::Household => {
                    if let Some(&factory_entity) = factories.first() {
                        let wage = 50.0;
                        tx.send(TransferIntent {
                            from: factory_entity,
                            to: *entity,
                            good: Some(GoodType::LABOURHOUR),
                            amount_f64: 1.0,
                            pv_amount: wage as u64,
                        }).unwrap();
                    }
                }
                BrainType::Factory => {
                    if let Some(&household_entity) = households.first() {
                        let price = 10.0;
                        tx.send(TransferIntent {
                            from: household_entity,
                            to: *entity,
                            good: Some(GoodType::BREAD),
                            amount_f64: 1.0,
                            pv_amount: price as u64,
                        }).unwrap();
                    }
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

        while let Ok(intent) = rx.try_recv() {
            let mut from_node_success = false;

            if let Ok(mut from_node) = self.world.get::<&mut ArithmodynamicNode>(intent.from) {
                if from_node.prime_value >= intent.pv_amount {
                    from_node.prime_value -= intent.pv_amount;
                    let old_counts = from_node.counts;
                    from_node.counts = arithmodynamics::get_counts_for_prime_value(from_node.prime_value);
                    from_node.entropy_delta += (from_node.counts as i64) - (old_counts as i64);
                    from_node_success = true;
                }
            }

            if from_node_success {
                if let Ok(mut to_node) = self.world.get::<&mut ArithmodynamicNode>(intent.to) {
                    to_node.prime_value += intent.pv_amount;
                    let old_counts_to = to_node.counts;
                    to_node.counts = arithmodynamics::get_counts_for_prime_value(to_node.prime_value);
                    to_node.entropy_delta += (to_node.counts as i64) - (old_counts_to as i64);

                    if let Some(good) = intent.good {
                        if let Ok(mut inv) = self.world.get::<&mut Inventory>(intent.to) {
                            *inv.goods.entry(good).or_insert(0.0) += intent.amount_f64;

                            if good == GoodType::BREAD {
                                if let Ok(mut h_state) = self.world.get::<&mut HouseholdState>(intent.to) {
                                    h_state.ticks_since_utility_met = 0;
                                }
                            }
                        }
                    }
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

        for (_, node) in self.world.query::<&ArithmodynamicNode>().iter() {
            current_stats.total_pv += node.prime_value;
            current_stats.total_entropy += node.entropy_delta;
            current_stats.agent_count += 1;
        }
        self.stats = current_stats;

        for entity in to_despawn {
            let _ = self.world.despawn(entity);
        }
        for _ in to_spawn {
            self.spawn_household(500);
        }
    }

    pub fn spawn_household(&mut self, initial_pv: u64) {
        self.world.spawn((
            ArithmodynamicNode::new(initial_pv),
            Inventory::new(),
            NeoclassicalBrain {
                brain_type: crate::economy::BrainType::Household,
                price_expectations: std::collections::HashMap::new(),
            },
            HouseholdState { ticks_since_utility_met: 0 },
        ));
    }

    pub fn spawn_factory(&mut self, initial_pv: u64, produced_good: crate::economy::GoodType) {
        self.world.spawn((
            ArithmodynamicNode::new(initial_pv),
            Inventory::new(),
            NeoclassicalBrain {
                brain_type: crate::economy::BrainType::Factory,
                price_expectations: std::collections::HashMap::new(),
            },
            FactoryState { produced_good },
        ));
    }
}
