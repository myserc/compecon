// ==========================================
// 1. IMPORTS & DEPENDENCIES
// ==========================================
// Cargo.toml requirements:
// tokio = { version = "1", features =["full"] }
// axum = { version = "0.7", features =["ws"] }
// serde = { version = "1.0", features = ["derive"] }
// serde_json = "1.0"
// rand = { version = "0.8", features =["small_rng"] }
// crossbeam = "0.8"
// rayon = "1.8"
// lazy_static = "1.4"

use axum::{
    Router,
    extract::{
        State as AxumState,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::{Html, IntoResponse},
    routing::get,
};
use crossbeam::channel;
use lazy_static::lazy_static;
use rand::{Rng, SeedableRng, rngs::SmallRng};
use rayon::prelude::*;
use serde::Serialize;
use std::collections::{HashSet, VecDeque};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;
use tokio::sync::broadcast;

// ==========================================
// 2. REAL-TIME CONTROLS & CONFIGURATION
// ==========================================
pub struct SimControls {
    pub is_paused: AtomicBool,
    pub speed_delay_ms: AtomicU64,     
    pub transfer_prob_bits: AtomicU64, 
    pub is_mpc_enabled: AtomicBool, // NEW: MPC Toggle Switch
}

lazy_static! {
    pub static ref SIM_CONTROLS: SimControls = SimControls {
        is_paused: AtomicBool::new(false),
        speed_delay_ms: AtomicU64::new(0), 
        transfer_prob_bits: AtomicU64::new(1.0_f64.to_bits()), 
        is_mpc_enabled: AtomicBool::new(false), // Defaults to ON
    };
}

const MAX_UI_AGENTS: usize = 2_500;
const NUM_SHARDS: usize = 64; 

pub struct Config {
    pub mode: String,
    pub domain: String,
    pub limit: usize,
    pub total_book_counts: i64,
    pub standard_mint_scarcity: u64,
    pub initial_value: u64,
    pub num_agents: usize,
}

lazy_static! {
    pub static ref CONFIG: Config = {
        let args: Vec<String> = std::env::args().collect();
        let mode = if args.len() > 1 { args[1].to_lowercase() } else { "coop".to_string() };
        let domain = if args.len() > 2 { args[2].to_lowercase() } else { "prime".to_string() };

        let (total_book_counts, standard_mint_scarcity, initial_value) = match (mode.as_str(), domain.as_str()) {
            ("finn", "comp") => (10_800, 12_267, 4),
            ("finn", "prime") | ("finn", _) => (180, 1_069, 2),
            ("coop", "comp") => (648_000, 704_922, 4),
            ("coop", "prime") | _ => (648_000, 9_731_081, 2),
        };

        Config {
            mode, domain,
            limit: 20_000_000,
            total_book_counts, standard_mint_scarcity, initial_value,
            num_agents: 1_000_000,
        }
    };
}

// ==========================================
// 3. O(1) TOPOLOGY ENGINE
// ==========================================
lazy_static! {
    static ref SIEVE: Vec<bool> = build_sieve(CONFIG.limit);
    static ref PRIMES: Vec<u64> = build_primes(CONFIG.limit, &SIEVE);
    static ref PRIME_INDEX_MAP: Vec<u32> = generate_index_map(CONFIG.limit, &PRIMES);
    static ref GPF_MAP: Vec<u32> = build_gpf_map(CONFIG.limit);
    
    static ref SEQUENCE: Vec<u64> = if CONFIG.domain == "comp" { build_composites(CONFIG.limit, &SIEVE) } else { PRIMES.clone() };
    static ref SEQUENCE_INDEX_MAP: Vec<u32> = generate_index_map(CONFIG.limit, &SEQUENCE);
}

fn build_sieve(limit: usize) -> Vec<bool> {
    let mut sieve = vec![true; limit];
    sieve[0] = false; if limit > 1 { sieve[1] = false; }
    let sqrt_limit = (limit as f64).sqrt() as usize;
    for p in 2..=sqrt_limit {
        if sieve[p] {
            let mut i = p * p;
            while i < limit { sieve[i] = false; i += p; }
        }
    }
    sieve
}

fn build_primes(limit: usize, sieve: &[bool]) -> Vec<u64> {
    let mut primes = Vec::with_capacity(limit / 10);
    for i in 2..limit { if sieve[i] { primes.push(i as u64); } }
    primes
}

fn build_composites(limit: usize, sieve: &[bool]) -> Vec<u64> {
    let mut comps = Vec::with_capacity(limit);
    for i in 4..limit { if !sieve[i] { comps.push(i as u64); } }
    comps
}

fn generate_index_map(limit: usize, seq: &[u64]) -> Vec<u32> {
    let mut map = vec![0; limit];
    let mut current_idx = 0;
    for i in 0..limit {
        if current_idx + 1 < seq.len() && (i as u64) >= seq[current_idx + 1] { current_idx += 1; }
        map[i] = current_idx as u32;
    }
    map
}

fn build_gpf_map(limit: usize) -> Vec<u32> {
    let mut gpf = vec![0; limit];
    if limit > 0 { gpf[0] = 0; }
    if limit > 1 { gpf[1] = 1; }
    for i in 2..limit {
        if gpf[i] == 0 {
            gpf[i] = i as u32;
            let mut j = i * 2;
            while j < limit { gpf[j] = i as u32; j += i; }
        }
    }
    gpf
}

#[inline(always)]
fn get_ordinal_for_sequence(value: u64) -> usize {
    if value >= CONFIG.limit as u64 { SEQUENCE.len() - 1 } else { SEQUENCE_INDEX_MAP[value as usize] as usize }
}

#[inline(always)]
fn get_ordinal_for_prime(value: u64) -> usize {
    if value >= CONFIG.limit as u64 { PRIMES.len() - 1 } else { PRIME_INDEX_MAP[value as usize] as usize }
}

fn greatest_prime_factor(mut n: u64) -> u64 {
    let mut max_prime = 0;
    while n % 2 == 0 { max_prime = 2; n /= 2; }
    let mut i = 3;
    while i * i <= n { while n % i == 0 { max_prime = i; n /= i; } i += 2; }
    if n > 2 { max_prime = n; }
    max_prime
}

// ==========================================
// 4. ECS DATA STRUCTURES & BEHAVIOR
// ==========================================
#[derive(Clone)]
pub struct NodeData {
    pub id: usize,
    pub transfer_prob: f64, 
    pub mpc: f64,            
    pub wake_tick: u64,
    pub vault_books: u64,
    pub active_book_counts: u64,
    pub counts: usize,
    pub prime_value: u64,
    pub balance_adjustment: u64,
    pub entropy_delta: i64,
    pub last_active_tick: u64,
    pub volume_facilitated: u64,
}

impl NodeData {
    #[inline(always)]
    fn update_prime_value(&mut self) {
        let mut ordinal_idx = if self.counts > 0 { self.counts - 1 } else { 0 };
        ordinal_idx = ordinal_idx.min(SEQUENCE.len() - 1);
        self.prime_value = unsafe { *SEQUENCE.get_unchecked(ordinal_idx) } + self.balance_adjustment;
    }
}

#[derive(Clone)]
pub struct TransferIntent {
    pub to: usize,
    pub amount: u64,
}

pub struct Shard {
    pub nodes: Vec<NodeData>,
    pub rx: channel::Receiver<Vec<TransferIntent>>, 
    pub rng: SmallRng,
    
    pub local_factors: Vec<(usize, u64)>,
    pub local_intents: Vec<(usize, u64)>,
    pub outbound_intents: Vec<Vec<TransferIntent>>,

    pub shard_vault_books: u64,
    pub shard_plasma_counts: u64,
    pub shard_pv: u64,
}

#[derive(Serialize, Clone, Default)]
pub struct FactorRank { pub counts: usize, pub sum_multiples: u64 }

#[derive(Serialize, Clone, Default)]
pub struct Telemetry {
    pub tick: u64, pub mode: String, pub domain: String, pub active_agents: u64, pub total_vault_books: u64, 
    pub sublimated_plasma_books: f64, pub inflation_rate: f64, pub net_entropy: i64, pub void_events: i64, 
    pub surplus_events: i64, pub total_transfers: u64, pub velocity_of_value: f64, pub pop_inflation_ratio: f64, 
    pub avalanche_peak: u64, pub chaos_variance: f64, pub churn_rate: f64, pub hash_difficulty: f64, 
    pub gini_coefficient: f64, pub tx_gini: f64, pub pct_wealth_top_1: f64, pub pct_wealth_top_20: f64, 
    pub pct_wealth_bottom_50: f64, pub agent_deltas: Vec<i64>, pub top_factors: Vec<FactorRank>, 
    pub bottom_factors: Vec<FactorRank>, pub sim_speed_ms: u64, pub transfer_prob: f64, pub is_paused: bool,
    pub mpc_enabled: bool, // NEW
}

// ==========================================
// 5. HIGH-PERFORMANCE ENGINE
// ==========================================
pub struct Engine {
    pub shards: Vec<Shard>,
    pub txs: Vec<channel::Sender<Vec<TransferIntent>>>,
    pub nodes_per_shard: usize,
    pub tick_count: u64,

    pub global_net_entropy: i64,
    pub global_surplus: i64,
    pub void_events: i64,
    pub surplus_events: i64,
    pub total_transfers: u64,

    pub last_top_100: HashSet<usize>,
    pub cached_churn_rate: f64,
    pub vault_history: VecDeque<u64>,
    pub global_factor_multiples: Vec<u64>,

    pub wealth_array: Vec<(usize, u64)>, 
}

impl Engine {
    pub fn new(agent_count: usize) -> Self {
        let _ = &*SEQUENCE; let _ = &*PRIMES; let _ = &*GPF_MAP;

        let nodes_per_shard = (agent_count + NUM_SHARDS - 1) / NUM_SHARDS;
        let mut shards = Vec::with_capacity(NUM_SHARDS);
        let mut txs = Vec::with_capacity(NUM_SHARDS);
        let mut current_id = 0;

        for _ in 0..NUM_SHARDS {
            let (tx, rx) = channel::unbounded();
            txs.push(tx);

            let mut nodes = Vec::with_capacity(nodes_per_shard);
            let mut shard_rng = SmallRng::from_entropy();

            for _ in 0..nodes_per_shard {
                if current_id >= agent_count { break; }
                nodes.push(NodeData {
                    id: current_id,
                    transfer_prob: shard_rng.gen_range(0.001..=0.1),
                    mpc: shard_rng.gen_range(0.5..=0.99), 
                    wake_tick: shard_rng.gen_range(0..=1000),
                    vault_books: 1, active_book_counts: 0, counts: 0,
                    prime_value: CONFIG.initial_value, balance_adjustment: 0,
                    entropy_delta: 0, last_active_tick: 0, volume_facilitated: 0,
                });
                current_id += 1;
            }

            let mut outbound_intents = Vec::with_capacity(NUM_SHARDS);
            for _ in 0..NUM_SHARDS { outbound_intents.push(Vec::with_capacity(100)); }

            shards.push(Shard { 
                nodes, rx, rng: shard_rng, 
                local_factors: Vec::with_capacity(5000), 
                local_intents: Vec::with_capacity(5000), 
                outbound_intents,
                shard_vault_books: nodes_per_shard as u64,
                shard_plasma_counts: 0,
                shard_pv: nodes_per_shard as u64 * CONFIG.initial_value,
            });
        }

        Self {
            shards, txs, nodes_per_shard, tick_count: 0, global_net_entropy: 0, global_surplus: 0,
            void_events: 0, surplus_events: 0, total_transfers: 0, last_top_100: HashSet::new(),
            cached_churn_rate: 100.0, vault_history: VecDeque::new(), global_factor_multiples: vec![0; PRIMES.len() + 1],
            wealth_array: Vec::with_capacity(agent_count),
        }
    }

    pub fn tick(&mut self, generate_telemetry: bool) -> Option<Telemetry> {
        self.tick_count += 1;
        let tick = self.tick_count;
        let prob_multiplier = f64::from_bits(SIM_CONTROLS.transfer_prob_bits.load(Ordering::Relaxed));
        let is_mpc_active = SIM_CONTROLS.is_mpc_enabled.load(Ordering::Relaxed);
        let txs = &self.txs;
        let nodes_per_shard = self.nodes_per_shard;

        // PHASE 1: EVALUATION & SENDING
        let shard_results: Vec<_> = self.shards.par_iter_mut().map(|shard| {
            shard.local_factors.clear();
            shard.local_intents.clear();
            for out in &mut shard.outbound_intents { out.clear(); }

            let mut local_entropy = 0i64;
            let mut local_mints = 0u64;
            let mut local_transfers = 0u64;
            let mut local_vol = 0u64;
            let num_shard_nodes = shard.nodes.len();
            for node in &mut shard.nodes {
                if tick < node.wake_tick { continue; }
                if !generate_telemetry { node.entropy_delta = 0; }

                let prev_vb = node.vault_books;
                let prev_pc = node.active_book_counts + node.counts as u64;
                let prev_pv = node.prime_value;

                if node.active_book_counts == 0 && node.vault_books > 0 {
                    node.vault_books -= 1;
                    node.active_book_counts = CONFIG.total_book_counts as u64;
                }

                let potential_work = 1;
                if node.active_book_counts >= potential_work {
                    node.active_book_counts -= potential_work;
                    node.counts += potential_work as usize;
                    node.update_prime_value();
                }

                loop {
                    // MPC Switchable Minting Threshold
                    let threshold = if is_mpc_active {
                        (CONFIG.standard_mint_scarcity as f64 * (1.0 + node.mpc)) as u64
                    } else {
                        CONFIG.standard_mint_scarcity
                    };

                    if node.prime_value < threshold { break; }

                    node.last_active_tick = tick;
                    local_mints += 1;
                    node.vault_books += 1;

                    let new_pv = node.prime_value.saturating_sub(threshold);
                    let new_counts_idx = get_ordinal_for_sequence(new_pv);
                    let seq_val = unsafe { *SEQUENCE.get_unchecked(new_counts_idx) };

                    node.prime_value = new_pv;
                    node.counts = new_counts_idx + 1;
                    node.balance_adjustment = new_pv.saturating_sub(seq_val);
                }

                let actual_prob = (node.transfer_prob * prob_multiplier).clamp(0.0, 1.0);

                if shard.rng.gen_bool(actual_prob) {
                    let max_affordable = node.prime_value.saturating_sub(CONFIG.initial_value);
                    
                    if max_affordable > 0 {
                        // MPC Switchable Spending Logistics
                        let amount = if is_mpc_active {
                            let wealth_adjusted_mpc = (node.mpc - (node.vault_books as f64 * 0.05)).max(0.1);
                            let target_spend = (max_affordable as f64 * wealth_adjusted_mpc) as u64;
                            let min_spend = (target_spend / 2).max(1);
                            let max_spend = ((target_spend * 3) / 2).clamp(1, max_affordable);
                            shard.rng.gen_range(min_spend..=max_spend)
                        } else {
                            shard.rng.gen_range(1..=max_affordable) // Pure zero-intelligence random
                        };
                        
                        let gpf = if (amount as usize) < CONFIG.limit { unsafe { *GPF_MAP.get_unchecked(amount as usize) as u64 } } else { greatest_prime_factor(amount) };
                        let multiple = if gpf > 0 { amount / gpf } else { 0 };
                        let gpf_counts = if gpf > 1 { get_ordinal_for_prime(gpf) + 1 } else { 0 };

                        let new_source_val = node.prime_value.saturating_sub(amount);
                        let new_source_counts_idx = get_ordinal_for_sequence(new_source_val);
                        let source_leap = (new_source_counts_idx as i64 + 1) - node.counts as i64;
                        let seq_val = unsafe { *SEQUENCE.get_unchecked(new_source_counts_idx) };

                        node.prime_value = new_source_val;
                        node.counts = new_source_counts_idx + 1;
                        node.balance_adjustment = new_source_val.saturating_sub(seq_val);

                        let is_local = shard.rng.gen_bool(0.5); 
                        if is_local {
                            let local_idx = shard.rng.gen_range(0..num_shard_nodes);
                            if local_idx != (node.id % nodes_per_shard) { shard.local_intents.push((local_idx, amount)); } 
                        } else {
                            let target_id = shard.rng.gen_range(0..CONFIG.num_agents);
                            if target_id != node.id {
                                let target_shard = target_id / nodes_per_shard;
                                shard.outbound_intents[target_shard].push(TransferIntent { to: target_id, amount });
                            }
                        }

                        node.last_active_tick = tick;
                        node.volume_facilitated += amount;
                        local_vol += amount;
                        local_transfers += 1;
                        local_entropy += source_leap;
                        node.entropy_delta += source_leap;

                        if multiple > 0 { shard.local_factors.push((gpf_counts, multiple)); }
                    }
                }

                // Incremental Telemetry Deltas
                shard.shard_vault_books = shard.shard_vault_books.wrapping_add(node.vault_books).wrapping_sub(prev_vb);
                shard.shard_plasma_counts = shard.shard_plasma_counts.wrapping_add(node.active_book_counts + node.counts as u64).wrapping_sub(prev_pc);
                shard.shard_pv = shard.shard_pv.wrapping_add(node.prime_value).wrapping_sub(prev_pv);
            }

            // Execute buffered Local Intents securely
            for &(local_idx, amount) in &shard.local_intents {
                if let Some(target) = shard.nodes.get_mut(local_idx) {
                    let prev_pc = target.active_book_counts + target.counts as u64;
                    let prev_pv = target.prime_value;

                    target.last_active_tick = tick;
                    target.volume_facilitated += amount;

                    let new_pv = target.prime_value.saturating_add(amount);
                    let new_counts_idx = get_ordinal_for_sequence(new_pv);
                    let target_leap = (new_counts_idx as i64 + 1) - target.counts as i64;
                    let seq_val = unsafe { *SEQUENCE.get_unchecked(new_counts_idx) };

                    target.prime_value = new_pv;
                    target.counts = new_counts_idx + 1;
                    target.balance_adjustment = target.prime_value.saturating_sub(seq_val);

                    local_entropy += target_leap;
                    target.entropy_delta += target_leap;

                    shard.shard_plasma_counts = shard.shard_plasma_counts.wrapping_add(target.active_book_counts + target.counts as u64).wrapping_sub(prev_pc);
                    shard.shard_pv = shard.shard_pv.wrapping_add(target.prime_value).wrapping_sub(prev_pv);
                }
            }

            for (shard_idx, batch) in shard.outbound_intents.iter().enumerate() {
                if !batch.is_empty() { let _ = txs[shard_idx].send(batch.clone()); }
            }

            (local_entropy, local_mints, local_transfers, local_vol)
        }).collect();

        // PHASE 2: SEQUENTIAL MATERIALIZATION
        let phase2_results: Vec<_> = self.shards.par_iter_mut().map(|shard| {
            let mut p2_entropy = 0i64;
            for batch in shard.rx.try_iter() {
                for intent in batch {
                    let local_idx = intent.to % nodes_per_shard;
                    if let Some(target) = shard.nodes.get_mut(local_idx) {
                        let prev_pc = target.active_book_counts + target.counts as u64;
                        let prev_pv = target.prime_value;

                        target.last_active_tick = tick;
                        target.volume_facilitated += intent.amount;

                        let new_pv = target.prime_value.saturating_add(intent.amount);
                        let new_counts_idx = get_ordinal_for_sequence(new_pv);
                        let target_leap = (new_counts_idx as i64 + 1) - target.counts as i64;
                        let seq_val = unsafe { *SEQUENCE.get_unchecked(new_counts_idx) };

                        target.prime_value = new_pv;
                        target.counts = new_counts_idx + 1;
                        target.balance_adjustment = target.prime_value.saturating_sub(seq_val);

                        p2_entropy += target_leap;
                        target.entropy_delta += target_leap;

                        shard.shard_plasma_counts = shard.shard_plasma_counts.wrapping_add(target.active_book_counts + target.counts as u64).wrapping_sub(prev_pc);
                        shard.shard_pv = shard.shard_pv.wrapping_add(target.prime_value).wrapping_sub(prev_pv);
                    }
                }
            }
            p2_entropy
        }).collect();

        // PHASE 3: REDUCTION
        let mut tick_entropy = 0i64; let mut tick_mints = 0u64; let mut tick_vol = 0u64;
        
        for res in shard_results {
            tick_entropy += res.0; tick_mints = tick_mints.max(res.1);
            self.total_transfers += res.2; tick_vol += res.3;
        }
        for e in phase2_results { tick_entropy += e; }

        for shard in &self.shards {
            for &(gpf_counts, multiple) in &shard.local_factors {
                self.global_factor_multiples[gpf_counts] += multiple;
            }
        }

        self.global_net_entropy += tick_entropy;
        let thresh = CONFIG.total_book_counts;
        while self.global_net_entropy >= thresh { self.global_net_entropy -= thresh; self.global_surplus += 1; self.surplus_events += 1; }
        while self.global_net_entropy <= -thresh { self.global_net_entropy += thresh; self.void_events += 1; if self.global_surplus > 0 { self.global_surplus -= 1; } }

        if !generate_telemetry { return None; }

        let num_agents = CONFIG.num_agents;
        let mut total_vault_books = 0u64;
        let mut global_pv = 0u64;
        let mut total_plasma_counts = 0u64;
        
        for shard in &self.shards {
            total_vault_books += shard.shard_vault_books;
            global_pv += shard.shard_pv;
            total_plasma_counts += shard.shard_plasma_counts;
        }

        let mut sample_wealth = Vec::with_capacity(10_000);
        let mut sample_volume = Vec::with_capacity(10_000);
        let mut sample_deltas = Vec::with_capacity(10_000);
        let mut active_agents_sample = 0;
        let mut rng = rand::thread_rng();

        let mut ui_deltas = Vec::with_capacity(MAX_UI_AGENTS);

        for _ in 0..10_000 {
            let s_idx = rng.gen_range(0..NUM_SHARDS);
            let n_idx = rng.gen_range(0..self.shards[s_idx].nodes.len());
            let node = &self.shards[s_idx].nodes[n_idx];
            
            sample_wealth.push(node.vault_books);
            sample_volume.push(node.volume_facilitated);
            sample_deltas.push(node.entropy_delta as f64);
            if tick >= node.wake_tick { active_agents_sample += 1; }
            if ui_deltas.len() < MAX_UI_AGENTS { ui_deltas.push(node.entropy_delta); }
        }

        sample_wealth.sort_unstable();
        sample_volume.sort_unstable();

        let sample_w_total = sample_wealth.iter().copied().sum::<u64>() as f64;
        let sample_v_total = sample_volume.iter().copied().sum::<u64>() as f64;
        let gini = calculate_gini_already_sorted(&sample_wealth, sample_w_total);
        let tx_gini = calculate_gini_already_sorted(&sample_volume, sample_v_total);

        let n_s = sample_wealth.len();
        let pct_top1 = if sample_w_total > 0.0 { (sample_wealth[n_s - (n_s as f64 * 0.01) as usize..].iter().copied().sum::<u64>() as f64 / sample_w_total) * 100.0 } else { 0.0 };
        let pct_top20 = if sample_w_total > 0.0 { (sample_wealth[n_s - (n_s as f64 * 0.20) as usize..].iter().copied().sum::<u64>() as f64 / sample_w_total) * 100.0 } else { 0.0 };
        let pct_bot50 = if sample_w_total > 0.0 { (sample_wealth[..(n_s as f64 * 0.50) as usize].iter().copied().sum::<u64>() as f64 / sample_w_total) * 100.0 } else { 0.0 };

        let mean = sample_deltas.iter().sum::<f64>() / 10_000.0;
        let variance_sum: f64 = sample_deltas.iter().map(|&d| (d - mean) * (d - mean)).sum();
        let chaos_variance = (variance_sum / 10_000.0).sqrt();

        if tick % 500 == 0 || self.last_top_100.is_empty() {
            self.wealth_array.clear();
            for shard in &self.shards { for node in &shard.nodes { self.wealth_array.push((node.id, node.vault_books)); } }
            
            let len = self.wealth_array.len();
            if len >= 100 {
                let target_idx = len - 100;
                self.wealth_array.select_nth_unstable_by(target_idx, |a, b| a.1.cmp(&b.1));
                let current_top_100: HashSet<usize> = self.wealth_array[target_idx..].iter().map(|x| x.0).collect();
                if !self.last_top_100.is_empty() {
                    let intersection = current_top_100.intersection(&self.last_top_100).count();
                    self.cached_churn_rate = 100.0 - (intersection as f64);
                }
                self.last_top_100 = current_top_100;
            }
        }

        self.vault_history.push_back(total_vault_books);
        if self.vault_history.len() > 50 { self.vault_history.pop_front(); }
        let oldest_vault_books = *self.vault_history.front().unwrap();
        let inflation_rate = if oldest_vault_books > 0 { ((total_vault_books as f64 - oldest_vault_books as f64) / oldest_vault_books as f64) * 100.0 } else { 0.0 };

        let plasma_books = total_plasma_counts as f64 / CONFIG.total_book_counts as f64;
        let velocity_of_value = if plasma_books > 0.0 { (tick_vol as f64) / plasma_books } else { 0.0 };
        let hash_difficulty = global_pv as f64 / total_vault_books.max(1) as f64;
        let pop_inflation_ratio = (total_vault_books as f64 + plasma_books) / num_agents as f64;
        let active_agents_extrapolated = ((active_agents_sample as f64 / 10_000.0) * num_agents as f64) as u64;

        let mut active_factors: Vec<(usize, u64)> = self.global_factor_multiples.iter().enumerate().filter(|&(_, &sum)| sum > 0).map(|(k, &v)| (k, v)).collect();
        active_factors.par_sort_unstable_by(|a, b| b.1.cmp(&a.1));
        
        let top_factors: Vec<FactorRank> = active_factors.iter().take(3).map(|&(counts, sum_multiples)| FactorRank { counts, sum_multiples }).collect();
        let bottom_factors: Vec<FactorRank> = active_factors.iter().rev().take(2).map(|&(counts, sum_multiples)| FactorRank { counts, sum_multiples }).collect();

        let sim_speed_ms = SIM_CONTROLS.speed_delay_ms.load(Ordering::Relaxed);
        let is_paused = SIM_CONTROLS.is_paused.load(Ordering::Relaxed);

        Some(Telemetry {
            tick, mode: CONFIG.mode.clone(), domain: CONFIG.domain.clone(), active_agents: active_agents_extrapolated, total_vault_books, 
            sublimated_plasma_books: plasma_books, inflation_rate, net_entropy: self.global_net_entropy, void_events: self.void_events, 
            surplus_events: self.surplus_events, total_transfers: self.total_transfers, velocity_of_value, pop_inflation_ratio, 
            avalanche_peak: tick_mints, chaos_variance, churn_rate: self.cached_churn_rate, hash_difficulty, gini_coefficient: gini, 
            tx_gini, pct_wealth_top_1: pct_top1, pct_wealth_top_20: pct_top20, pct_wealth_bottom_50: pct_bot50, agent_deltas: ui_deltas,
            top_factors, bottom_factors, sim_speed_ms, transfer_prob: prob_multiplier, is_paused, mpc_enabled: is_mpc_active,
        })
    }
}

#[inline(always)]
fn calculate_gini_already_sorted(values: &[u64], total: f64) -> f64 {
    if total == 0.0 { return 0.0; }
    let n = values.len() as f64; let mut sum_iy = 0.0;
    for (i, &v) in values.iter().enumerate() { sum_iy += (i + 1) as f64 * (v as f64); }
    (2.0 * sum_iy) / (n * total) - (n + 1.0) / n
}

// ==========================================
// 6. AXUM WEB SERVER & WEBSOCKETS
// ==========================================
struct AppState { tx: broadcast::Sender<String> }

#[tokio::main]
async fn main() {
    let (tx, _rx) = broadcast::channel(16);
    let app_state = Arc::new(AppState { tx: tx.clone() });

    tokio::task::spawn_blocking(move || {
        let mut engine = Engine::new(CONFIG.num_agents);
        loop {
            if SIM_CONTROLS.is_paused.load(Ordering::Relaxed) { std::thread::sleep(Duration::from_millis(50)); continue; }
            let generate_telemetry = engine.tick_count % 20 == 0;
            if let Some(telemetry) = engine.tick(generate_telemetry) { let _ = tx.send(serde_json::to_string(&telemetry).unwrap()); }
            let delay = SIM_CONTROLS.speed_delay_ms.load(Ordering::Relaxed);
            if delay > 0 { std::thread::sleep(Duration::from_millis(delay)); }
        }
    });

    let app = Router::new().route("/", get(serve_dashboard)).route("/ws", get(ws_handler)).with_state(app_state);
    println!("🌐 HPC Arithmodynamic Observatory on http://localhost:3005");
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3005").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn ws_handler(ws: WebSocketUpgrade, AxumState(state): AxumState<Arc<AppState>>) -> impl IntoResponse {
    let rx = state.tx.subscribe();
    ws.on_upgrade(move |socket| handle_socket(socket, rx))
}

async fn handle_socket(mut socket: WebSocket, mut rx: broadcast::Receiver<String>) {
    loop {
        tokio::select! {
            Ok(msg) = rx.recv() => { if socket.send(Message::Text(msg)).await.is_err() { break; } }
            Some(Ok(msg)) = socket.recv() => {
                if let Message::Text(text) = msg {
                    if let Ok(cmd) = serde_json::from_str::<serde_json::Value>(&text) {
                        if let Some(msg_type) = cmd["type"].as_str() {
                            match msg_type {
                                "pause" => { if let Some(val) = cmd["value"].as_bool() { SIM_CONTROLS.is_paused.store(val, Ordering::Relaxed); } }
                                "mpc" => { if let Some(val) = cmd["value"].as_bool() { SIM_CONTROLS.is_mpc_enabled.store(val, Ordering::Relaxed); } }
                                "speed" => { if let Some(val) = cmd["value"].as_u64() { SIM_CONTROLS.speed_delay_ms.store(val, Ordering::Relaxed); } }
                                "prob" => { if let Some(val) = cmd["value"].as_f64() { SIM_CONTROLS.transfer_prob_bits.store(val.to_bits(), Ordering::Relaxed); } }
                                _ => {}
                            }
                        }
                    }
                }
            }
            else => break,
        }
    }
}

async fn serve_dashboard() -> Html<String> {
    let raw_html = r#"
<!DOCTYPE html>
<html>
<head>
    <title>HPC Arithmodynamics Observatory</title>
    <script src="https://cdn.jsdelivr.net/npm/chart.js"></script>
    <style>
        body { background-color: #020617; color: white; font-family: sans-serif; margin: 0; padding: 20px; text-align: center;}
        h1 { color: #eab308; margin-bottom: 5px; font-size: 2rem; text-transform: uppercase; letter-spacing: 2px;}
        .mode-badge { background-color: #3b82f6; color: white; display: inline-block; padding: 5px 15px; border-radius: 20px; font-weight: bold; font-size: 0.8rem; margin-bottom: 20px; }
        .grid-container { display: grid; grid-template-columns: 1fr 1fr; gap: 20px; max-width: 1200px; margin: 0 auto; text-align: left;}
        .panel { background-color: #0f172a; border: 1px solid #334155; border-radius: 8px; padding: 15px; box-shadow: 0 4px 6px rgba(0,0,0,0.5); }
        .panel-header { font-size: 1.1rem; font-weight: bold; color: #cbd5e1; margin-bottom: 15px; border-bottom: 1px solid #334155; padding-bottom: 5px;}
        .stat-row { display: flex; justify-content: space-between; margin-bottom: 8px; font-size: 0.95rem; }
        .stat-val { font-weight: bold; font-family: monospace; font-size: 1.05rem; }
        .chart-container { width: 100%; height: 250px; margin-top: 15px; }
        .agent-grid { display: flex; flex-wrap: wrap; width: 100%; gap: 1px; background-color: #020617; padding: 5px; border-radius: 4px; margin-top: 10px; }
        .agent { width: 6px; height: 6px; background-color: #334155; border-radius: 1px; }
        .pareto-bar { width: 100%; height: 15px; background-color: #334155; border-radius: 4px; display: flex; overflow: hidden; margin-top: 5px; }
        .pareto-segment-top { background-color: #ef4444; height: 100%; transition: width 0.1s; }
        .pareto-segment-mid { background-color: #eab308; height: 100%; transition: width 0.1s; }
        .pareto-segment-bot { background-color: #22c55e; height: 100%; transition: width 0.1s; }
        .tooltip { cursor: help; border-bottom: 1px dotted #64748b; }
    </style>
</head>
<body>
    <h1>HPC Arithmodynamic Observatory</h1>
    <div class="mode-badge" id="mode-badge">MODE: BOOTING...</div>

    <div class="panel" style="max-width: 1170px; margin: 0 auto 20px auto; display: flex; justify-content: space-between; align-items: center; border-color: #3b82f6;">
        <button id="pauseBtn" style="padding: 10px 20px; cursor: pointer; font-weight: bold; background-color: #3b82f6; color: white; border: none; border-radius: 4px; transition: 0.2s;">Pause Simulation</button>
        <button id="mpcBtn" style="padding: 10px 20px; cursor: pointer; font-weight: bold; background-color: #10b981; color: white; border: none; border-radius: 4px; transition: 0.2s;">MPC: ON</button>
        <div style="width: 20%; text-align: center; background-color: #020617; padding: 5px; border-radius: 4px; border: 1px solid #334155;">
            <label style="font-size:0.75rem; color:#94a3b8; text-transform: uppercase;">Real World Time</label><br>
            <span id="realWorldTime" style="color:#10b981; font-weight:bold; font-size:1.1rem; font-family:monospace;">0.00 Hours</span>
        </div>
        <div style="width: 20%;"><label style="font-size:0.9rem;">Loop Delay: <span id="speedVal" style="color:#fff;">0</span> ms</label><input type="range" id="speedSlider" min="0" max="250" value="0" step="5" style="width: 100%;"></div>
        <div style="width: 20%;"><label style="font-size:0.9rem;">Prob Multiplier: <span id="probVal" style="color:#fff;">1.0</span>x</label><input type="range" id="probSlider" min="0" max="10" value="1.0" step="0.1" style="width: 100%;"></div>
    </div>

    <div class="grid-container">
        <!-- MACRO ECONOMICS -->
        <div class="panel">
            <div class="panel-header" id="macro-header" style="color:#fcd34d;">I. Macro Economics (MPC ON)</div>
            <div class="stat-row"><span>Circulating Supply (Books)</span><span class="stat-val" id="vault_books" style="color:#fcd34d;">0</span></div>
            <div class="stat-row"><span>In-Transit Plasma</span><span class="stat-val" id="plasma_books" style="color:#38bdf8;">0.00</span></div>
            <div class="stat-row"><span class="tooltip" title="Rolling 1k-Tick Inflation">Inflation Rate</span><span class="stat-val" id="inflation_rate">0.00%</span></div>
            <div class="stat-row" style="margin-top: 15px;"><span class="tooltip" title="Transaction Volume / Circulating Plasma">Velocity of Value (V)</span><span class="stat-val" id="velocity" style="color:#a855f7;">0.00</span></div>
            <div class="stat-row"><span class="tooltip" title="Total Circulating Books / Population">Supply / Population</span><span class="stat-val" id="pop_inflation" style="color:#fb7185;">0.00</span></div>
            <div class="chart-container"><canvas id="macroChart"></canvas></div>
        </div>

        <!-- THERMODYNAMICS & PHYSICS -->
        <div class="panel">
            <div class="panel-header" style="color:#34d399;">II. Thermodynamics & Compute Engine</div>
            <div class="stat-row"><span>Global Tick</span><span class="stat-val" id="tick">0</span></div>
            <div class="stat-row"><span>Net System Entropy</span><span class="stat-val" id="entropy_val" style="color:#22d3ee;">0</span></div>
            <div class="stat-row"><span>Surplus Peaks / Void Exhaust</span><span class="stat-val"><span id="surplus" style="color:#34d399;">0</span> / <span id="voids" style="color:#fb7185;">0</span></span></div>
            <div class="stat-row" style="margin-top: 15px;"><span class="tooltip" title="Max books minted in a single tick">Avalanche Peak</span><span class="stat-val" id="avalanche" style="color:#fb923c;">0</span></div>
            <div class="stat-row"><span class="tooltip" title="Standard Deviation of node entropy">Lyapunov Chaos Index</span><span class="stat-val" id="chaos" style="color:#c084fc;">0.00</span></div>
            <div class="stat-row"><span class="tooltip" title="Raw Value required per circulating Book">Compute-per-Book Difficulty</span><span class="stat-val" id="hash_diff" style="color:#94a3b8;">0.00</span></div>
            <div class="chart-container"><canvas id="chaosChart"></canvas></div>
        </div>

        <!-- SOCIO-ECONOMICS & NETWORK MOBILITY -->
        <div class="panel" style="grid-column: 1 / -1;">
            <div class="panel-header" style="color:#ef4444;">III. Network Dynamics & Rankings</div>
            <div style="display:flex; justify-content: space-between;">
                <div style="width: 32%;">
                    <div class="stat-row"><span class="tooltip" title="Inequality of Stored Wealth">Wealth Gini</span><span class="stat-val" id="gini_text" style="color:#ef4444; font-size: 1.2rem;">0.00</span></div>
                    <div class="stat-row"><span>Top 1% Controls</span><span class="stat-val" id="p_top1">0.00%</span></div>
                    <div class="pareto-bar">
                        <div class="pareto-segment-top" id="bar_top20"></div><div class="pareto-segment-mid" id="bar_mid30"></div><div class="pareto-segment-bot" id="bar_bot50"></div>
                    </div>
                </div>
                <div style="width: 32%;">
                    <div class="stat-row"><span class="tooltip" title="Inequality of Transfer Volume">Transaction Gini</span><span class="stat-val" id="tx_gini" style="color:#3b82f6; font-size: 1.2rem;">0.00</span></div>
                    <div class="stat-row"><span class="tooltip" title="% of Top 100 wealthiest nodes that are 'new money' every 500 ticks">Churn Rate</span><span class="stat-val" id="churn" style="color:#10b981;">100%</span></div>
                </div>
                <div style="width: 32%;">
                    <div class="stat-row"><span class="tooltip" title="Most transferred prime factors (Counts)">Top 3 GPF (Counts)</span></div>
                    <div id="top_factors" style="font-size: 0.85rem; margin-bottom: 10px; color:#38bdf8;"></div>
                    <div class="stat-row"><span class="tooltip" title="Least transferred prime factors (Counts)">Bottom 2 GPF (Counts)</span></div>
                    <div id="bottom_factors" style="font-size: 0.85rem; color:#fb7185;"></div>
                </div>
            </div>
            <div class="agent-grid" id="agentGrid"></div>
        </div>
    </div>

    <script>
        const ctxMac = document.getElementById('macroChart').getContext('2d');
        const macroChart = new Chart(ctxMac, { type: 'line', data: { labels:[], datasets:[{ label: 'Circulating Supply', borderColor: '#fcd34d', data: [], fill: false, tension: 0.1, yAxisID: 'y' }, { label: 'Velocity (V)', borderColor: '#a855f7', data:[], fill: false, tension: 0.1, yAxisID: 'y1' }]}, options: { responsive: true, maintainAspectRatio: false, animation: false, scales: { x: { display: false }, y: { display: true }, y1: { position: 'right', display: true, grid:{drawOnChartArea:false} } } } });
        const ctxChaos = document.getElementById('chaosChart').getContext('2d');
        const chaosChart = new Chart(ctxChaos, { type: 'line', data: { labels:[], datasets:[{ label: 'Avalanche Peaks', borderColor: '#fb923c', data: [], fill: false, tension: 0.1, yAxisID: 'y' }, { label: 'Lyapunov Chaos', borderColor: '#c084fc', data:[], fill: false, tension: 0.1, yAxisID: 'y1' }]}, options: { responsive: true, maintainAspectRatio: false, animation: false, scales: { x: { display: false }, y: { display: true }, y1: { position: 'right', display: true, grid:{drawOnChartArea:false} } } } });

        const grid = document.getElementById('agentGrid');
        const agents =[];
        for(let i=0; i<2500; i++) { const div = document.createElement('div'); div.className = 'agent'; grid.appendChild(div); agents.push(div); }

        const ws = new WebSocket("ws://" + window.location.host + "/ws");
        
        let isDraggingSpeed = false; let isDraggingProb = false;
        const speedSlider = document.getElementById('speedSlider'); const probSlider = document.getElementById('probSlider');
        const pauseBtn = document.getElementById('pauseBtn');
        const mpcBtn = document.getElementById('mpcBtn');

        speedSlider.addEventListener('mousedown', () => isDraggingSpeed = true); speedSlider.addEventListener('mouseup', () => isDraggingSpeed = false);
        speedSlider.addEventListener('touchstart', () => isDraggingSpeed = true); speedSlider.addEventListener('touchend', () => isDraggingSpeed = false);
        probSlider.addEventListener('mousedown', () => isDraggingProb = true); probSlider.addEventListener('mouseup', () => isDraggingProb = false);
        probSlider.addEventListener('touchstart', () => isDraggingProb = true); probSlider.addEventListener('touchend', () => isDraggingProb = false);
        
        let localPauseState = false;
        pauseBtn.addEventListener('click', (e) => {
            localPauseState = !localPauseState; 
            if (ws.readyState === WebSocket.OPEN) { ws.send(JSON.stringify({ type: "pause", value: localPauseState })); }
        });

        let localMpcState = true;
        mpcBtn.addEventListener('click', (e) => {
            localMpcState = !localMpcState; 
            if (ws.readyState === WebSocket.OPEN) { ws.send(JSON.stringify({ type: "mpc", value: localMpcState })); }
        });
        
        speedSlider.addEventListener('input', (e) => { 
            document.getElementById('speedVal').innerText = e.target.value; 
            if (ws.readyState === WebSocket.OPEN) ws.send(JSON.stringify({ type: "speed", value: parseInt(e.target.value) })); 
        });
        
        probSlider.addEventListener('input', (e) => { 
            document.getElementById('probVal').innerText = parseFloat(e.target.value).toFixed(1); 
            if (ws.readyState === WebSocket.OPEN) ws.send(JSON.stringify({ type: "prob", value: parseFloat(e.target.value) })); 
        });
        
        ws.onmessage = function(event) {
            const data = JSON.parse(event.data);
            
            localPauseState = data.is_paused;
            pauseBtn.innerText = localPauseState ? "Resume Simulation" : "Pause Simulation"; 
            pauseBtn.style.backgroundColor = localPauseState ? '#ef4444' : '#3b82f6';

            localMpcState = data.mpc_enabled;
            mpcBtn.innerText = localMpcState ? "MPC: ON" : "MPC: OFF";
            mpcBtn.style.backgroundColor = localMpcState ? '#10b981' : '#64748b';
            document.getElementById('macro-header').innerText = localMpcState ? "I. Macro Economics (MPC ON)" : "I. Macro Economics (Zero Intelligence)";

            if (!isDraggingSpeed) { speedSlider.value = data.sim_speed_ms; document.getElementById('speedVal').innerText = data.sim_speed_ms; }
            if (!isDraggingProb) { let probMult = data.transfer_prob.toFixed(1); probSlider.value = probMult; document.getElementById('probVal').innerText = probMult; }
            
            const counts_per_tick = 1; let hours = (data.tick * 2 * counts_per_tick) / 3600.0; let timeStr = "";
            if (hours < 24) { timeStr = hours.toFixed(2) + " Hours"; } else if (hours < 730) { timeStr = (hours / 24).toFixed(2) + " Days"; } 
            else if (hours < 8760) { timeStr = (hours / 730).toFixed(2) + " Months"; } else { timeStr = (hours / 8760).toFixed(2) + " Years"; }
            
            document.getElementById('realWorldTime').innerText = timeStr;
            document.getElementById('mode-badge').innerText = `MODE: ${data.mode.toUpperCase()} / ${data.domain.toUpperCase()} (~${data.active_agents.toLocaleString()} Nodes)`;
            document.getElementById('tick').innerText = data.tick.toLocaleString();
            document.getElementById('vault_books').innerText = data.total_vault_books.toLocaleString();
            document.getElementById('plasma_books').innerText = data.sublimated_plasma_books.toLocaleString(undefined, {minimumFractionDigits: 2});
            document.getElementById('entropy_val').innerText = data.net_entropy.toLocaleString();
            document.getElementById('voids').innerText = data.void_events.toLocaleString();
            document.getElementById('surplus').innerText = data.surplus_events.toLocaleString();
            
            const infEl = document.getElementById('inflation_rate');
            infEl.innerText = (data.inflation_rate > 0 ? '+' : '') + data.inflation_rate.toFixed(3) + '%';
            infEl.style.color = data.inflation_rate > 0 ? '#fb7185' : '#34d399';

            document.getElementById('velocity').innerText = data.velocity_of_value.toFixed(4);
            document.getElementById('pop_inflation').innerText = data.pop_inflation_ratio.toFixed(4);
            document.getElementById('avalanche').innerText = data.avalanche_peak.toLocaleString();
            document.getElementById('chaos').innerText = data.chaos_variance.toFixed(2);
            document.getElementById('hash_diff').innerText = data.hash_difficulty.toLocaleString(undefined, {maximumFractionDigits: 0});
            document.getElementById('churn').innerText = data.churn_rate.toFixed(1) + '%';
            
            document.getElementById('gini_text').innerText = data.gini_coefficient.toFixed(3);
            document.getElementById('tx_gini').innerText = data.tx_gini.toFixed(3);
            document.getElementById('p_top1').innerText = data.pct_wealth_top_1.toFixed(2) + '%';
            document.getElementById('bar_top20').style.width = data.pct_wealth_top_20 + '%';
            document.getElementById('bar_bot50').style.width = data.pct_wealth_bottom_50 + '%';
            document.getElementById('bar_mid30').style.width = Math.max(0, 100 - data.pct_wealth_top_20 - data.pct_wealth_bottom_50) + '%';

            let topHtml = ""; data.top_factors.forEach((f, i) => { topHtml += `<div style="display:flex; justify-content:space-between; border-bottom: 1px solid #1e293b; padding: 2px 0;"><span>#${i+1} [${f.counts} c]</span><span style="font-weight:bold; font-family:monospace;">${f.sum_multiples.toLocaleString()}</span></div>`; });
            document.getElementById('top_factors').innerHTML = topHtml;

            let botHtml = ""; data.bottom_factors.forEach((f, i) => { botHtml += `<div style="display:flex; justify-content:space-between; border-bottom: 1px solid #1e293b; padding: 2px 0;"><span>Low [${f.counts} c]</span><span style="font-weight:bold; font-family:monospace;">${f.sum_multiples.toLocaleString()}</span></div>`; });
            document.getElementById('bottom_factors').innerHTML = botHtml;

            macroChart.data.labels.push(data.tick); macroChart.data.datasets[0].data.push(data.total_vault_books); macroChart.data.datasets[1].data.push(data.velocity_of_value);
            if(macroChart.data.labels.length > 50) { macroChart.data.labels.shift(); macroChart.data.datasets.forEach(ds => ds.data.shift()); } macroChart.update();
            chaosChart.data.labels.push(data.tick); chaosChart.data.datasets[0].data.push(data.avalanche_peak); chaosChart.data.datasets[1].data.push(data.chaos_variance);
            if(chaosChart.data.labels.length > 50) { chaosChart.data.labels.shift(); chaosChart.data.datasets.forEach(ds => ds.data.shift()); } chaosChart.update();

            data.agent_deltas.forEach((delta, i) => {
                if(!agents[i]) return;
                const el = agents[i];
                if (delta > 0) { el.style.backgroundColor = '#34d399'; } else if (delta < 0) { el.style.backgroundColor = '#fb7185'; } else { el.style.backgroundColor = '#334155'; }
            });
        };
    </script>
</body>
</html>
"#;
    Html(raw_html.to_string())
}
