// Copyright (c) Learned Index
// Batch ingestion and rebuild benchmark comparing streaming PLA merges to dynamic BTreeMap inserts

use crate::bench::dataset::Rng;
use crate::bench::target::clear_cpu_cache;
use learned_index_lab::{MultilevelPgmIndex, PgmIndex};
use std::collections::BTreeMap;
use std::time::Instant;

pub fn run_batch_ingestion_benchmark(n_base: usize, batch_size: usize, epsilon: usize) {
    println!("\n{}", "=".repeat(110));
    println!("   BATCH INGESTION & REBUILD BENCHMARK (SSTable Merge vs BTreeMap Insert)");
    println!(
        "   Taille de base : {} clés | Batch : {} clés | Epsilon : {}",
        n_base, batch_size, epsilon
    );
    println!("{}", "=".repeat(110));

    let mut rng = Rng::new(42);
    let mut base_data = Vec::with_capacity(n_base);
    let mut cur = 100u64;
    for _ in 0..n_base {
        cur += 2 + (rng.next_u64() % 10);
        base_data.push(cur);
    }

    let mut batch_keys = Vec::with_capacity(batch_size);
    let min_k = base_data[0];
    let max_k = *base_data.last().expect("invariant: non-empty base dataset");
    let span = max_k.saturating_sub(min_k).max(1);
    for _ in 0..batch_size {
        batch_keys.push(min_k + (rng.next_u64() % span));
    }
    batch_keys.sort_unstable();

    // 1. BTreeMap in-place insert
    clear_cpu_cache();
    let mut btree = BTreeMap::new();
    for (i, &k) in base_data.iter().enumerate() {
        btree.insert(k, i);
    }
    clear_cpu_cache();
    let t0 = Instant::now();
    for (i, &k) in batch_keys.iter().enumerate() {
        btree.insert(k, n_base + i);
    }
    let btree_batch_ms = t0.elapsed().as_secs_f64() * 1000.0;
    let btree_throughput = (batch_size as f64) / (btree_batch_ms / 1000.0);

    // 2. Flat PgmIndex merge_sorted_batch (2-way merge + streaming PLA)
    clear_cpu_cache();
    let pgm = PgmIndex::build(base_data.clone(), epsilon);
    clear_cpu_cache();
    let t0 = Instant::now();
    let pgm_merged = pgm.merge_sorted_batch(&batch_keys);
    std::hint::black_box(&pgm_merged);
    let pgm_merge_ms = t0.elapsed().as_secs_f64() * 1000.0;
    let pgm_throughput = (batch_size as f64) / (pgm_merge_ms / 1000.0);

    // 3. MultilevelPgmIndex merge_sorted_batch
    clear_cpu_cache();
    let ml = MultilevelPgmIndex::build(base_data, epsilon);
    clear_cpu_cache();
    let t0 = Instant::now();
    let ml_merged = ml.merge_sorted_batch(&batch_keys);
    std::hint::black_box(&ml_merged);
    let ml_merge_ms = t0.elapsed().as_secs_f64() * 1000.0;
    let ml_throughput = (batch_size as f64) / (ml_merge_ms / 1000.0);

    println!(
        "{:^25} | {:^18} | {:^22} | {:^20}",
        "Cible", "Durée Batch (ms)", "Débit (clés/sec)", "Segments résultants"
    );
    println!("{:-<95}", "");
    println!(
        "{:<25} | {:>15.2} ms | {:>18.0} ops/s | {:>20}",
        "BTreeMap (in-place)", btree_batch_ms, btree_throughput, "N/A"
    );
    println!(
        "{:<25} | {:>15.2} ms | {:>18.0} ops/s | {:>16} segs",
        "PGM (merge + streaming)",
        pgm_merge_ms,
        pgm_throughput,
        pgm_merged.segments.len()
    );
    println!(
        "{:<25} | {:>15.2} ms | {:>18.0} ops/s | {:>16} segs",
        "Multilevel PGM (merge)",
        ml_merge_ms,
        ml_throughput,
        ml_merged.total_segments()
    );
    println!("{}", "=".repeat(110));
}

