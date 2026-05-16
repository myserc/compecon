use std::sync::Arc;
use arrow::array::{UInt64Array, UInt32Array, Int64Array, Float64Array};
use arrow::record_batch::RecordBatch;
use parquet::arrow::arrow_writer::ArrowWriter;
use parquet::file::properties::WriterProperties;
use std::fs::File;
use crate::arithmodynamics::ArithmodynamicNode;
use crate::engine::MacroStats;

pub fn dump_nodes_to_parquet(nodes: &[ArithmodynamicNode], path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let prime_values = UInt64Array::from(nodes.iter().map(|n| n.prime_value).collect::<Vec<_>>());
    let counts = UInt32Array::from(nodes.iter().map(|n| n.counts).collect::<Vec<_>>());
    let vault_books = UInt32Array::from(nodes.iter().map(|n| n.vault_books).collect::<Vec<_>>());
    let entropy_deltas = Int64Array::from(nodes.iter().map(|n| n.entropy_delta).collect::<Vec<_>>());

    let batch = RecordBatch::try_from_iter(vec![
        ("prime_value", Arc::new(prime_values) as Arc<dyn arrow::array::Array>),
        ("counts", Arc::new(counts) as Arc<dyn arrow::array::Array>),
        ("vault_books", Arc::new(vault_books) as Arc<dyn arrow::array::Array>),
        ("entropy_delta", Arc::new(entropy_deltas) as Arc<dyn arrow::array::Array>),
    ])?;

    let file = File::create(path)?;
    let props = WriterProperties::builder().build();
    let mut writer = ArrowWriter::try_new(file, batch.schema(), Some(props))?;

    writer.write(&batch)?;
    writer.close()?;

    Ok(())
}

pub fn dump_stats_to_parquet(stats_history: &[MacroStats], path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let m0 = UInt64Array::from(stats_history.iter().map(|s| s.m0).collect::<Vec<_>>());
    let total_utility = Float64Array::from(stats_history.iter().map(|s| s.total_utility).collect::<Vec<_>>());
    let agent_count = UInt64Array::from(stats_history.iter().map(|s| s.agent_count as u64).collect::<Vec<_>>());
    let gini = Float64Array::from(stats_history.iter().map(|s| s.gini).collect::<Vec<_>>());

    let batch = RecordBatch::try_from_iter(vec![
        ("m0", Arc::new(m0) as Arc<dyn arrow::array::Array>),
        ("total_utility", Arc::new(total_utility) as Arc<dyn arrow::array::Array>),
        ("agent_count", Arc::new(agent_count) as Arc<dyn arrow::array::Array>),
        ("gini", Arc::new(gini) as Arc<dyn arrow::array::Array>),
    ])?;

    let file = File::create(path)?;
    let props = WriterProperties::builder().build();
    let mut writer = ArrowWriter::try_new(file, batch.schema(), Some(props))?;

    writer.write(&batch)?;
    writer.close()?;

    Ok(())
}
