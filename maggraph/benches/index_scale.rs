use std::env;
use std::fs;
use std::time::{Duration, Instant};

use std::collections::HashMap;

use maggraph::{GraphIndex, HybridQueryOptions, QueryOptions};
use tempfile::TempDir;

fn sizes() -> Vec<usize> {
    if let Ok(value) = env::var("MAGGRAPH_BENCH_SIZES") {
        return value
            .split(',')
            .filter_map(|item| item.trim().parse().ok())
            .collect();
    }
    if env::var_os("MAGGRAPH_BENCH_FULL").is_some() {
        vec![1_000, 10_000, 100_000]
    } else {
        vec![1_000, 10_000]
    }
}

fn write_fixture(root: &std::path::Path, count: usize) {
    fs::create_dir_all(root).expect("create fixture root");
    for index in 0..count {
        let previous = index.checked_sub(1).unwrap_or(count - 1);
        let tags = if index % 25 == 0 {
            "tags: [\"benchmark\", \"selected\"]\n"
        } else {
            "tags: [\"benchmark\"]\n"
        };
        let markdown = format!(
            "---\nid: \"node_{index:06}\"\ntype: \"project_fact\"\n{tags}links: [\"node_{previous:06}\"]\n---\nSynthetic graph fact {index}. Search needle group {}.\n",
            index % 100
        );
        fs::write(root.join(format!("node_{index:06}.md")), markdown).expect("write node");
    }
}

fn timed<T>(operation: impl FnOnce() -> T) -> (T, Duration) {
    let started = Instant::now();
    let result = operation();
    (result, started.elapsed())
}

fn main() {
    println!("MagGraph index scale benchmark");
    println!("nodes,open_ms,search_ms,hybrid_ms,backlinks_ms,recall_us,update_us");

    for count in sizes() {
        let temp = TempDir::new().expect("temp dir");
        let root = temp.path().join("graph");
        write_fixture(&root, count);

        let (mut index, open) = timed(|| GraphIndex::open(&root).expect("open index"));
        let options = QueryOptions {
            text: Some("needle group 42".into()),
            tags: vec!["benchmark".into()],
            limit: 50,
            ..QueryOptions::default()
        };
        let (results, search) = timed(|| index.search(&options).expect("search"));
        assert!(!results.is_empty());
        let hybrid_options = HybridQueryOptions {
            text: Some("needle group 42".into()),
            tags: vec!["benchmark".into()],
            seed_ids: vec!["node_000042".into()],
            semantic_scores: HashMap::from([("node_000142".into(), 0.9)]),
            limit: 50,
            ..HybridQueryOptions::default()
        };
        let (hybrid_results, hybrid) =
            timed(|| index.hybrid_search(&hybrid_options).expect("hybrid search"));
        assert!(!hybrid_results.is_empty());

        let (_, backlinks) = timed(|| index.backlinks("node_000042").expect("backlinks"));
        let (_, recall) = timed(|| {
            index
                .recall_bundle("node_000042", "scale benchmark", 600)
                .expect("recall bundle")
        });
        let (_, update) = timed(|| {
            index
                .update_file(root.join("node_000042.md"))
                .expect("incremental update")
        });

        println!(
            "{count},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3}",
            open.as_secs_f64() * 1_000.0,
            search.as_secs_f64() * 1_000.0,
            hybrid.as_secs_f64() * 1_000.0,
            backlinks.as_secs_f64() * 1_000.0,
            recall.as_secs_f64() * 1_000_000.0,
            update.as_secs_f64() * 1_000_000.0,
        );
    }
}
