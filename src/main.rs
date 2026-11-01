// Copyright (c) Learned Index
// Multi-Target Isolated Benchmark Harness with Target Selection and Statistical Aggregation

mod bench;

use bench::batch::run_batch_ingestion_benchmark;
use bench::cli::parse_cli_args;
use bench::reporting::{mean, print_comparative_table};
use bench::suite::execute_target_suite;
use bench::target::TargetAggregateSummary;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let config = match parse_cli_args(&args) {
        Some(c) => c,
        None => return,
    };

    if config.run_batch {
        run_batch_ingestion_benchmark(config.n_items, config.batch_size, 16);
        return;
    }

    println!("{}", "=".repeat(110));
    println!("   LEARNED INDEX BENCHMARK HARNESS : STATISTICAL MULTI-TARGET SUITE");
    println!(
        "   Cibles actives : {:?} | Runs par cible : {} | Éléments : {} | Requêtes : {}",
        config
            .selected_targets
            .iter()
            .map(|t| t.name())
            .collect::<Vec<_>>(),
        config.num_runs,
        config.n_items,
        config.n_queries
    );
    println!("{}", "=".repeat(110));

    let mut all_suite_summaries = Vec::new();

    for (dist, epsilon) in config.suites {
        let summaries = execute_target_suite(
            dist,
            &config.selected_targets,
            epsilon,
            config.n_items,
            config.n_queries,
            config.num_runs,
        );
        all_suite_summaries.push((dist, summaries));
    }

    if all_suite_summaries.len() > 1 {
        println!("\n{}", "=".repeat(110));
        println!(
            "   SYNTHÈSE GLOBALE TOUS SCÉNARIOS CONFONDUS (MOYENNE TOTALE PAR CIBLE SUR {} TESTS)",
            config.num_runs * all_suite_summaries.len()
        );
        println!("{}", "=".repeat(110));

        let mut consolidated_summaries = Vec::new();

        for &target in &config.selected_targets {
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
                config.num_runs * all_suite_summaries.len()
            ),
            &consolidated_summaries,
        );
    }
}
