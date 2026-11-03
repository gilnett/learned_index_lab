// Copyright (c) Learned Index
// Command-line arguments parsing and help menu

use crate::bench::dataset::DistributionType;
use crate::bench::target::BenchmarkTarget;

pub struct CliConfig {
    pub selected_targets: Vec<BenchmarkTarget>,
    pub num_runs: usize,
    pub n_items: usize,
    pub n_queries: usize,
    pub suites: Vec<(DistributionType, usize)>,
    pub run_batch: bool,
    pub batch_size: usize,
}

pub fn print_help() {
    println!("Learned Index Benchmark Runner");
    println!("USAGE:");
    println!("    cargo run --release -- [OPTIONS] [TARGETS...]\n");
    println!("OPTIONS:");
    println!("    -t, --target <T>        Target(s) to benchmark (comma-separated or multiple):");
    println!(
        "                            all, btree, slice, eytzinger, eytzinger-index, pgm, multilevel [default: all]"
    );
    println!("    -r, --runs <N>          Number of distinct test runs per target [default: 10]");
    println!("    -k, --keys <N>          Number of dataset keys [default: 1000000]");
    println!("    -q, --queries <N>       Number of search queries per run [default: 100000]");
    println!(
        "    -s, --suite <S>         Distribution suite: all, dense, uniform, lognormal, osm, wiki, books, sosd [default: all]"
    );
    println!(
        "    -b, --batch             Run batch ingestion & rebuild benchmark (SSTable vs BTree)"
    );
    println!(
        "        --batch-size <N>    Batch size for batch ingestion benchmark [default: 50000]"
    );
    println!("    -h, --help              Print this help menu\n");
    println!("EXAMPLES:");
    println!("    cargo run --release");
    println!("    cargo run --release -- --target multilevel,pgm");
    println!("    cargo run --release -- --suite sosd --target pgm,multilevel");
    println!("    cargo run --release -- --batch --batch-size 100000");
}

pub fn parse_cli_args(args: &[String]) -> Option<CliConfig> {
    let mut selected_targets = Vec::new();
    let mut num_runs = 10usize;
    let mut n_items = 1_000_000usize;
    let mut n_queries = 100_000usize;
    let mut suite_filter = "all".to_string();
    let mut run_batch = false;
    let mut batch_size = 50_000usize;

    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        match arg.as_str() {
            "-h" | "--help" => {
                print_help();
                return None;
            }
            "-b" | "--batch" => {
                run_batch = true;
            }
            "--batch-size" => {
                if i + 1 < args.len() {
                    i += 1;
                    if let Ok(v) = args[i].parse::<usize>() {
                        batch_size = v.max(10);
                    }
                }
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
        "osm" | "cellids" => {
            suites.push((
                DistributionType::SosdOsmCellIds,
                DistributionType::SosdOsmCellIds.default_epsilon(),
            ));
        }
        "wiki" | "wikits" | "timestamps" => {
            suites.push((
                DistributionType::SosdWikiTs,
                DistributionType::SosdWikiTs.default_epsilon(),
            ));
        }
        "books" => {
            suites.push((
                DistributionType::SosdBooks,
                DistributionType::SosdBooks.default_epsilon(),
            ));
        }
        "sosd" => {
            suites.push((
                DistributionType::SosdOsmCellIds,
                DistributionType::SosdOsmCellIds.default_epsilon(),
            ));
            suites.push((
                DistributionType::SosdWikiTs,
                DistributionType::SosdWikiTs.default_epsilon(),
            ));
            suites.push((
                DistributionType::SosdBooks,
                DistributionType::SosdBooks.default_epsilon(),
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
            suites.push((
                DistributionType::SosdOsmCellIds,
                DistributionType::SosdOsmCellIds.default_epsilon(),
            ));
            suites.push((
                DistributionType::SosdWikiTs,
                DistributionType::SosdWikiTs.default_epsilon(),
            ));
            suites.push((
                DistributionType::SosdBooks,
                DistributionType::SosdBooks.default_epsilon(),
            ));
        }
    }

    Some(CliConfig {
        selected_targets,
        num_runs,
        n_items,
        n_queries,
        suites,
        run_batch,
        batch_size,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_help() {
        let args = vec!["--help".to_string()];
        assert!(parse_cli_args(&args).is_none());
    }

    #[test]
    fn test_cli_defaults() {
        let args = vec![];
        let cfg = parse_cli_args(&args).expect("Default config should parse");
        assert_eq!(cfg.num_runs, 10);
        assert_eq!(cfg.n_items, 1_000_000);
        assert_eq!(cfg.n_queries, 100_000);
        assert_eq!(cfg.selected_targets.len(), 6);
        assert_eq!(cfg.suites.len(), 6);
        assert!(!cfg.run_batch);
    }

    #[test]
    fn test_cli_target_and_suite_filter() {
        let args = vec![
            "-t".to_string(),
            "multilevel,pgm".to_string(),
            "-s".to_string(),
            "sosd".to_string(),
            "-r".to_string(),
            "5".to_string(),
        ];
        let cfg = parse_cli_args(&args).expect("Valid args should parse");
        assert_eq!(cfg.selected_targets.len(), 2);
        assert!(cfg.selected_targets.contains(&BenchmarkTarget::Multilevel));
        assert!(cfg.selected_targets.contains(&BenchmarkTarget::Pgm));

        let args_eyt = vec!["-t".to_string(), "eytzinger,eyt-index".to_string()];
        let cfg_eyt = parse_cli_args(&args_eyt).expect("Eytzinger targets should parse");
        assert_eq!(cfg_eyt.selected_targets.len(), 2);
        assert!(
            cfg_eyt
                .selected_targets
                .contains(&BenchmarkTarget::Eytzinger)
        );
        assert!(
            cfg_eyt
                .selected_targets
                .contains(&BenchmarkTarget::EytzingerIndex)
        );
        assert_eq!(cfg.suites.len(), 3); // 3 SOSD suites
        assert_eq!(cfg.num_runs, 5);
    }

    #[test]
    fn test_cli_batch_mode() {
        let args = vec![
            "--batch".to_string(),
            "--batch-size".to_string(),
            "25000".to_string(),
        ];
        let cfg = parse_cli_args(&args).expect("Batch args should parse");
        assert!(cfg.run_batch);
        assert_eq!(cfg.batch_size, 25_000);
    }
}
