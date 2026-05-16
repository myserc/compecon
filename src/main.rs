pub mod arithmodynamics;
pub mod economy;
pub mod engine;
pub mod math;
pub mod telemetry;

use crate::engine::Simulation;

fn main() {
    println!("Initializing Arithmodynamic Engine...");
    // Force lazy initialization
    let primes = arithmodynamics::get_primes();
    println!("Primes initialized. Total primes up to {}: {}", arithmodynamics::LIMIT, primes.len());

    let mut sim = Simulation::new();
    sim.spawn_household(1000);
    sim.spawn_factory(5000, economy::GoodType::BREAD);

    println!("Simulation initialized with {} entities.", sim.world.len());

    for _ in 0..200 {
        sim.tick();
    }
    println!("Simulation reached tick {}.", sim.tick);

    let mut nodes = Vec::new();
    for (_entity, node) in sim.world.query::<&arithmodynamics::ArithmodynamicNode>().iter() {
        nodes.push(node.clone());
    }
    telemetry::dump_nodes_to_parquet(&nodes, "state_dump.parquet").expect("Failed to dump state");
    println!("State dumped to state_dump.parquet");
}
