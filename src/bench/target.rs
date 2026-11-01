// Copyright (c) Learned Index
// Target selection, metrics tracking, and single-target isolated execution

use learned_index_lab::{EytzingerSearch, MultilevelPgmIndex, PgmIndex};
use std::collections::BTreeMap;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BenchmarkTarget {
    BTree,
    Slice,
    Eytzinger,
    Pgm,
    Multilevel,
}

impl BenchmarkTarget {
    pub const ALL: [BenchmarkTarget; 5] = [
        BenchmarkTarget::BTree,
        BenchmarkTarget::Slice,
        BenchmarkTarget::Eytzinger,
        BenchmarkTarget::Pgm,
        BenchmarkTarget::Multilevel,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            Self::BTree => "BTreeMap",
            Self::Slice => "Flat Slice",
            Self::Eytzinger => "Eytzinger",
            Self::Pgm => "PGM-Index",
            Self::Multilevel => "Multilevel PGM",
        }
    }

    pub fn from_str_opt(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "btree" | "btreemap" | "b-tree" => Some(Self::BTree),
            "slice" | "flat" | "flatslice" | "binary_search" => Some(Self::Slice),
            "eyt" | "eytzinger" => Some(Self::Eytzinger),
            "pgm" | "pgmindex" | "pgm-index" | "learned" => Some(Self::Pgm),
            "multilevel" | "multi" | "ml-pgm" | "multilevel-pgm" => Some(Self::Multilevel),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct TargetRunMetric {
    pub build_ms: f64,
    pub hit_ns: f64,
    pub miss_ns: f64,
    pub lower_bound_ns: f64,
    pub memory_kb: f64,
    pub segments: usize,
}

#[derive(Debug, Clone, Default)]
pub struct TargetAggregateSummary {
    pub target: Option<BenchmarkTarget>,
    pub label: String,
    pub mean_build_ms: f64,
    pub mean_hit_ns: f64,
    pub mean_miss_ns: f64,
    pub mean_lb_ns: f64,
    pub mean_memory_kb: f64,
    pub mean_segments: usize,
    pub mean_total_latency_ns: f64,
}

/// Purges CPU caches (L1, L2, L3) via a 32MB buffer eviction barrier.
pub fn clear_cpu_cache() {
    let mut dummy = vec![0u8; 32 * 1024 * 1024];
    for byte in dummy.iter_mut().step_by(64) {
        *byte = byte.wrapping_add(1);
    }
    std::hint::black_box(&dummy);
}

/// Runs a single isolated benchmark for one specific target structure.
pub fn run_isolated_target_benchmark(
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
            let mut lb_sum = 0usize;
            for &k in hit_queries {
                if let Some((_, &idx)) = btree.range(std::hint::black_box(k)..).next() {
                    lb_sum += idx;
                }
            }
            std::hint::black_box(lb_sum);
            let lower_bound_ns = t0.elapsed().as_nanos() as f64 / hit_queries.len() as f64;

            assert_eq!(hits, hit_queries.len());
            assert_eq!(misses, 0);

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
            clear_cpu_cache();
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
        BenchmarkTarget::Multilevel => {
            clear_cpu_cache();
            let t0 = Instant::now();
            let ml_pgm = MultilevelPgmIndex::build(dataset.to_vec(), epsilon);
            std::hint::black_box(&ml_pgm);
            let build_ms = t0.elapsed().as_secs_f64() * 1000.0;

            clear_cpu_cache();
            let t0 = Instant::now();
            let mut hits = 0;
            for &k in hit_queries {
                if ml_pgm.find(std::hint::black_box(k)).is_some() {
                    hits += 1;
                }
            }
            std::hint::black_box(hits);
            let hit_ns = t0.elapsed().as_nanos() as f64 / hit_queries.len() as f64;

            clear_cpu_cache();
            let t0 = Instant::now();
            let mut misses = 0;
            for &k in miss_queries {
                if ml_pgm.find(std::hint::black_box(k)).is_some() {
                    misses += 1;
                }
            }
            std::hint::black_box(misses);
            let miss_ns = t0.elapsed().as_nanos() as f64 / miss_queries.len() as f64;

            clear_cpu_cache();
            let t0 = Instant::now();
            let mut lb_sum = 0u64;
            for &k in hit_queries {
                let idx = ml_pgm.lower_bound(std::hint::black_box(k));
                if idx < dataset.len() {
                    lb_sum += dataset[idx];
                }
            }
            std::hint::black_box(lb_sum);
            let lower_bound_ns = t0.elapsed().as_nanos() as f64 / hit_queries.len() as f64;

            assert_eq!(hits, hit_queries.len());
            assert_eq!(misses, 0);

            let memory_kb = ml_pgm.index_size_in_bytes() as f64 / 1024.0;
            let segments = ml_pgm.total_segments();

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

