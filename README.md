# SOTA Arithmodynamic Computational Economy (Rust)

This is a State of the Art (SOTA) Agent-Based Computational Economy (ACE) implemented in Rust, migrated from a legacy Java OOP engine.

## Features
- **ECS Architecture**: Uses `hecs` for high-performance Entity Component System.
- **Data-Oriented Design**: Maximizes CPU cache coherency with contiguous memory layout.
- **Arithmodynamics**: Implements a thermodynamic monetary physics layer where money is topological (based on prime numbers).
- **Neoclassical Brains**: Agents use Cobb-Douglas and CES functions for decision-making.
- **Parallel Execution Loop**: Uses `rayon` for data-parallelism and `crossbeam` for safe message passing.
- **Telemetry**: State dumps to Apache Parquet for high-throughput analysis.

## Running the Simulation
```bash
cargo run
```

## Testing
```bash
cargo test
```
