// Copyright (c) Learned Index
// Comparative table rendering and Markdown reporting

use crate::bench::target::TargetAggregateSummary;

pub fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}

/// Renders the comparative matrix table with the bottom "Moyenne totale par cible" row.
pub fn print_comparative_table(title: &str, summaries: &[TargetAggregateSummary]) {
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

    // Bottom Row: Total Latency Average
    print!("{:<30}", "| MOYENNE TOTALE PAR CIBLE");
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
    println!();
}

