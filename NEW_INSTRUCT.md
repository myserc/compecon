# ARCHITECTURAL DIRECTIVE: FULL SYSTEM MIGRATION (JAVA TO RUST ECS)

**To:** Google Jules Agent
**From:** Senior Systems Architect
**Context:** We have successfully laid the engine block for the SOTA Agent-Based Computational Economy (ACE) in Rust. We have an ECS (`hecs`), a multi-threaded parallel tick loop (`rayon` + `crossbeam`), and the Arithmodynamic PV money system. 
**Objective:** Complete the migration. Achieve 100% feature parity with the legacy Java OOP codebase, but do it idiomatically in Rust using Data-Oriented Design (DOD). You will map the deep Java class hierarchies (Agents, Markets, Financials, Behaviors, Math) into flat, contiguous ECS components and disjoint systems.

---

## 🛑 STRICT RULES OF ENGAGEMENT
1.  **NO OOP:** Do not use `Box<dyn Trait>` for agents, behaviors, or properties. Use components and enums. 
2.  **CACHE LOCALITY:** Replace `HashMap<GoodType, f64>` with `[f64; GOOD_TYPE_COUNT]`. Map the `GoodType` enum variants to `usize` indices.
3.  **THE BRIDGE:** "Mental" calculations (utility, production, price expectations) use `f64`. Actual ledger transactions (bank balances, bond face values, cash) use Arithmodynamic Prime Value (`u64` PV).
4.  **DETERMINISM:** Use a seeded pseudo-random number generator (e.g., `rand_chacha`) initialized in the world resource to replace Java's `StochasticNumberGeneratorImpl`.
5.  **SYSTEM COMMUNICATION:** Systems communicate intents via `crossbeam::channel` or deferred command queues. Only one system mutates wallets/ledgers to prevent race conditions.

---

## EXECUTION PLAN: PHASE BY PHASE

You must implement the following phases sequentially. Do not skip any Java features.

### PHASE 1: Time, Core Entities, and Arrays
The Java engine relies heavily on `TimeSystemImpl` (Hours, Days, Months).
*   [ ] **Chronos Resource:** Create a `TimeSystem` struct (held globally or passed to systems) that tracks `tick`, `hour` (0-23), `day`, `month`, and `year`. 
*   [ ] **Event Scheduling:** Instead of Java's `TimeSystemEvent` callbacks, implement time-gated execution in systems. (e.g., `if time.hour == 23 { execute_balance_sheet_system(...) }`).
*   [ ] **Inventory Optimization:** Implement an `Inventory` component as a struct wrapping `[f64; 10]` (matching the 10 `GoodType` variants). Implement helper methods to safely get/set by `GoodType`.
*   [ ] **Property & Ownership:** `Share` and `Bond` (Fixed/ZeroCoupon) are no longer OOP classes. They are entities. Create `Issuer(Entity)` and `Owner(Entity)` components, alongside `BondData { face_value_pv: u64, coupon_pv: u64, term_ticks: u64 }`.

### PHASE 2: Neoclassical Math & Behaviors
Java's `math` package has complex optimization logic (`CESFunctionImpl`, `CobbDouglasFunctionImpl`, `RootFunctionImpl`). 
*   [ ] **Production & Utility Components:** Create `ProductionFunction` and `UtilityFunction` enums/structs holding the coefficients and exponents (`alphas`).
*   [ ] **Optimization Solvers:** Port the analytical partial derivative logic (`calculateOutputMaximizingInputsAnalyticalWithFixedPrices` and step-function solvers) to Rust. This must be a pure, side-effect-free math module.
*   [ ] **Pricing Behavior Component:** Port `PricingBehaviourImpl`. Create a `PricingStrategy` component tracking `last_offered`, `last_sold`, `current_price_f64`, and `price_change_increment`. Implement the state machine (Sold Nothing -> drop price; Sold Everything -> raise price).
*   [ ] **Budgeting Behavior:** Port `BudgetingBehaviourImpl` and the `IntertemporalConsumptionFunction` (Modigliani/Irving Fisher) used by Households for retirement savings logic.

### PHASE 3: The Market & Order Books
The Java `SettlementMarketServiceImpl` matches buyers and sellers via step-functions.
*   [ ] **Order Book Resource:** Create a centralized `Market` struct holding `BTreeMap<OrderedFloat<f64>, Vec<SellOrder>>` partitioned by `GoodType` and `Currency`.
*   [ ] **Market Intent System (Parallel):** In Phase 1 of the tick, agents (Households/Factories) evaluate the order book's marginal prices and emit `BuyIntent` or `SellIntent` to a lock-free channel.
*   [ ] **Market Clearing System (Sequential):** In Phase 2, the market processes intents. It matches bids and asks, generates `TransferIntent`s for the Arithmodynamic engine, and updates the `MarketDepth` telemetry.
*   [ ] **Price Discovery:** The market must expose a method to calculate the marginal price of a good at a specific volume (porting Java's `getMarginalMarketPrice`).

### PHASE 4: Agents, Banks, and the State
Migrate the specific logic for all 5 agent types.
*   [ ] **Factory System:** Buy inputs (Machine + Labour) -> Process `ProductionFunction` -> Produce output -> Place sell orders. Apply capital depreciation.
*   [ ] **Household System:** Supply Labour -> Earn PV Wage/Dividends -> Save/Consume (Modigliani) -> Buy consumer goods -> Generate Utility. Handle lifecycle (aging, retirement, death/spawning).
*   [ ] **Trader System:** Import/Arbitrage goods between markets based on `TraderConfig`.
*   [ ] **State System:** Collect taxes (if applicable), execute deficit spending (buy goods from market), and issue `FixedRateBond` entities to finance operations.
*   [ ] **CreditBank System:** Port `CreditBankImpl`. Accept deposits, issue loans, calculate interest (translating f64 interest rates into discrete PV adjustments), and buy State bonds. 

### PHASE 5: Accounting, Statistics & Telemetry
The Java engine has a massive `ModelRegistry` tracking everything from M0/M1/M2 to the Gini coefficient.
*   [ ] **Balance Sheets:** Implement a `BalanceSheet` component. At the designated `TimeSystem` hour, an `accounting_system` calculates Assets (Hard Cash PV, Deposits, Inventory value at current marginal prices) and Liabilities (Loans, Issued Bonds) to derive Equity.
*   [ ] **Macro Accumulators:** Create a `StatisticsRegistry` resource. Port the `PeriodDataAccumulator` logic. Track:
    *   M0, M1, M2 Money Supply.
    *   Money Circulation & Velocity.
    *   Total Net Entropy (from Arithmodynamic nodes).
    *   Lorenz Curve / Income Distribution (Gini).
    *   Total Economy Utility.
*   [ ] **Parquet Sink:** Extend the existing `telemetry::dump_nodes_to_parquet` to write these macro-statistics to Parquet files periodically.

### PHASE 6: Axum WebSocket Dashboard (The UI Replacement)
Replace the Java Swing UI (`Dashboard.java`, `HouseholdsPanel.java`, etc.) with a modern streaming backend.
*   [ ] **Axum Integration:** Add `tokio`, `axum`, and `tungstenite` to `Cargo.toml`.
*   [ ] **WebSocket Server:** Spawn an asynchronous Tokio task alongside the synchronous `hecs` tick loop. 
*   [ ] **State Broadcasting:** At the end of every "Day" tick, extract the `StatisticsRegistry`, serialize it to JSON, and broadcast it over the Axum WebSocket to any connected frontend clients. 
*   [ ] **Control Endpoints:** Expose Axum HTTP POST routes to allow external triggers equivalent to Java's `ControlModel.java` (e.g., `/api/control/deficit_spending`, `/api/control/economic_shock`). Send these commands to the ECS via a `crossbeam` control channel.

---

## 🛠 HOW TO PROCEED
Begin immediately by scaffolding **Phase 1 (Time & Inventory)** and **Phase 2 (Math & Behaviors)**. Treat the Java codebase as a specification document for the mathematical logic, but throw away its object-oriented patterns entirely. 

Once Phase 1 and 2 compile and pass unit tests, proceed to **Phase 3 (Order Books)**. 

Write robust, idiomatic, high-performance Rust. Leave inline comments linking complex logic back to the original Java class/method names for auditability.
