***

# FULL MIGRATION DIRECTIVE: COMPECON JAVA -> RUST ECS
**To:** Google Jules Agent
**From:** Senior Systems Architect
**Status:** Phase 1 (Basic ECS & PV) Complete. 
**Objective:** Execute Phases 2-6 to achieve 100% feature parity with the Java simulation. Translate object-oriented hierarchies into flat, contiguous Data-Oriented Design (DOD). 

---

## PHASE 2: ADVANCED FINANCIAL INSTRUMENTS & ECS COMPONENTS
In Java, ownership is tracked via object references (`PropertyOwner`, `BankAccountDelegate`). In Rust ECS, ownership is tracked via Entity IDs and relationships.

### 2.1 The Ledger & Financial Components
Create `src/economy/finance.rs`. Remove Java's `BankAccountImpl` object graph. 

```rust
use hecs::Entity;

#[derive(Debug, Clone)]
pub enum AccountType { Transactions, Savings, Dividends, BondLoans }

#[derive(Debug, Clone)]
pub struct BankAccount {
    pub owner: Entity,
    pub bank: Entity,
    pub account_type: AccountType,
    pub balance_pv: u64,
    pub overdraft_allowed: bool,
}

#[derive(Debug, Clone)]
pub struct FixedRateBond {
    pub issuer: Entity,
    pub owner: Entity,
    pub face_value_pv: u64,
    pub coupon_rate: f64, 
    pub maturity_tick: u64,
}

#[derive(Debug, Clone)]
pub struct Share {
    pub issuer: Entity,
    pub owner: Entity,
}
```

### 2.2 The Neoclassical Mathematical Solvers
Java relies on `AnalyticalConvexFunctionImpl.java` to solve step-function price constraints. 
In `src/math/optimize.rs`, implement the analytical solvers for Cobb-Douglas and CES under budget constraints.

**Jules Directive:** 
Port Java's `calculateOutputMaximizingInputsAnalyticalWithMarketPrices`.
1.  **Input:** Available Budget (`f64`), Prices (`[f64; 10]`), Exponents/Coefficients (`[f64; 10]`).
2.  **Output:** Optimal purchase volumes (`[f64; 10]`).
3.  *Note:* Java handles `Double.NaN` for missing goods. In Rust, use `Option<f64>` for prices. If a price is `None`, the allocated volume for that good is strictly `0.0`.

---

## PHASE 3: THE STEP-FUNCTION MARKET ENGINE
The Java `SettlementMarketServiceImpl.java` clears the market by matching buyers and sellers based on marginal prices.

**Jules Directive:** Implement `src/engine/market.rs`. 
Do **not** use OOP MarketOrders. Use a flattened Order Book.

```rust
use hecs::Entity;
use std::collections::BTreeMap;
use ordered_float::OrderedFloat; // Add to Cargo.toml

pub struct SellOrder {
    pub seller: Entity,
    pub amount: f64,
    pub price_per_unit_f64: f64, 
}

pub struct MarketBook {
    // Sorted lowest price to highest
    pub asks: BTreeMap<OrderedFloat<f64>, Vec<SellOrder>>,
}

impl MarketBook {
    /// Port of Java's findBestFulfillmentSet
    pub fn execute_buy(&mut self, buyer: Entity, max_budget_pv: u64, max_price: f64, max_amount: f64) -> (f64, u64) {
        let mut budget_left_f64 = max_budget_pv as f64;
        let mut amount_acquired = 0.0;
        let mut total_spent_pv = 0;

        // Iterate orders lowest price first
        // 1. Calculate how much can be bought without exceeding max_amount OR budget_left
        // 2. Round the actual spent money to u64 PV for the Arithmodynamic transfer
        // 3. Mutate the SellOrder remaining amount. If 0, remove it.
        // 4. Return (amount_acquired, total_spent_pv)
        
        // ... Jules: Implement the exact step-function loop from Java ...
        (amount_acquired, total_spent_pv)
    }
}
```

---

## PHASE 4: THE AGENT SYSTEMS (THE ECONOMY LOOP)
Java triggers agents via `TimeSystemEvent`. In Rust, we use explicit ECS system phases inside `Simulation::tick()`.

### 4.1 System: Household Lifecycle (Modigliani Consumption)
**Jules Directive:** In `src/engine/systems/household.rs`:
1.  **Wage & Dividends:** Transfer received PV from `Transactions` account to the `Savings` account based on `ModiglianiIntertemporalConsumptionFunction`.
2.  **Optimize:** Call the CES/Cobb-Douglas solver to get the optimal consumer goods basket.
3.  **Market Action:** Emit `BuyIntent`s to the Market.
4.  **Utility & Survival:** If utility hits constraints, reset `ticks_since_utility_met`. If `ticks >= 60 days`, despawn the Entity.

### 4.2 System: Factory Production & Budgeting
**Jules Directive:** In `src/engine/systems/factory.rs`:
1.  **Depreciation:** Reduce durable goods (MACHINE, REALESTATE) by `capitalDepreciationRatioPerPeriod`.
2.  **Credit Budgeting:** Calculate transmission-based budget (Base PV + Credit Capacity from Bank).
3.  **Production Optimization:** Solve for profit-maximizing inputs where Marginal Cost = Marginal Revenue.
4.  **Market Action:** Emit `BuyIntent`s for inputs. Produce output. Emit `SellOrder`s to the `MarketBook`.

### 4.3 System: State & Central Bank
**Jules Directive:** In `src/engine/systems/state.rs`:
1.  **Deficit Spending:** If enabled via config, the State generates `BuyIntent`s for FOOD/WHEAT and pays with newly minted `FixedRateBond`s.
2.  **Coupons:** Every "Year" (or specific tick), iterate all `FixedRateBond`s. Transfer PV from State account to `owner` account.

### 4.4 System: Credit Banks (Fractional Reserve)
**Jules Directive:** In `src/engine/systems/finance.rs`:
1.  **Interest:** Every `Hour::02`, apply `dailyInterestRate` to all `BankAccount`s. If balance > 0, bank pays customer. If balance < 0 (loan), customer pays bank.
2.  **Bond Trading:** Banks scan their aggregate customer deposits. They use excess reserves to buy `FixedRateBond`s from the State.

### 4.5 System: Traders (Arbitrage)
**Jules Directive:** In `src/engine/systems/trader.rs`:
1.  Traders act as price takers. They scan market marginal prices.
2.  If `price < normal`, they buy. If `price > normal + margin`, they sell.

---

## PHASE 5: ACCOUNTING & MACRO-STATISTICS
Java tracks this in `BalanceSheetsModel.java` and `PeriodDataAccumulator`.

**Jules Directive:** Create `src/engine/statistics.rs`.
Implement a system that runs at `tick % 24 == 23` (matching Java's `HOUR_23` config).
1.  **Iterate all Entities:** Calculate Assets (PV balance + inventory value at marginal prices + bond face values). Calculate Liabilities (Loans + Issued Bonds).
2.  **Aggregate Metrics:** 
    *   `M0`: Sum of all `ArithmodynamicNode.prime_value`.
    *   `M1/M2`: Sum of all `BankAccount` balances.
    *   `Gini`: Sort all PV balances, calculate Gini formula.
    *   `Total Net Entropy`: Sum of `entropy_delta`.

---

## PHASE 6: AXUM WEBSOCKET DASHBOARD (UI REPLACEMENT)
Java used `Dashboard.java` with Swing/JFreeChart. We replace this entirely with a modern reactive web stack.

**Jules Directive:** In `src/engine/dashboard.rs`:
1.  Maintain the `axum` router established in the skeleton.
2.  **Payload Expansion:** The JSON sent over the WebSocket must include the full `ModelRegistry` parity:
    ```json
    {
      "tick": 12450,
      "m0": 5400000,
      "m1": 12000000,
      "gini": 0.42,
      "prices": { "WHEAT": 10.5, "COAL": 45.2, "LABOURHOUR": 15.0 },
      "market_depth": { "WHEAT": 500.0, "COAL": 120.0 },
      "bank_reserves": 450000
    }
    ```
3.  **Control Endpoints:** Wire the `/api/control/shock` endpoint to actually loop over `Factory` entities and modify their `ProductionFunction.main_coefficient` by `±5%` to simulate growth/contraction exactly as `ControlModel.java` did.

---

## EXECUTION ORDER FOR JULES
1. Update `economy/mod.rs` to include all Component structs (`BankAccount`, `FixedRateBond`, `Share`, `MarketBook`).
2. Write `math/optimize.rs` to port the analytical partial derivative logic.
3. Rewrite `engine/mod.rs` `tick()` function to execute the 5 distinct sector systems strictly in order (Households -> Factories -> Banks -> State -> Traders -> Market Clearing -> Arithmodynamic PV Updates -> Demographics).
4. Expand `dashboard.rs` to parse the new comprehensive statistics.

*Proceed with generating the code files matching these architectural directives.*
