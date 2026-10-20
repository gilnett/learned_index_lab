// Copyright (c) Learned Index
// Multi-Target Isolated Benchmark Harness with Target Selection and Statistical Aggregation
// Targets: BTreeMap, Flat Slice, Eytzinger Layout, PGM-Index

use learned_index::{EytzingerSearch, PgmIndex};
use std::collections::BTreeMap;
use std::time::Instant;

/// Minimal PRNG (Xorshift64) for reproducible dataset and query generation.
struct Rng {
    state: u64,
}

impl Rng {
    fn new(seed: u64) -> Self {
        Self { state: seed.max(1) }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    fn next_f64(&mut self) -> f64 {
        (self.next_u64() as f64) / (u64::MAX as f64)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DistributionType {
    DenseTimestamp,
    UniformSparse,
    ClusteredLognormal,
}

impl DistributionType {
    fn name(&self) -> &'static str {
        match self {
            Self::DenseTimestamp => "Dense Sequential / Time-Series",
            Self::UniformSparse => "Uniform Sparse Random Distribution",
            Self::ClusteredLognormal => "Heavy-Tailed Lognormal Distribution",
        }
    }

    fn default_epsilon(&self) -> usize {
        match self {
            Self::DenseTimestamp => 32,
            Self::UniformSparse => 16,
            Self::ClusteredLognormal => 16,
        }
    }
}

fn generate_dataset(dist: DistributionType, count: usize, seed: u64) -> Vec<u64> {
    let mut rng = Rng::new(seed);
    let mut data = Vec::with_capacity(count);

    match dist {
        DistributionType::DenseTimestamp => {
            let mut current = 1_700_000_000u64;
            for _ in 0..count {
                current += 1 + (rng.next_u64() % 5);
                data.push(current);
            }
        }
        DistributionType::UniformSparse => {
            let mut set = std::collections::BTreeSet::new();
            while set.len() < count {
                let v = rng.next_u64() % 1_000_000_000_000;
                set.insert(v);
            }
            data.extend(set);
        }
        DistributionType::ClusteredLognormal => {
            let mut raw = Vec::with_capacity(count);
            for _ in 0..count {
                let u1 = rng.next_f64().max(1e-9);
                let u2 = rng.next_f64().max(1e-9);
                let z0 = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
                let lognormal = (z0 * 1.5 + 10.0).exp();
                raw.push(lognormal as u64);
            }
            raw.sort_unstable();
            raw.dedup();
            let mut current = raw.last().copied().unwrap_or(0);
            while raw.len() < count {
                current += 1 + (rng.next_u64() % 10);
                raw.push(current);
            }
            data = raw;
        }
    }

    data
}

fn generate_internal_miss_queries(dataset: &[u64], count: usize, rng: &mut Rng) -> Vec<u64> {
    let mut queries = Vec::with_capacity(count);
    if dataset.len() < 2 {
        return queries;
    }

    let min_k = dataset[0];
    let max_k = *dataset.last().unwrap();
    let span = max_k.saturating_sub(min_k);

    if span == 0 {
        return queries;
    }

    // Sample within internal gaps between adjacent keys
    let mut attempts = 0;
    while queries.len() < count && attempts < count * 20 {
        attempts += 1;
        let idx = (rng.next_u64() as usize) % (dataset.len() - 1);
        let left = dataset[idx];
        let right = dataset[idx + 1];
        if right > left + 1 {
            let gap = right - left - 1;
            let offset = 1 + (rng.next_u64() % gap);
            queries.push(left + offset);
        }
    }

    // Fallback: uniform sampling strictly within [min_k, max_k] absent from dataset
    while queries.len() < count {
        let candidate = min_k + (rng.next_u64() % (span + 1));
        if dataset.binary_search(&candidate).is_err() {
            queries.push(candidate);
        }
    }

    queries
}

/// Purges CPU caches (L1, L2, L3) via a 32MB buffer eviction barrier.
fn clear_cpu_cache() {
    let mut dummy = vec![0u8; 32 * 1024 * 1024];
    for byte in dummy.iter_mut().step_by(64) {
        *byte = byte.wrapping_add(1);
    }
    std::hint::black_box(&dummy);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum BenchmarkTarget {
    BTree,
    Slice,
    Eytzinger,
    Pgm,
}

impl BenchmarkTarget {
    const ALL: [BenchmarkTarget; 4] = [
        BenchmarkTarget::BTree,
        BenchmarkTarget::Slice,
        BenchmarkTarget::Eytzinger,
        BenchmarkTarget::Pgm,
    ];

    fn name(&self) -> &'static str {
        match self {
            Self::BTree => "BTreeMap",
            Self::Slice => "Flat Slice",
            Self::Eytzinger => "Eytzinger",
            Self::Pgm => "PGM-Index",
        }
    }

    fn from_str_opt(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "btree" | "btreemap" | "b-tree" => Some(Self::BTree),
            "slice" | "flat" | "flatslice" | "binary_search" => Some(Self::Slice),
            "eyt" | "eytzinger" => Some(Self::Eytzinger),
            "pgm" | "pgmindex" | "pgm-index" | "learned" => Some(Self::Pgm),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct TargetRunMetric {
    build_ms: f64,
    hit_ns: f64,
    miss_ns: f64,
    lower_bound_ns: f64,
    memory_kb: f64,
    segments: usize,
}

#[derive(Debug, Clone, Default)]
struct TargetAggregateSummary {
    target: Option<BenchmarkTarget>,
    label: String,
    mean_build_ms: f64,
    mean_hit_ns: f64,
    mean_miss_ns: f64,
    mean_lb_ns: f64,
    mean_memory_kb: f64,
    mean_segments: usize,
    mean_total_latency_ns: f64,
}

fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}

/// Runs a single isolated benchmark for one specific target structure.
fn run_isolated_target_benchmark(
    target: BenchmarkTarget,
    dataset: &[u64],
    hit_queries: &[u64],
    miss_queries: &[u64],
    epsilon: usize,
) -> TargetRunMetric {
    match target {
        BenchmarkTarget::BTree => {
            clear_cpu_cache();
            let t0 = Instant::now();
            let mut btree = BTreeMap::new();
            for (idx, &k) in dataset.iter().enumerate() {
                btree.insert(k, idx);
            }
            std::hint::black_box(&btree);
            let build_ms = t0.elapsed().as_secs_f64() * 1000.0;

            clear_cpu_cache();
            let t0 = Instant::now();
            let mut hits = 0;
            for &k in hit_queries {
                if btree.contains_key(&std::hint::black_box(k)) {
                    hits += 1;
                }
            }
            std::hint::black_box(hits);
            let hit_ns = t0.elapsed().as_nanos() as f64 / hit_queries.len() as f64;

            clear_cpu_cache();
            let t0 = Instant::now();
            let mut misses = 0;
            for &k in miss_queries {
                if btree.contains_key(&std::hint::black_box(k)) {
                    misses += 1;
                }
            }
            std::hint::black_box(misses);
            let miss_ns = t0.elapsed().as_nanos() as f64 / miss_queries.len() as f64;

            clear_cpu_cache();
            let t0 = Instant::now();
            let mut lb_sum = 0u64;
            for &k in hit_queries {
                if let Some((&val, _)) = btree.range(std::hint::black_box(k)..).next() {
                    lb_sum += val;
                }
            }
            std::hint::black_box(lb_sum);
            let lower_bound_ns = t0.elapsed().as_nanos() as f64 / hit_queries.len() as f64;

            assert_eq!(hits, hit_queries.len());
            assert_eq!(misses, 0);

            // BTreeMap nodes overhead: ~48 bytes per entry on 64-bit platforms
            let memory_kb = (dataset.len() * 48) as f64 / 1024.0;

            TargetRunMetric {
                build_ms,
                hit_ns,
                miss_ns,
                lower_bound_ns,
                memory_kb,
                segments: 0,
            }
        }
        BenchmarkTarget::Slice => {
            // Flat Slice: Pre-sorted slice, zero build time
            let build_ms = 0.0;

            clear_cpu_cache();
            let t0 = Instant::now();
            let mut hits = 0;
            for &k in hit_queries {
                if dataset.binary_search(&std::hint::black_box(k)).is_ok() {
                    hits += 1;
                }
            }
            std::hint::black_box(hits);
            let hit_ns = t0.elapsed().as_nanos() as f64 / hit_queries.len() as f64;

            clear_cpu_cache();
            let t0 = Instant::now();
            let mut misses = 0;
            for &k in miss_queries {
                if dataset.binary_search(&std::hint::black_box(k)).is_ok() {
                    misses += 1;
                }
            }
            std::hint::black_box(misses);
            let miss_ns = t0.elapsed().as_nanos() as f64 / miss_queries.len() as f64;

            clear_cpu_cache();
            let t0 = Instant::now();
            let mut lb_sum = 0u64;
            for &k in hit_queries {
                let idx = dataset.partition_point(|&x| x < std::hint::black_box(k));
                if idx < dataset.len() {
                    lb_sum += dataset[idx];
                }
            }
            std::hint::black_box(lb_sum);
            let lower_bound_ns = t0.elapsed().as_nanos() as f64 / hit_queries.len() as f64;

            assert_eq!(hits, hit_queries.len());
            assert_eq!(misses, 0);

            TargetRunMetric {
                build_ms,
                hit_ns,
                miss_ns,
                lower_bound_ns,
                memory_kb: 0.0,
                segments: 0,
            }
        }
        BenchmarkTarget::Eytzinger => {
            clear_cpu_cache();
            let t0 = Instant::now();
            let eyt = EytzingerSearch::build(dataset);
            std::hint::black_box(&eyt);
            let build_ms = t0.elapsed().as_secs_f64() * 1000.0;

            clear_cpu_cache();
            let t0 = Instant::now();
            let mut hits = 0;
            for &k in hit_queries {
                if eyt.contains(std::hint::black_box(k)) {
                    hits += 1;
                }
            }
            std::hint::black_box(hits);
            let hit_ns = t0.elapsed().as_nanos() as f64 / hit_queries.len() as f64;

            clear_cpu_cache();
            let t0 = Instant::now();
            let mut misses = 0;
            for &k in miss_queries {
                if eyt.contains(std::hint::black_box(k)) {
                    misses += 1;
                }
            }
            std::hint::black_box(misses);
            let miss_ns = t0.elapsed().as_nanos() as f64 / miss_queries.len() as f64;

            clear_cpu_cache();
            let t0 = Instant::now();
            let mut lb_sum = 0u64;
            for &k in hit_queries {
                if let Some(val) = eyt.lower_bound(std::hint::black_box(k)) {
                    lb_sum += val;
                }
            }
            std::hint::black_box(lb_sum);
            let lower_bound_ns = t0.elapsed().as_nanos() as f64 / hit_queries.len() as f64;

            assert_eq!(hits, hit_queries.len());
            assert_eq!(misses, 0);

            let memory_kb = eyt.memory_bytes() as f64 / 1024.0;

            TargetRunMetric {
                build_ms,
                hit_ns,
                miss_ns,
                lower_bound_ns,
                memory_kb,
                segments: 0,
            }
        }
        BenchmarkTarget::Pgm => {
            clear_cpu_cache();
            let t0 = Instant::now();
            let pgm = PgmIndex::build(dataset.to_vec(), epsilon);
            std::hint::black_box(&pgm);
            let build_ms = t0.elapsed().as_secs_f64() * 1000.0;

            clear_cpu_cache();
            let t0 = Instant::now();
            let mut hits = 0;
            for &k in hit_queries {
                if pgm.find(std::hint::black_box(k)).is_some() {
                    hits += 1;
                }
            }
            std::hint::black_box(hits);
            let hit_ns = t0.elapsed().as_nanos() as f64 / hit_queries.len() as f64;

            clear_cpu_cache();
            let t0 = Instant::now();
            let mut misses = 0;
            for &k in miss_queries {
                if pgm.find(std::hint::black_box(k)).is_some() {
                    misses += 1;
                }
            }
            std::hint::black_box(misses);
            let miss_ns = t0.elapsed().as_nanos() as f64 / miss_queries.len() as f64;

            clear_cpu_cache();
            let t0 = Instant::now();
            let mut lb_sum = 0u64;
            for &k in hit_queries {
                let idx = pgm.lower_bound(std::hint::black_box(k));
                if idx < dataset.len() {
                    lb_sum += dataset[idx];
                }
            }
            std::hint::black_box(lb_sum);
            let lower_bound_ns = t0.elapsed().as_nanos() as f64 / hit_queries.len() as f64;

            assert_eq!(hits, hit_queries.len());
            assert_eq!(misses, 0);

            let memory_kb = pgm.index_only_bytes() as f64 / 1024.0;
            let segments = pgm.segments.len();

            TargetRunMetric {
                build_ms,
                hit_ns,
                miss_ns,
                lower_bound_ns,
                memory_kb,
                segments,
            }
        }
    }
}

/// Executes isolated tests for each target across `num_runs` iterations, printing details and summary.
fn execute_target_suite(
    dist: DistributionType,
    targets: &[BenchmarkTarget],
    epsilon: usize,
    n_items: usize,
    n_queries: usize,
    num_runs: usize,
) -> Vec<TargetAggregateSummary> {
    println!("\n{}", "=".repeat(110));
    println!(
        ">>> SUITE: {} [epsilon = {}, {} tests distincts par cible]",
        dist.name(),
        epsilon,
        num_runs
    );
    println!("{}", "=".repeat(110));

    let mut summaries = Vec::new();

    for &target in targets {
        println!(
            "\n--- CIBLE : {:<15} ({} RUNS ISOLÉS) ---",
            target.name(),
            num_runs
        );
        println!(
            "{:>5} | {:>14} | {:>14} | {:>14} | {:>14} | {:>14}",
            "Run", "Build (ms)", "Hit (ns/op)", "Miss (ns/op)", "Lower Bound", "Mémoire"
        );
        println!("{}", "-".repeat(95));

        let mut runs_data = Vec::with_capacity(num_runs);

        for run in 1..=num_runs {
            let seed = (run as u64) * 101 + 7;
            let dataset = generate_dataset(dist, n_items, seed);

            let mut rng = Rng::new(seed + 10_000);
            let mut hit_queries = Vec::with_capacity(n_queries / 2);
            for _ in 0..(n_queries / 2) {
                let idx = (rng.next_u64() as usize) % dataset.len();
                hit_queries.push(dataset[idx]);
            }
            let miss_queries = generate_internal_miss_queries(&dataset, n_queries / 2, &mut rng);

            let m = run_isolated_target_benchmark(
                target,
                &dataset,
                &hit_queries,
                &miss_queries,
                epsilon,
            );

            let mem_str = if m.memory_kb >= 1024.0 {
                format!("{:.2} MB", m.memory_kb / 1024.0)
            } else {
                format!("{:.1} KB", m.memory_kb)
            };

            println!(
                "#{:02}  | {:>11.2} ms | {:>11.1} ns | {:>11.1} ns | {:>11.1} ns | {:>14}",
                run, m.build_ms, m.hit_ns, m.miss_ns, m.lower_bound_ns, mem_str
            );

            runs_data.push(m);
        }

        let builds: Vec<f64> = runs_data.iter().map(|m| m.build_ms).collect();
        let hits: Vec<f64> = runs_data.iter().map(|m| m.hit_ns).collect();
        let misses: Vec<f64> = runs_data.iter().map(|m| m.miss_ns).collect();
        let lbs: Vec<f64> = runs_data.iter().map(|m| m.lower_bound_ns).collect();
        let mems: Vec<f64> = runs_data.iter().map(|m| m.memory_kb).collect();
        let segments_mean = if runs_data.iter().any(|m| m.segments > 0) {
            runs_data.iter().map(|m| m.segments).sum::<usize>() / runs_data.len()
        } else {
            0
        };

        let mean_build = mean(&builds);
        let mean_hit = mean(&hits);
        let mean_miss = mean(&misses);
        let mean_lb = mean(&lbs);
        let mean_mem = mean(&mems);
        let mean_total_lat = (mean_hit + mean_miss + mean_lb) / 3.0;

        println!("{}", "-".repeat(95));
        let mem_str = if mean_mem >= 1024.0 {
            format!("{:.2} MB", mean_mem / 1024.0)
        } else {
            format!("{:.1} KB", mean_mem)
        };
        println!(
            "MOY. | {:>11.2} ms | {:>11.1} ns | {:>11.1} ns | {:>11.1} ns | {:>14}",
            mean_build, mean_hit, mean_miss, mean_lb, mem_str
        );

        summaries.push(TargetAggregateSummary {
            target: Some(target),
            label: target.name().to_string(),
            mean_build_ms: mean_build,
            mean_hit_ns: mean_hit,
            mean_miss_ns: mean_miss,
            mean_lb_ns: mean_lb,
            mean_memory_kb: mean_mem,
            mean_segments: segments_mean,
            mean_total_latency_ns: mean_total_lat,
        });
    }

    // Print Scenario Comparative Table
    print_comparative_table(&format!("TABLEAU COMPARATIF : {}", dist.name()), &summaries);

    summaries
}

/// Renders the comparative matrix table with the bottom "Moyenne totale par cible" row.
fn print_comparative_table(title: &str, summaries: &[TargetAggregateSummary]) {
    if summaries.is_empty() {
        return;
    }

    println!("\n{}", "=".repeat(110));
    println!("   {}", title);
    println!("{}", "=".repeat(110));

    // 1. Column-oriented display (like README comparative table)
    let col_w = 18;
    print!("{:<30}", "| Métrique / Cible");
    for s in summaries {
        print!("| {:^width$} ", s.label, width = col_w);
    }
    println!("|");

    print!("{:<30}", "|:----------------------------");
    for _ in summaries {
        print!("|:{:-<width$}:", "", width = col_w);
    }
    println!("|");

    // Build Time
    print!("{:<30}", "| Build Time (ms)");
    for s in summaries {
        print!("| {:>width$.2} ms ", s.mean_build_ms, width = col_w - 3);
    }
    println!("|");

    // Hit Latency
    print!("{:<30}", "| Hit Latency (ns/op)");
    for s in summaries {
        print!("| {:>width$.1} ns ", s.mean_hit_ns, width = col_w - 3);
    }
    println!("|");

    // Miss Latency
    print!("{:<30}", "| Miss Latency (ns/op)");
    for s in summaries {
        print!("| {:>width$.1} ns ", s.mean_miss_ns, width = col_w - 3);
    }
    println!("|");

    // Lower Bound
    print!("{:<30}", "| Lower Bound (ns/op)");
    for s in summaries {
        print!("| {:>width$.1} ns ", s.mean_lb_ns, width = col_w - 3);
    }
    println!("|");

    // Memory
    print!("{:<30}", "| Index Memory Overhead");
    for s in summaries {
        let mem_str = if s.mean_memory_kb >= 1024.0 {
            format!("{:.2} MB", s.mean_memory_kb / 1024.0)
        } else if s.mean_memory_kb == 0.0 {
            "0.00 KB".to_string()
        } else {
            format!("{:.1} KB", s.mean_memory_kb)
        };
        print!("| {:>width$} ", mem_str, width = col_w);
    }
    println!("|");

    // Segments (if any)
    if summaries.iter().any(|s| s.mean_segments > 0) {
        print!("{:<30}", "| Nombre de Segments");
        for s in summaries {
            let seg_str = if s.mean_segments > 0 {
                format!("{} segs", s.mean_segments)
            } else {
                "N/A".to_string()
            };
            print!("| {:>width$} ", seg_str, width = col_w);
        }
        println!("|");
    }

    // Speedup vs first baseline (if multiple targets)
    if summaries.len() > 1 {
        let baseline_hit = summaries[0].mean_hit_ns;
        print!("{:<30}", format!("| Speedup vs {}", summaries[0].label));
        for s in summaries {
            let sp = if s.mean_hit_ns > 0.0 {
                baseline_hit / s.mean_hit_ns
            } else {
                1.0
            };
            print!("| {:>width$.2}x ", sp, width = col_w - 1);
        }
        println!("|");
    }

    // Divider
    print!("{:<30}", "|=============================");
    for _ in summaries {
        print!("|={:=<width$}=", "", width = col_w);
    }
    println!("|");

    // Case "Moyenne totale par cible" (Latence combinée)
    print!("{:<30}", "| MOYENNE TOTALE PAR CIBLE   ");
    for s in summaries {
        print!(
            "| {:>width$.1} ns ",
            s.mean_total_latency_ns,
            width = col_w - 3
        );
    }
    println!("|");
    println!("{}\n", "-".repeat(110));

    // 2. Row-oriented Markdown table for report export
    println!("Format Markdown récapitulatif :");
    println!(
        "| Cible / Target | Build Time | Hit Latency | Miss Latency | Lower Bound | Index Memory | Moyenne Totale Latence |"
    );
    println!("| :--- | :--- | :--- | :--- | :--- | :--- | :--- |");
    for s in summaries {
        let mem_str = if s.mean_memory_kb >= 1024.0 {
            format!("{:.2} MB", s.mean_memory_kb / 1024.0)
        } else if s.mean_memory_kb == 0.0 {
            "0.00 KB".to_string()
        } else {
            format!("{:.1} KB", s.mean_memory_kb)
        };
        println!(
            "| **{:<12}** | {:>8.2} ms | {:>9.1} ns | {:>10.1} ns | {:>9.1} ns | {:>12} | **{:>9.1} ns** |",
            s.label,
            s.mean_build_ms,
            s.mean_hit_ns,
            s.mean_miss_ns,
            s.mean_lb_ns,
            mem_str,
            s.mean_total_latency_ns
        );
    }
    // Bottom row: average across targets
    let avg_build = mean(
        &summaries
            .iter()
            .map(|s| s.mean_build_ms)
            .collect::<Vec<_>>(),
    );
    let avg_hit = mean(&summaries.iter().map(|s| s.mean_hit_ns).collect::<Vec<_>>());
    let avg_miss = mean(&summaries.iter().map(|s| s.mean_miss_ns).collect::<Vec<_>>());
    let avg_lb = mean(&summaries.iter().map(|s| s.mean_lb_ns).collect::<Vec<_>>());
    let avg_mem = mean(
        &summaries
            .iter()
            .map(|s| s.mean_memory_kb)
            .collect::<Vec<_>>(),
    );
    let avg_tot = mean(
        &summaries
            .iter()
            .map(|s| s.mean_total_latency_ns)
            .collect::<Vec<_>>(),
    );
    let avg_mem_str = if avg_mem >= 1024.0 {
        format!("{:.2} MB", avg_mem / 1024.0)
    } else {
        format!("{:.1} KB", avg_mem)
    };
    println!(
        "| **MOYENNE GLOBALE** | {:>8.2} ms | {:>9.1} ns | {:>10.1} ns | {:>9.1} ns | {:>12} | **{:>9.1} ns** |",
        avg_build, avg_hit, avg_miss, avg_lb, avg_mem_str, avg_tot
    );
    println!();
}

fn print_help() {
    println!("Learned Index Benchmark Runner");
    println!("USAGE:");
    println!("    cargo run --release -- [OPTIONS] [TARGETS...]\n");
    println!("OPTIONS:");
    println!("    -t, --target <T>        Target(s) to benchmark (comma-separated or multiple):");
    println!("                            all, btree, slice, eytzinger, pgm [default: all]");
    println!("    -r, --runs <N>          Number of distinct test runs per target [default: 10]");
    println!("    -k, --keys <N>          Number of dataset keys [default: 1000000]");
    println!("    -q, --queries <N>       Number of search queries per run [default: 100000]");
    println!(
        "    -s, --suite <S>         Distribution suite: all, dense, uniform, lognormal [default: all]"
    );
    println!("    -h, --help              Print this help menu\n");
    println!("EXAMPLES:");
    println!("    cargo run --release");
    println!("    cargo run --release -- --target eytzinger,pgm");
    println!("    cargo run --release -- pgm --runs 10");
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let mut selected_targets = Vec::new();
    let mut num_runs = 10usize;
    let mut n_items = 1_000_000usize;
    let mut n_queries = 100_000usize;
    let mut suite_filter = "all".to_string();

    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        match arg.as_str() {
            "-h" | "--help" => {
                print_help();
                return;
            }
            "-t" | "--target" | "--targets" => {
                if i + 1 < args.len() {
                    i += 1;
                    for part in args[i].split(',') {
                        if part.trim().eq_ignore_ascii_case("all") {
                            selected_targets = BenchmarkTarget::ALL.to_vec();
                        } else if let Some(t) = BenchmarkTarget::from_str_opt(part)
                            .filter(|t| !selected_targets.contains(t))
                        {
                            selected_targets.push(t);
                        }
                    }
                }
            }
            "-r" | "--runs" => {
                if i + 1 < args.len() {
                    i += 1;
                    if let Ok(v) = args[i].parse::<usize>() {
                        num_runs = v.max(1);
                    }
                }
            }
            "-k" | "--keys" | "--items" => {
                if i + 1 < args.len() {
                    i += 1;
                    if let Ok(v) = args[i].parse::<usize>() {
                        n_items = v.max(100);
                    }
                }
            }
            "-q" | "--queries" => {
                if i + 1 < args.len() {
                    i += 1;
                    if let Ok(v) = args[i].parse::<usize>() {
                        n_queries = v.max(10);
                    }
                }
            }
            "-s" | "--suite" => {
                if i + 1 < args.len() {
                    i += 1;
                    suite_filter = args[i].to_ascii_lowercase();
                }
            }
            other => {
                if other.eq_ignore_ascii_case("all") {
                    selected_targets = BenchmarkTarget::ALL.to_vec();
                } else if let Some(t) =
                    BenchmarkTarget::from_str_opt(other).filter(|t| !selected_targets.contains(t))
                {
                    selected_targets.push(t);
                }
            }
        }
        i += 1;
    }

    if selected_targets.is_empty() {
        selected_targets = BenchmarkTarget::ALL.to_vec();
    }

    println!("{}", "=".repeat(110));
    println!("   LEARNED INDEX BENCHMARK HARNESS : STATISTICAL MULTI-TARGET SUITE");
    println!(
        "   Cibles actives : {:?} | Runs par cible : {} | Éléments : {} | Requêtes : {}",
        selected_targets
            .iter()
            .map(|t| t.name())
            .collect::<Vec<_>>(),
        num_runs,
        n_items,
        n_queries
    );
    println!("{}", "=".repeat(110));

    let mut suites = Vec::new();
    match suite_filter.as_str() {
        "dense" | "timestamp" => {
            suites.push((
                DistributionType::DenseTimestamp,
                DistributionType::DenseTimestamp.default_epsilon(),
            ));
        }
        "uniform" | "sparse" => {
            suites.push((
                DistributionType::UniformSparse,
                DistributionType::UniformSparse.default_epsilon(),
            ));
        }
        "lognormal" | "clustered" => {
            suites.push((
                DistributionType::ClusteredLognormal,
                DistributionType::ClusteredLognormal.default_epsilon(),
            ));
        }
        _ => {
            suites.push((
                DistributionType::DenseTimestamp,
                DistributionType::DenseTimestamp.default_epsilon(),
            ));
            suites.push((
                DistributionType::UniformSparse,
                DistributionType::UniformSparse.default_epsilon(),
            ));
            suites.push((
                DistributionType::ClusteredLognormal,
                DistributionType::ClusteredLognormal.default_epsilon(),
            ));
        }
    }

    let mut all_suite_summaries = Vec::new();

    for (dist, epsilon) in suites {
        let summaries = execute_target_suite(
            dist,
            &selected_targets,
            epsilon,
            n_items,
            n_queries,
            num_runs,
        );
        all_suite_summaries.push((dist, summaries));
    }

    // Consolidated Final Grand Summary across all suites
    if all_suite_summaries.len() > 1 {
        println!("\n{}", "=".repeat(110));
        println!(
            "   SYNTHÈSE GLOBALE TOUS SCÉNARIOS CONFONDUS (MOYENNE TOTALE PAR CIBLE SUR {} TESTS)",
            num_runs * all_suite_summaries.len()
        );
        println!("{}", "=".repeat(110));

        let mut consolidated_summaries = Vec::new();

        for &target in &selected_targets {
            let mut target_builds = Vec::new();
            let mut target_hits = Vec::new();
            let mut target_misses = Vec::new();
            let mut target_lbs = Vec::new();
            let mut target_mems = Vec::new();
            let mut target_segments = Vec::new();

            for (_, suite_sums) in &all_suite_summaries {
                if let Some(s) = suite_sums.iter().find(|s| s.target == Some(target)) {
                    target_builds.push(s.mean_build_ms);
                    target_hits.push(s.mean_hit_ns);
                    target_misses.push(s.mean_miss_ns);
                    target_lbs.push(s.mean_lb_ns);
                    target_mems.push(s.mean_memory_kb);
                    if s.mean_segments > 0 {
                        target_segments.push(s.mean_segments);
                    }
                }
            }

            let mean_build = mean(&target_builds);
            let mean_hit = mean(&target_hits);
            let mean_miss = mean(&target_misses);
            let mean_lb = mean(&target_lbs);
            let mean_mem = mean(&target_mems);
            let mean_seg = if target_segments.is_empty() {
                0
            } else {
                target_segments.iter().sum::<usize>() / target_segments.len()
            };
            let mean_total_lat = (mean_hit + mean_miss + mean_lb) / 3.0;

            consolidated_summaries.push(TargetAggregateSummary {
                target: Some(target),
                label: target.name().to_string(),
                mean_build_ms: mean_build,
                mean_hit_ns: mean_hit,
                mean_miss_ns: mean_miss,
                mean_lb_ns: mean_lb,
                mean_memory_kb: mean_mem,
                mean_segments: mean_seg,
                mean_total_latency_ns: mean_total_lat,
            });
        }

        print_comparative_table(
            &format!(
                "SYNTHÈSE MULTI-SCÉNARIOS (MOYENNE TOTALE PAR CIBLE SUR {} TESTS)",
                num_runs * all_suite_summaries.len()
            ),
            &consolidated_summaries,
        );
    }
}
