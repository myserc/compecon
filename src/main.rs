pub mod arithmodynamics;
pub mod economy;
pub mod engine;
pub mod math;
pub mod telemetry;

use crate::engine::Simulation;

#[tokio::main]
async fn main() {
    println!("Initializing Arithmodynamic Engine...");
    // Force lazy initialization
    let primes = arithmodynamics::get_primes();
    println!("Primes initialized. Total primes up to {}: {}", arithmodynamics::LIMIT, primes.len());

    let mut sim = Simulation::new();

    let dashboard_state = std::sync::Arc::new(engine::dashboard::DashboardState {
        last_stats: std::sync::Mutex::new(sim.stats.clone()),
    });

    let dashboard_state_clone = dashboard_state.clone();
    tokio::spawn(async move {
        engine::dashboard::start_dashboard(dashboard_state_clone).await;
    });

    sim.spawn_household(1000);
    sim.spawn_factory(5000, economy::GoodType::FOOD);

    println!("Simulation initialized with {} entities.", sim.world.len());

    let mut stats_history = Vec::new();

    for _ in 0..1000 {
        sim.tick();
        stats_history.push(sim.stats.clone());
        {
            let mut last_stats = dashboard_state.last_stats.lock().unwrap();
            *last_stats = sim.stats.clone();
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    println!("Simulation reached tick {}.", sim.time.tick);

    let mut nodes = Vec::new();
    for (_entity, node) in sim.world.query::<&arithmodynamics::ArithmodynamicNode>().iter() {
        nodes.push(node.clone());
    }
    telemetry::dump_nodes_to_parquet(&nodes, "state_dump.parquet").expect("Failed to dump state");
    println!("State dumped to state_dump.parquet");

    telemetry::dump_stats_to_parquet(&stats_history, "macro_stats.parquet").expect("Failed to dump stats");
    println!("Macro stats dumped to macro_stats.parquet");

    println!("Dashboard running at http://localhost:3000/ws. Waiting for 30s before exit...");
    tokio::time::sleep(std::time::Duration::from_secs(30)).await;
}
