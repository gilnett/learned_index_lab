// Copyright (c) Learned Index
// Repetitive suite runner and metric aggregation

use crate::bench::dataset::{
    DistributionType, Rng, generate_dataset, generate_internal_miss_queries,
};
use crate::bench::reporting::{mean, print_comparative_table};
use crate::bench::target::{
    BenchmarkTarget, TargetAggregateSummary, TargetRunMetric, run_isolated_target_benchmark,
};

/// Executes isolated tests for each target across `num_runs` iterations, printing details and summary.
pub fn execute_target_suite(
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
            } else if m.memory_kb == 0.0 {
                "0.00 KB".to_string()
            } else {
                format!("{:.1} KB", m.memory_kb)
            };

            let lb_str = format!("{:.1} ns", m.lower_bound_ns);

            println!(
                "#{:02}  | {:>11.2} ms | {:>11.1} ns | {:>11.1} ns | {:>14} | {:>14}",
                run, m.build_ms, m.hit_ns, m.miss_ns, lb_str, mem_str
            );

            runs_data.push(m);
        }

        // Summary row for this target
        let mean_build = mean(
            &runs_data
                .iter()
                .map(|r: &TargetRunMetric| r.build_ms)
                .collect::<Vec<_>>(),
        );
        let mean_hit = mean(&runs_data.iter().map(|r| r.hit_ns).collect::<Vec<_>>());
        let mean_miss = mean(&runs_data.iter().map(|r| r.miss_ns).collect::<Vec<_>>());
        let mean_lb = mean(
            &runs_data
                .iter()
                .map(|r| r.lower_bound_ns)
                .collect::<Vec<_>>(),
        );
        let mean_mem = mean(&runs_data.iter().map(|r| r.memory_kb).collect::<Vec<_>>());
        let segments_mean = runs_data.first().map(|r| r.segments).unwrap_or(0);
        let mean_total_lat = (mean_hit + mean_miss + mean_lb) / 3.0;

        let mem_summary_str = if mean_mem >= 1024.0 {
            format!("{:.2} MB", mean_mem / 1024.0)
        } else if mean_mem == 0.0 {
            "0.00 KB".to_string()
        } else {
            format!("{:.1} KB", mean_mem)
        };

        println!("{}", "-".repeat(95));
        println!(
            "{:>5} | {:>11.2} ms | {:>11.1} ns | {:>11.1} ns | {:>11.1} ns | {:>14}",
            "MOY.", mean_build, mean_hit, mean_miss, mean_lb, mem_summary_str
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
