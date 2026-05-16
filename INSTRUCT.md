# ARCHITECTURAL DIRECTIVE: SOTA Arithmodynamic Computational Economy (Rust)
**To: Google Jules Agent**
**Objective:** Rewrite the legacy OOP Java compecon engine into a State of the Art (SOTA) Agent-Based Computational Economy (ACE) in Rust. The engine must fuse continuous Neoclassical decision-making (Cobb-Douglas/CES) with a discrete, thermodynamic monetary physics layer (Arithmodynamics).
## CORE PHILOSOPHY
You are abandoning Object-Oriented Programming completely. Do not create deep struct hierarchies. You will employ strict Data-Oriented Design (DOD) using an Entity-Component-System (ECS) architecture.
**Strict Exclusions:** Do NOT implement formal verification proofs (e.g., Kani, Lean 4, Proptest). Rely on robust Rust type-safety and standard testing.
## PHASE 1: Memory Architecture (ECS / SoA)
Do not use Box<dyn Agent>. Memory must be contiguous to maximize CPU cache coherency. You may use a lightweight custom Struct-of-Arrays (SoA) layout or an optimized ECS framework (like Bevy's core ECS or hecs).
**1. Entities:** Agents are just u32 or usize indices.
**2. Components:**
 * ArithmodynamicNode (The Wallet): prime_value: u64, counts: u32, vault_books: u32, active_book_counts: u32, balance_adjustment: u64, entropy_delta: i64.
 * Inventory: HashMap<GoodType, f64> (or a flat array mapped to a GoodType enum).
 * NeoclassicalBrain: Holds pricing behavior states, production functions, or utility functions.
   **3. Systems:** Logic is executed via disjoint parallel iteration over component arrays.
## PHASE 2: The Arithmodynamic Physics Engine
Money is topological, not a fiat float. All currency is PV (Prime Value) stored strictly as u64.
**1. Topology Sieve (lazy_static or OnceLock):**
 * Precompute a prime sieve up to 5,000,000.
 * Generate an O(1) lookup table mapping any u64 value to its topological sequence index (its count).
**2. The Metronome System (Heartbeat):**
 * A global loop that touches *every* ArithmodynamicNode each tick.
 * **Sublimation:** If active_book_counts == 0 and vault_books > 0, decrement vault_books and grant 180 active_book_counts.
 * **Kinetic Traversal:** If active_book_counts > 0, decrement it, increment counts, and jump to the next prime sequence value.
**3. Crystallization (Phase Transition):**
 * Minting threshold = 1069.
 * If prime_value >= 1069, vault_books += 1, prime_value -= 1069. Recalculate topological counts.
**4. Thermodynamic Transfers & GPF:**
 * Transactions calculate **Greatest Prime Factor (GPF)** of the transfer volume. Log the factor occurrences to analyze network dynamics.
 * **Entropy Generation:** Calculate the ordinal leap across the prime sieve.
   * *Target Leap:* new_target_counts - old_target_counts
   * *Source Leap:* old_source_counts - new_source_counts
   * Accumulate these as entropy_delta.
## PHASE 3: The Neoclassical "Brains"
Agents use fractional math (f64) to *think*, but quantum integers (u64) to *transact*.
**1. Optimization Logic:**
 * Implement CobbDouglas and CES functions for both Utility (Households) and Production (Factories).
 * Agents use partial derivatives to evaluate marginal revenue vs. marginal cost.
 * *Bridge Rule:* If a brain decides to buy 2.75 units of Wheat at 10.5 PV each, the Inventory component receives 2.75 Wheat, but the exact PV transferred must be rounded to a strict u64 (e.g., 29 PV).
**2. Demographic & Spawning Systems:**
 * Households spawn with 0 PV and rely on selling LABOURHOUR to factories to survive.
 * If a household hits 60 ticks without meeting utility constraints (Thermodynamic Freeze), delete the entity.
 * Households that survive past mature thresholds spawn new entities.
## PHASE 4: Concurrency & The Execution Loop
Simulating millions of agents requires lock-free parallelism. Use rayon for data-parallelism and crossbeam channels for state-safe message passing.
Implement a strict **3-Phase Tick Loop**:
 1. **Phase 1 (Evaluation & Intent):** Use par_iter_mut() over agent brains. Agents evaluate local market prices, calculate utility, and emit a TransferIntent { to_id, amount } into a lock-free multi-producer channel. *No wallets are mutated here.*
 2. **Phase 2 (Sequential Materialization):** The engine processes the channel inbox. Wallets are mutated sequentially (or via isolated shards/spatial partitions) to prevent double-spend race conditions. GPF and Entropy are calculated here.
 3. **Phase 3 (Macro Reduction):** Reduce all entropy_delta and GPF counts into global macroeconomic statistics (Inflation, Gini Coefficient, Lyapunov Chaos Index, Velocity of Value).
## PHASE 5: Telemetry & I/O
Do NOT use strings, stdout logging, or CSV writes inside the simulation loop. This will cripple throughput.
 * Use serde and serialize state dumps to **Apache Parquet** format via the parquet crate, or write a highly optimized binary columnar output.
 * For real-time UI viewing (if building a dashboard hook), pipe macro-statistics over an asynchronous tokio WebSocket (axum) exactly like the abm.txt spec.
## STRICT ANTI-PATTERNS (DO NOT USE)
 * f64 or f32 for prime_value or currency balances.
 * Box<dyn Agent> traits storing data on the heap.
 * Mutex or RwLock around individual agents (will cause massive thread contention).
 * Central Banks, Fiat Interest Rates, or Fractional Reserve lending. The Metronome controls the money base.

