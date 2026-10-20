# Learned Index: Piecewise Geometric Model (PGM-Index) in Rust

[![CI](https://github.com/gilnett/learned_index_lab/actions/workflows/ci.yml/badge.svg)](https://github.com/gilnett/learned_index_lab/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/learned_index_lab.svg)](https://crates.io/crates/learned_index_lab)
[![Docs.rs](https://docs.rs/learned_index_lab/badge.svg)](https://docs.rs/learned_index_lab)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Ko-fi](https://img.shields.io/badge/Ko--fi-gilnet-F16061?style=flat&logo=ko-fi&logoColor=white)](https://ko-fi.com/gilnet)
[![GitHub Sponsors](https://img.shields.io/badge/Sponsor-gilnett-EA4AAA?style=flat&logo=githubsponsors&logoColor=white)](https://github.com/sponsors/gilnett)

A clean, zero-dependency Rust implementation and comparative benchmark suite of **Piecewise Geometric Model (PGM) Learned Index Structures**.

This repository evaluates mathematical function approximations of Empirical Cumulative Distribution Functions (CDFs) against standard pointer trees (`BTreeMap`), contiguous binary search (`slice::binary_search`), and cache-conscious breadth-first search (`EytzingerSearch`).

> [!NOTE]
> **Academic Attribution**: Learned Indexes were originated by Kraska et al. (MIT/Google, SIGMOD 2018). The PGM-index was invented and proven by Paolo Ferragina and Giorgio Vinciguerra (University of Pisa, PVLDB 2020). The Eytzinger layout was formalized by Paul-Virak Khuong and Pat Morin (ACM JEA 2017). This repository is an independent, clean-room implementation in pure safe Rust.

---

## Architectural Comparison (4 Search Paradigms)

| Dimension | Standard `BTreeMap` | Flat `slice::binary_search` | `EytzingerSearch` (Breadth-First) | `PGM-Index` (Learned Model) |
| :--- | :--- | :--- | :--- | :--- |
| **Data Layout** | Heap node hierarchy | Contiguous sorted slice | 1-based complete binary tree array | Compact array of $(a, b, k_0)$ segments |
| **Lookup Mechanism** | 4–6 pointer hops across heap | $\sim 20$ binary jumps across RAM | Branchless arithmetic traversal (`k = 2k + bit`) | In-register ALU prediction + bounded local scan |
| **Cache Locality** | Poor (scattered heap nodes) | Medium (strided binary jumps) | High (upper tree levels in L1/L2) | **Highest** (segments fit entirely in L1/L2) |
| **Index Overhead** | $\sim 45.78\text{ MB}$ per $10^6$ keys | $0\text{ KB}$ (in-situ data only) | $7.63\text{ MB}$ per $10^6$ keys (duplicate layout) | **3.1 KB – 48.1 KB** (-99.9% vs BTree) |
| **Primary Strength** | Dynamic in-place mutations | Zero extra memory | Branchless search, cache friendly | **Sub-microsecond lookups, extreme compression** |

---

## Benchmark Summary (v0.3.0 Consolidated Results)

Measured on x86_64 hardware in `--release` mode. Consolidated average across all 3 workload distributions (30 independent runs per target structure with a 32MB L1-L3 CPU cache eviction barrier between phases):

| Cible / Target | Build Time | Hit Latency | Miss Latency | Lower Bound | Index Memory | Moyenne Totale Latence | Speedup vs BTree |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **BTreeMap** | 84.12 ms | 314.4 ns | 331.4 ns | 350.0 ns | 45.78 MB | 331.9 ns | 1.00x |
| **Flat Slice** | 0.00 ms | 282.7 ns | 272.9 ns | 275.3 ns | 0.00 KB | 277.0 ns | 1.20x |
| **Eytzinger** | 6.30 ms | 163.7 ns | 160.9 ns | **169.2 ns** | 7.63 MB | **164.6 ns** | **2.02x** |
| **PGM-Index** | 6.82 ms | **153.0 ns** | **154.1 ns** | 208.6 ns | **21.1 KB** | 171.9 ns | **1.93x** |
| **MOYENNE GLOBALE** | **24.31 ms** | **228.4 ns** | **229.8 ns** | **250.8 ns** | **13.36 MB** | **236.3 ns** | — |

> Detailed breakdown per distribution (Dense Time-Series, Uniform Sparse, and Heavy-Tailed Lognormal) is available in [docs/benchmarks/statistical_benchmarks.md](docs/benchmarks/statistical_benchmarks.md).

---

## Roadmap & Architecture Evolution

- **v0.1.0 – v0.2.0** *(Completed)*:
  - Streaming PLA index builder ($O(N)$ single-pass construction).
  - Exact point queries (`find`), predecessor search (`lower_bound`), and zero-copy slice scans (`range`).
  - Baseline comparison against `BTreeMap` and `slice::binary_search`.
  - Multi-platform CI matrix (Ubuntu, Windows, macOS).
- **v0.3.0** *(Current Release)*:
  - Cache-conscious Eytzinger array layout baseline (`EytzingerSearch`) with branchless search.
  - Multi-target isolated benchmark harness with `--target` selector and 32MB CPU cache eviction barrier.
  - Single-threaded isolated test execution (`--test-threads=1`).
  - Per-target summary matrix with combined total average metrics.
  - Documentation modularization into dedicated `docs/` topic guides.
- **v0.4.0** *(Planned Milestone)*:
  - SOSD (Search on Sorted Data) real-world dataset ingestion (OpenStreetMap, Books, Wikipedia).
  - Skewed Zipfian query distribution generator.
  - Multi-level recursive PGM indexing for massive datasets ($S > 10^5$).
- **v0.5.0** *(Future Monorepo Architecture)*:
  - Cargo Workspace structure within this repository:
    - `crates/learned-index`: Pure, zero-dependency production storage engine core.
    - `crates/index-benchmarks`: Exhaustive comparative testbed.
  - Dynamic LSM extensions (buffered writes with asynchronous background segment compaction).

---

## Quickstart & Code Example

Add `learned-index` to your `Cargo.toml`:

```toml
[dependencies]
learned-index = "0.3.0"
```

```rust
use learned_index::{EytzingerSearch, PgmIndex};

fn main() {
    // 1. Prepare sorted dataset
    let data: Vec<u64> = vec![
        100, 105, 110, 115, 120, 250, 500, 1000, 1500, 2000, 5000, 10000
    ];

    // 2. Build PGM index with error bound epsilon (e.g. 16 or 32)
    let pgm = PgmIndex::build(data.clone(), 16);

    // 3. Exact point lookup (O(log S + log epsilon))
    assert_eq!(pgm.find(1500), Some(8));

    // 4. Predecessor / Lower Bound search (first element >= target)
    let lb_idx = pgm.lower_bound(1200);
    println!("Lower bound for 1200 is index {} (value: {})", lb_idx, pgm.data[lb_idx]);

    // 5. Zero-copy Range Query (slice scan)
    let slice = pgm.range(110..=2000);
    println!("Keys in range [110, 2000]: {:?}", slice);

    // 6. Cache-conscious Eytzinger layout baseline
    let eyt = EytzingerSearch::build(&data);
    assert!(eyt.contains(1500));
}
```

---

## Reproducibility & Verification

The v0.3.0 test suite and benchmark harness support strict single-threaded isolation and modular target selection:

```bash
# Clone the repository
git clone https://github.com/gilnett/learned_index_lab.git
cd learned_index_lab

# Run the unit test suite with thread isolation
cargo test --release -- --test-threads=1

# Execute the full 10-run release benchmark across all 4 targets
cargo run --release

# Run isolated benchmarks on specific targets
cargo run --release -- --target eytzinger,pgm --runs 10
cargo run --release -- --target pgm --suite uniform
```

---

## Documentation Index

- **Theoretical Foundations**:
  - [CDF Approximation in Learned Indexes](docs/theory/cdf_learned_index.md): Mathematical regression formulation.
  - [Optimal Streaming PLA](docs/theory/streaming_pla.md): Single-pass bounded slope corridor algorithm.
  - [Eytzinger Layout](docs/theory/eytzinger_layout.md): Complete binary tree array with branchless traversal.
  - [Bounded Local Search](docs/theory/bounded_local_search.md): In-register ALU bounds and local verification.
  - [Theory Index & Full Papers](docs/theory/README.md): Overview with direct links to academic publications.
- **Benchmarks & Performance**:
  - [Statistical Benchmarks (10-Run Matrix)](docs/benchmarks/statistical_benchmarks.md): Detailed results for Dense Time-Series, Uniform Sparse, and Heavy-Tailed Lognormal distributions.
- **Architecture & Use Cases**:
  - [Production Architecture & Real-World Use Cases](docs/architecture/use_cases_and_production.md): OLAP columnar zone maps, LSM-Tree NVMe cache residency, and decision matrices.
- **Historical Archives**:
  - [v0.2.0 Documentation Archive](docs/history/README_v0.2.0.md)
  - [v0.1.1 Documentation Archive](docs/history/README_v0.1.1.md)

---

## References

### Technical Articles & Deep Dives
- **[Why We've Been Indexing Data Wrong for 50 Years: Replacing B-Trees with Math in Rust](https://dev.to/gilnett/why-weve-been-indexing-data-wrong-for-50-years-replacing-b-trees-with-math-in-rust-nch)**: Architectural breakdown of CPU cache dynamics, the memory wall, mathematical CDF approximation, and real-world system implications of replacing pointer hierarchies with linear functions.
- **[Learned Indexes in Rust v0.2: Answering Reddit, Adding Range Queries, and Beating B-Trees](https://dev.to/gilnett/learned-indexes-in-rust-v02-answering-reddit-adding-range-queries-and-beating-pd6)**: Community technical follow-up addressing memory overhead questions, implementing predecessor search (`lower_bound`), slice scans, and expanding benchmark baselines.

### Seminal Research Papers
- **Kraska et al. (2018)**: *"The Case for Learned Index Structures"* (ACM SIGMOD 2018) – [arXiv:1712.01208](https://arxiv.org/abs/1712.01208). Seminal paper proving that sorted indexes can be modeled as cumulative distribution function (CDF) regressions.
- **Ferragina & Vinciguerra (2020)**: *"The PGM-index: a fully-dynamic compressed learned index with provable worst-case bounds"* (PVLDB 2020) – [PVLDB Official (PDF)](https://www.vldb.org/pvldb/vol13/p1162-ferragina.pdf) / [arXiv:1910.06169](https://arxiv.org/abs/1910.06169). Establishes optimal piecewise linear approximation (PLA) under bounded error corridors $\epsilon$.
- **Khuong & Morin (2017)**: *"Array Layouts for Comparison-Based Searching"* (ACM JEA 2017) – [arXiv:1509.05053](https://arxiv.org/abs/1509.05053). Formalizes the cache-conscious Eytzinger array layout and branchless level-order search.

### Reference Codebases & Implementations
- **[gvinciguerra/PGM-index](https://github.com/gvinciguerra/PGM-index)**: Official C++ reference implementation of the Piecewise Geometric Model index (University of Pisa, [pgm.di.unipi.it](https://pgm.di.unipi.it/)).
- **[std::collections::BTreeMap](https://doc.rust-lang.org/std/collections/struct.BTreeMap.html)**: Rust Standard Library B-Tree implementation.
- **[slice::binary_search](https://doc.rust-lang.org/std/primitive.slice.html#method.binary_search)**: Rust Standard Library core slice binary search.
- **[Algorithmica: Eytzinger Binary Search](https://en.algorithmica.org/hpc/data-structures/binary-search/)**: Microarchitectural analysis of cache line alignment and branchless search.

### Industry Storage Engines & Analytical Systems
- **[facebook/rocksdb](https://github.com/facebook/rocksdb)**: High-performance embedded key-value database using LSM-Trees and [BlockBasedTable format](https://github.com/facebook/rocksdb/wiki/Rocksdb-BlockBasedTable-Format).
- **[duckdb/duckdb](https://github.com/duckdb/duckdb)**: High-performance embedded analytical columnar database.
- **[ClickHouse/ClickHouse](https://github.com/ClickHouse/ClickHouse)**: High-performance columnar database management system for real-time analytics.
- **[apache/parquet-format](https://github.com/apache/parquet-format)**: Open-source columnar storage format for deep nested and flat record data.
- **[scylladb/scylladb](https://github.com/scylladb/scylladb)**: Low-latency NoSQL columnar database written in C++.
- **[google/leveldb](https://github.com/google/leveldb)**: Foundational LSM-Tree storage engine written by Sanjay Ghemawat and Jeff Dean.

---

## License

Licensed under the **Apache License, Version 2.0** ([LICENSE](LICENSE) or [http://www.apache.org/licenses/LICENSE-2.0](http://www.apache.org/licenses/LICENSE-2.0)).
