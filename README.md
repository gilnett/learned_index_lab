# Learned Index Lab: Modern Search Paradigms in Rust

[![CI](https://github.com/gilnett/learned_index_lab/actions/workflows/ci.yml/badge.svg)](https://github.com/gilnett/learned_index_lab/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/learned_index_lab.svg)](https://crates.io/crates/learned_index_lab)
[![Docs.rs](https://docs.rs/learned_index_lab/badge.svg)](https://docs.rs/learned_index_lab)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Buy Me a Coffee](https://img.shields.io/badge/Buy%20Me%20a%20Coffee-gilnett-FFDD00?style=flat&logo=buymeacoffee&logoColor=black)](https://buymeacoffee.com/gilnett)
[![GitHub Sponsors](https://img.shields.io/badge/Sponsor-gilnett-EA4AAA?style=flat&logo=githubsponsors&logoColor=white)](https://github.com/sponsors/gilnett)

A clean, zero-dependency Rust implementation and comparative benchmark suite of **Piecewise Geometric Model (PGM) Learned Index Structures**.

This repository evaluates mathematical function approximations of Empirical Cumulative Distribution Functions (CDFs) against standard pointer trees (`BTreeMap`), contiguous binary search (`slice::binary_search`), and cache-conscious breadth-first search (`EytzingerSearch`).

> [!NOTE]
> **Academic Attribution**: Learned Indexes were originated by Kraska et al. (MIT/Google, SIGMOD 2018). The PGM-index was invented and proven by Paolo Ferragina and Giorgio Vinciguerra (University of Pisa, PVLDB 2020). The Eytzinger layout was formalized by Paul-Virak Khuong and Pat Morin (ACM JEA 2017). This repository is an independent, clean-room implementation in pure safe Rust.

---

## Architectural Comparison

| Structure | Layout | Lookup | Cache Locality | Index Memory (1M keys) |
| :--- | :--- | :--- | :--- | :--- |
| **`BTreeMap`** | Heap nodes | Pointer hops ($O(\log N)$) | Poor | ~45.8 MB |
| **`Flat Slice`** | Contiguous slice | Binary search ($O(\log N)$) | Medium | 0 KB |
| **`Eytzinger`** | 1-based array | Branchless traversal | High | ~7.6 MB |
| **`PgmIndex`** | Linear segments | ALU projection + local scan | Highest | 3 KB – 48 KB |
| **`MultilevelPgmIndex`** | Recursive PGM tree | Top-down arithmetic ($O(h \log \epsilon)$) | Highest | 3.5 KB – 50 KB |

---

## Benchmark Summary (v0.4.0 Consolidated Results)

Measured on x86_64 hardware in `--release` mode. Consolidated average across all 6 workload distributions (60 independent runs per target structure with a 32MB L1-L3 CPU cache eviction barrier between phases):

| Cible / Target | Build Time | Hit Latency | Miss Latency | Lower Bound | Index Memory | Moyenne Totale Latence | Speedup vs BTree |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **BTreeMap** | 53.63 ms | 182.9 ns | 186.3 ns | 187.4 ns | 45.78 MB | 185.6 ns | 1.00x |
| **Flat Slice** | 0.00 ms | 136.0 ns | 115.5 ns | 117.0 ns | 0.00 KB | 122.8 ns | 1.35x |
| **Eytzinger** | 5.30 ms | 112.6 ns | 107.1 ns | **121.6 ns** | 7.63 MB | 113.8 ns | 1.62x |
| **PGM-Index** | **4.49 ms** | **102.7 ns** | **90.8 ns** | 135.3 ns | **330.6 KB** | **109.6 ns** | **1.78x** |
| **Multilevel PGM** | 4.57 ms | 128.3 ns | 118.7 ns | 181.1 ns | **274.5 KB** | 142.7 ns | 1.43x |

> Detailed breakdown per distribution (Synthetic and SOSD real-world benchmarks) is available in [docs/benchmarks/statistical_benchmarks.md](docs/benchmarks/statistical_benchmarks.md).

---

## Reproducibility & Verification

The v0.4.0 test suite and benchmark harness support strict single-threaded isolation, SOSD real-world suites, and modular target selection:

```bash
# Clone the repository
git clone https://github.com/gilnett/learned_index_lab.git
cd learned_index_lab

# Run the unit test suite with thread isolation
cargo test --release -- --test-threads=1

# Execute the full 10-run release benchmark across all 5 targets
cargo run --release

# Run isolated benchmarks on specific targets (including Multilevel PGM)
cargo run --release -- --target multilevel,pgm --runs 10
cargo run --release -- --target pgm,multilevel --suite sosd

# Run the batch ingestion and rebuild benchmark (SSTable merge vs BTree in-place)
cargo run --release -- --batch --batch-size 100000
```

---

## Documentation Index

- **Theoretical Foundations**:
  - [CDF Approximation in Learned Indexes](docs/theory/cdf_learned_index.md): Mathematical regression formulation.
  - [Optimal Streaming PLA](docs/theory/streaming_pla.md): Single-pass bounded slope corridor algorithm.
  - [Hierarchical Multilevel PGM](docs/theory/hierarchical_multilevel_pgm.md): Recursive segment indexing ($O(h \cdot \log \epsilon)$).
  - [Batch Merge & Streaming Rebuild](docs/theory/batch_merge_rebuild.md): Two-way sorted merge and $O(N + M)$ streaming PLA rebuild.
  - [Eytzinger Layout](docs/theory/eytzinger_layout.md): Complete binary tree array with branchless traversal.
  - [Bounded Local Search](docs/theory/bounded_local_search.md): In-register ALU bounds and local verification.
  - [Theory Index & Full Papers](docs/theory/README.md): Overview with direct links to academic publications.
- **Benchmarks & Performance**:
  - [Statistical Benchmarks (10-Run Matrix)](docs/benchmarks/statistical_benchmarks.md): Detailed results for Dense Time-Series, Uniform Sparse, and Heavy-Tailed Lognormal distributions.
- **Architecture & Use Cases**:
  - [Code Architecture & Memory Layout Guide](docs/architecture/code_architecture_and_memory_layout.md): Internal Rust implementation, cache line alignment, and ALU arithmetic prediction.
  - [Production Architecture & Real-World Use Cases](docs/architecture/use_cases_and_production.md): OLAP columnar zone maps, LSM-Tree NVMe cache residency, and decision matrices.
- **Roadmap & Future Work**:
  - [Project Roadmap](ROADMAP.md): SOSD real-world datasets, batch rebuild benchmarks, and multilevel indexing.
- **Historical Archives**:
  - [v0.3.1 Documentation Archive](docs/history/README_v0.3.1.md)
  - [v0.2.0 Documentation Archive](docs/history/README_v0.2.0.md)
  - [v0.1.1 Documentation Archive](docs/history/README_v0.1.1.md)

---

## References

### Technical Articles & Deep Dives
- **[Why We've Been Indexing Data Wrong for 50 Years: Replacing B-Trees with Math in Rust](https://dev.to/gilnett/why-weve-been-indexing-data-wrong-for-50-years-replacing-b-trees-with-math-in-rust-nch)**: Architectural breakdown of CPU cache dynamics, the memory wall, mathematical CDF approximation, and real-world system implications of replacing pointer hierarchies with linear functions.
- **[Learned Indexes in Rust v0.2: Answering Reddit, Adding Range Queries, and Beating B-Trees](https://dev.to/gilnett/learned-indexes-in-rust-v02-answering-reddit-adding-range-queries-and-beating-pd6)**: Community technical follow-up addressing memory overhead questions, implementing predecessor search (`lower_bound`), slice scans, and expanding benchmark baselines.

### Seminal Research Papers
- **Kraska et al. (2018)**: *"The Case for Learned Index Structures"* (ACM SIGMOD 2018) – [PDF (arXiv)](https://arxiv.org/pdf/1712.01208) · [arXiv:1712.01208](https://arxiv.org/abs/1712.01208). Seminal paper proving that sorted indexes can be modeled as cumulative distribution function (CDF) regressions.
- **Ferragina & Vinciguerra (2020)**: *"The PGM-index: a fully-dynamic compressed learned index with provable worst-case bounds"* (PVLDB 2020) – [PDF (PVLDB)](https://www.vldb.org/pvldb/vol13/p1162-ferragina.pdf) · [PDF (arXiv)](https://arxiv.org/pdf/1910.06169) · [arXiv:1910.06169](https://arxiv.org/abs/1910.06169). Establishes optimal piecewise linear approximation (PLA) under bounded error corridors $\epsilon$.
- **Khuong & Morin (2017)**: *"Array Layouts for Comparison-Based Searching"* (ACM JEA 2017) – [PDF (arXiv)](https://arxiv.org/pdf/1509.05053) · [arXiv:1509.05053](https://arxiv.org/abs/1509.05053). Formalizes the cache-conscious Eytzinger array layout and branchless level-order search.

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
