use std::path::PathBuf;
use std::time::Instant;

use maggraph::{traverse, GraphAdjacency, GraphIndex, TraversalOrder};

fn example_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../examples/basic/knowledge_graph")
}

fn main() {
    let root = example_root();
    let index = GraphIndex::open(&root).expect("open index");
    let adjacency = GraphAdjacency::from_index(&index).expect("adjacency");

    let iterations = 1_000usize;
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = traverse(&adjacency, &index, "welcome", 2, TraversalOrder::Bfs).expect("traverse");
    }
    let elapsed = start.elapsed();
    let per_op_us = elapsed.as_micros() as f64 / iterations as f64;

    println!("MagGraph traversal benchmark");
    println!("fixture: {}", root.display());
    println!("iterations: {iterations}");
    println!("total: {elapsed:?}");
    println!("per_traversal_us: {per_op_us:.2}");

    // Smoke gate aligned with Phase 3 acceptance (<1ms per traversal on small graph).
    assert!(
        per_op_us < 1_000.0,
        "expected <1000µs per traversal, got {per_op_us:.2}µs"
    );
}
