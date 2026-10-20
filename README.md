# Learned Index: Piecewise Geometric Model (PGM-Index) in Rust

[![CI](https://github.com/gilnett/learned-index/actions/workflows/ci.yml/badge.svg)](https://github.com/gilnett/learned-index/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/learned-index.svg)](https://crates.io/crates/learned-index)
[![Docs.rs](https://docs.rs/learned-index/badge.svg)](https://docs.rs/learned-index)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Ko-fi](https://img.shields.io/badge/Ko--fi-gilnet-F16061?style=flat&logo=ko-fi&logoColor=white)](https://ko-fi.com/gilnet)
[![GitHub Sponsors](https://img.shields.io/badge/Sponsor-gilnett-EA4AAA?style=flat&logo=githubsponsors&logoColor=white)](https://github.com/sponsors/gilnett)

A zero-dependency, production-grade implementation of **Piecewise Geometric Model (PGM) Learned Index Structures** in pure Rust.

This repository demonstrates the paradigm shift away from traditional comparison-based pointer hierarchies (such as B-Trees and LSM block indexes) to **mathematical function approximations** of Empirical Cumulative Distribution Functions (CDFs) with provable $\epsilon$ error bounds.

> [!NOTE]
> **Academic Attribution & Acknowledgments**: I am **not** the original creator of Learned Indexes or the Piecewise Geometric Model (PGM-index).
> - The foundational concept of **Learned Index Structures** was originated by **Tim Kraska, Alex Beutel, Ed H. Chi, Jeffrey Dean, and Neoklis Polyzotis** (MIT / Google) in their landmark paper [*"The Case for Learned Index Structures"* (SIGMOD 2018)](https://arxiv.org/abs/1712.01208).
> - The **Piecewise Geometric Model Index (PGM-index)** and its optimal streaming piecewise linear approximation (PLA) algorithm were invented and proven by **Paolo Ferragina and Giorgio Vinciguerra** (University of Pisa) in [*"The PGM-index: a fully-dynamic compressed learned index with provable worst-case bounds"* (PVLDB 2020)](https://www.vldb.org/pvldb/vol13/p1162-ferragina.pdf) ([arXiv:1910.06169](https://arxiv.org/abs/1910.06169), official site: [pgm.di.unipi.it](https://pgm.di.unipi.it/), C++ repository: [gvinciguerra/PGM-index](https://github.com/gvinciguerra/PGM-index)).
> 
> This repository (`learned-index`) is an independent, clean-room educational and production-oriented implementation in pure safe Rust, built to explore, benchmark, and evaluate learned indexing mechanics within the modern systems ecosystem.

---

## Table of Contents

- [Overview & The Paradigm Shift](#overview--the-paradigm-shift)
- [Mathematical Foundation](#mathematical-foundation)
  - [Optimal Streaming Piecewise Linear Approximation (PLA)](#optimal-streaming-piecewise-linear-approximation-pla)
  - [Bounded Local Search](#bounded-local-search)
- [Architectural Comparison & Hardware Dynamics](#architectural-comparison--hardware-dynamics)
- [Decision Matrix: When to Use PGM-Index vs. B-Tree](#decision-matrix-when-to-use-pgm-index-vs-b-tree)
- [Statistical Benchmark Suite (10-Run Rigorous Analysis)](#statistical-benchmark-suite-10-run-rigorous-analysis)
  - [1. Dense Sequential Time-Series Data](#1-dense-sequential-time-series-data)
  - [2. Uniform Sparse Random Keys](#2-uniform-sparse-random-keys)
  - [3. Clustered Heavy-Tailed Lognormal Distribution](#3-clustered-heavy-tailed-lognormal-distribution)
- [Real-World Production Applications](#real-world-production-applications)
- [Quickstart & Code Example](#quickstart--code-example)
- [Reproducibility & Verification](#reproducibility--verification)
- [Resources & References](#resources--references)
  - [Referenced Repositories & Implementations](#referenced-repositories--implementations)
  - [Historical Documentation & Version Tracking](#historical-documentation--version-tracking)
- [License](#license)


---

## Overview & The Paradigm Shift

For more than 50 years, database indexing has relied on tree-based data structures (`B-Tree`, `B+Tree`, `LSM-Tree`). While mature, B-Trees possess inherent physical constraints on modern CPU architectures:

1. **Pointer Chasing**: Navigating down a tree requires sequential memory dereferences across disparate memory addresses, causing recurring L1/L2/L3 cache misses.
2. **High Memory Overhead**: Tree nodes require significant metadata per key (child pointers, balance factors, keys, values), consuming 30–50 bytes per entry.
3. **Branch Misprediction**: Binary comparisons at each node introduce branch penalties on deeply pipelined modern CPUs.

**Learned Indexes** (originally introduced by Kraska et al., MIT / Google) reframe indexing as a regression problem:
Given a sorted sequence of keys $K = (k_0, k_1, \dots, k_{N-1})$, the position of any key $k$ in memory corresponds to the scaled Empirical Cumulative Distribution Function:

$$\text{Pos}(k) \approx N \times \text{CDF}(k)$$

Rather than allocating deep node hierarchies, a **PGM-Index** partitions the data into a minimal set of linear segments $y = ax + b$, guaranteeing that the true position of any indexed key is within a strict error window $[y - \epsilon, y + \epsilon]$.

---

## Mathematical Foundation

### Optimal Streaming Piecewise Linear Approximation (PLA)

The index builder executes in a single streaming pass ($O(N)$ time, $O(1)$ extra space). 

For a segment starting at point $(x_0, y_0)$ and a given maximum error tolerance $\epsilon$:
Each subsequent point $(x_i, y_i)$ restricts the feasible slope interval $[s_{\min}, s_{\max}]$:

$$s_{\min} = \max_{j} \left[ \frac{(y_j - \epsilon) - y_0}{x_j - x_0} \right]$$

$$s_{\max} = \min_{j} \left[ \frac{(y_j + \epsilon) - y_0}{x_j - x_0} \right]$$

As long as $s_{\min} \le s_{\max}$, all points observed so far can be represented by a single straight line with error at most $\epsilon$. Once $s_{\min} > s_{\max}$, the segment is finalized with slope $s = \frac{s_{\min} + s_{\max}}{2}$, and a new segment starts.

### Bounded Local Search

During point lookup:
1. The responsible segment is found via a rapid binary search across the segment array ($O(\log S)$ where $S \ll N$).
2. The position estimate is computed directly via ALU instructions:
   $$\text{Pos}_{\text{approx}} = \text{round}(y_0 + s \cdot (k - x_0))$$
3. The exact match is retrieved via a local binary search bounded strictly inside:
   $$[\max(0, \text{Pos}_{\text{approx}} - \epsilon), \min(N - 1, \text{Pos}_{\text{approx}} + \epsilon)]$$

- **Time Complexity**: $O(\log S + \log \epsilon)$
- **Space Complexity**: $O(S)$ where $S \approx \frac{N}{\text{compression ratio}}$
- **Correctness**: **100% exact** (0 false positives, 0 false negatives).

---

## Architectural Comparison & Hardware Dynamics

Starting with v0.2.0, `learned-index` now evaluates PGM against **both** common baseline approaches:
1. [**`std::collections::BTreeMap`**](https://doc.rust-lang.org/std/collections/struct.BTreeMap.html): The standard library mutable, dynamic pointer-based search tree.
2. [**`Flat slice::binary_search`**](https://doc.rust-lang.org/std/primitive.slice.html#method.binary_search): The standard library immutable baseline (contiguous array binary search).


| Dimension | Standard `BTreeMap` | Flat `slice::binary_search` | `PGM-Index` (Learned) | Microarchitectural Advantage |
| :--- | :--- | :--- | :--- | :--- |
| **Index Structure** | Dynamic nodes across heap | Contiguous raw buffer | Compact array of $(a, b, k_0)$ segments | Metadata fits entirely inside **L1/L2 cache** |
| **Lookup Mechanism** | 4–6 node hops across RAM | $\sim 20$ binary jumps across 8 MB | 1 in-register math eval + local 4 cache lines | Eliminates DRAM stalls & cache pollution |
| **Index Metadata** | $\sim 45\text{ MB}$ per $10^6$ keys | $0\text{ KB}$ (data must stay resident) | **3.1 KB – 48 KB** per $10^6$ keys | **-99.9% index memory vs B-Tree** |
| **Lookup Latency** | $220 - 290\text{ ns}$ | $140 - 160\text{ ns}$ | **48 - 70 ns** | **3x faster vs B-Tree, ~2.2x vs Slice** |
| **Construction Time** | $\sim 70\text{ ms}$ ($O(N \log N)$) | $0\text{ ms}$ (pre-sorted input) | **$3.5\text{ ms}$** ($O(N)$ streaming) | **~18x faster construction than B-Tree** |

---

## Decision Matrix: When to Use PGM-Index vs. B-Tree

| Workload Characteristic | Recommended Structure | Rationale |
| :--- | :--- | :--- |
| **Frequent in-place mutations / OLTP** | **`B-Tree` / `B+Tree`** | Better for concurrent writes, transactional locking, and dynamic in-place page splits. |
| **Immutable / Read-heavy sorted datasets** | **`PGM-Index`** | Ideal for SSTables in LSM engines (RocksDB), columnar row groups (DuckDB, ClickHouse), and static tables. |
| **Severe memory constraints (RAM / Cache)** | **`PGM-Index`** | Compresses index metadata up to 14,000x, enabling billions of keys to fit into resident CPU cache. |
| **Contiguous in-memory binary search** | **`PGM-Index`** | Up to **2.9x faster** than flat `slice::binary_search` by cutting memory traffic from 20 cache lines down to 4. |

---

## Statistical Benchmark Suite (10-Run Rigorous Analysis)

All benchmarks were measured locally on x86_64 hardware using Rust in `--release` mode.
Each scenario was executed over **10 consecutive runs** with distinct randomized seeds to eliminate CPU scheduling noise and warm-cache anomalies.

- **Dataset Size**: 1,000,000 keys (64-bit integers)
- **Queries per Run**: 100,000 queries (50% existing keys / hits, 50% non-existent keys / misses)

### 1. Dense Sequential Time-Series Data

Simulates monotonically increasing timestamps, network log sequences, or primary auto-increment keys.

```
Distribution: Dense Timestamp | Tolerance: epsilon = 32
```

| Metric | `BTreeMap` | Flat `slice::binary_search` | `PGM-Index` (Learned) | Improvement Factor |
| :--- | :--- | :--- | :--- | :--- |
| **Mean Build Time** | 67.94 ms | 0.00 ms (pre-sorted) | 3.65 ms | **18.6x faster than BTree** |
| **Index-Only Memory** | 45.78 MB | 0 KB | 3.1 KB | **14,700x compression** |
| **Mean Hit Latency** | 221.8 ns/op | 152.7 ns/op | **63.5 ns/op** | **3.50x vs BTree \| 2.41x vs Slice** |
| **Mean Miss Latency** | 10.0 ns/op | 23.7 ns/op | **0.7 ns/op** | **14.3x faster negative rejection** |
| **Segments Count** | N/A | N/A | 83 segments | **12,048x factor** |
| **Verification** | 100.0% match | 100.0% match | 100.0% match | **0 discrepancies across 10 runs** |

---

### 2. Uniform Sparse Random Keys

Simulates randomly distributed 64-bit object IDs, hashed identifiers, and sparse primary keys.

```
Distribution: Uniform Sparse Random | Tolerance: epsilon = 16
```

| Metric | `BTreeMap` | Flat `slice::binary_search` | `PGM-Index` (Learned) | Improvement Factor |
| :--- | :--- | :--- | :--- | :--- |
| **Mean Build Time** | 68.79 ms | 0.00 ms (pre-sorted) | 3.82 ms | **18.0x faster than BTree** |
| **Index-Only Memory** | 45.78 MB | 0 KB | 48.1 KB | **-99.89% memory reduction** |
| **Mean Hit Latency** | 223.0 ns/op | 150.9 ns/op | **70.5 ns/op** | **3.16x vs BTree \| 2.14x vs Slice** |
| **Mean Miss Latency** | 214.6 ns/op | 104.0 ns/op | **68.1 ns/op** | **3.15x vs BTree \| 1.53x vs Slice** |
| **Segments Count** | N/A | N/A | 1,406 segments | **711x compression** |
| **Verification** | 100.0% match | 100.0% match | 100.0% match | **0 discrepancies across 10 runs** |

### 3. Clustered Heavy-Tailed Lognormal Distribution

Simulates real-world power-law distributions (web traffic requests, financial transaction sizes, file size distributions).

```
Distribution: Lognormal Clustered | Tolerance: epsilon = 16
```

| Metric | `BTreeMap` | Flat `slice::binary_search` | `PGM-Index` (Learned) | Improvement Factor |
| :--- | :--- | :--- | :--- | :--- |
| **Mean Build Time** | 69.75 ms | 0.00 ms (pre-sorted) | 3.90 ms | **17.9x faster than BTree** |
| **Index-Only Memory** | 45.78 MB | 0 KB | 12.1 KB | **-99.97% memory reduction** |
| **Mean Hit Latency** | 216.4 ns/op | 141.0 ns/op | **48.6 ns/op** | **4.45x vs BTree \| 2.90x vs Slice** |
| **Mean Miss Latency** | 61.4 ns/op | 39.8 ns/op | **38.2 ns/op** | **1.61x vs BTree \| 1.04x vs Slice** |
| **Segments Count** | N/A | N/A | 455 segments | **2,197x compression** |
| **Verification** | 100.0% match | 100.0% match | 100.0% match | **0 discrepancies across 10 runs** |

---

## Real-World Production Applications

### 1. Columnar OLAP Databases ([ClickHouse](https://github.com/ClickHouse/ClickHouse), [DuckDB](https://github.com/duckdb/duckdb), [Apache Parquet](https://github.com/apache/parquet-format))
Analytical engines ingest massive sorted blocks. PGM-Index acts as a lightweight zone map replacement, locating record offsets across millions of rows with minimal CPU instruction overhead.

### 2. LSM-Tree Storage Engines ([RocksDB](https://github.com/facebook/rocksdb), [ScyllaDB](https://github.com/scylladb/scylladb), [LevelDB](https://github.com/google/leveldb))
In an LSM storage engine, SSTables written to NVMe are strictly immutable. Replacing traditional sparse block index tables ([RocksDB BlockBasedTable format](https://github.com/facebook/rocksdb/wiki/Rocksdb-BlockBasedTable-Format)) with PGM segments allows billions of keys to be indexed **entirely in host CPU cache or RAM**, reducing NVMe read amplification to exactly 1 physical block I/O per query.


### 3. OS File Indexing & Cataloging
Fast local file indexers (e.g., Everything, NTFS USN Journal analyzers) can catalog tens of millions of records within a few megabytes of RAM while delivering sub-100ns path resolution.

### 4. High-Throughput Network Routing (FIB)
Forwarding Information Bases (FIB) at 100+ Gbps line rates require deterministic sub-microsecond packet IP classification where L1/L2 cache saturation would immediately induce packet drops.

---

## Quickstart & Code Example

Add `learned-index` to your `Cargo.toml`:

```rust
use learned_index::PgmIndex;

fn main() {
    // 1. Prepare sorted dataset
    let data: Vec<u64> = vec![
        100, 105, 110, 115, 120, 250, 500, 1000, 1500, 2000, 5000, 10000
    ];

    // 2. Build PGM index with error bound epsilon (e.g. 16 or 32)
    let pgm = PgmIndex::build(data, 16);

    // 3. Exact point lookup (O(log S + log epsilon))
    match pgm.find(1500) {
        Some(index) => println!("Found key at position: {}", index),
        None => println!("Key not present in dataset"),
    }

    // 4. Predecessor / Lower Bound search (first element >= target)
    let lb_idx = pgm.lower_bound(1200);
    println!("Lower bound for 1200 is at index: {} (value: {})", lb_idx, pgm.data[lb_idx]);

    // 5. Zero-copy Range Query (slice scan)
    let slice = pgm.range(110..=2000);
    println!("Keys in range [110, 2000]: {:?}", slice);

    // 6. Inspect memory overhead
    println!("Total segments created: {}", pgm.segments.len());
    println!("Index-only memory: {} bytes", pgm.index_only_bytes());
}
```

---

## Reproducibility & Verification

To verify the test suite and run the full 10-run statistical benchmark on your own machine:

```bash
# Clone the repository
git clone https://github.com/gilnett/learned-index.git
cd learned-index

# Run unit tests covering edge cases, gaps, and duplicates
cargo test

# Execute the 10-run release benchmark suite
cargo run --release
```

---

## Resources & References

### Primary Article & Technical Deep Dive
- **[Why We've Been Indexing Data Wrong for 50 Years: Replacing B-Trees with Math in Rust](https://dev.to/gilnett/why-weve-been-indexing-data-wrong-for-50-years-replacing-b-trees-with-math-in-rust-nch)**: A comprehensive architectural breakdown exploring CPU cache dynamics, the memory wall, mathematical CDF approximation, and real-world system implications of replacing pointer hierarchies with linear functions.

### Seminal Research Papers
- **Kraska et al. (2018)**: *"The Case for Learned Index Structures"* (SIGMOD 2018) – [arXiv:1712.01208](https://arxiv.org/abs/1712.01208). Seminal paper proving that sorted indexes can be modeled as cumulative distribution function (CDF) regressions.
- **Ferragina & Vinciguerra (2020)**: *"The PGM-index: a fully-dynamic compressed learned index with provable worst-case bounds"* (PVLDB 2020) – [PVLDB (PDF)](https://www.vldb.org/pvldb/vol13/p1162-ferragina.pdf) / [arXiv:1910.06169](https://arxiv.org/abs/1910.06169). Establishes optimal piecewise linear approximation (PLA) under bounded error corridors $\epsilon$.
- **Ferragina & Vinciguerra (2020)**: Reference C++ PGM implementation & resources – [pgm.di.unipi.it](https://pgm.di.unipi.it/) / [GitHub: gvinciguerra/PGM-index](https://github.com/gvinciguerra/PGM-index).

### Referenced Repositories & Implementations
- **[gvinciguerra/PGM-index](https://github.com/gvinciguerra/PGM-index)**: Official C++ reference implementation of the Piecewise Geometric Model index.
- **[std::collections::BTreeMap](https://doc.rust-lang.org/std/collections/struct.BTreeMap.html)**: Rust Standard Library B-Tree implementation.
- **[slice::binary_search](https://doc.rust-lang.org/std/primitive.slice.html#method.binary_search)**: Rust Standard Library core slice binary search implementation.
- **[facebook/rocksdb](https://github.com/facebook/rocksdb)**: High-performance embedded key-value database using LSM-Trees and [BlockBasedTable format](https://github.com/facebook/rocksdb/wiki/Rocksdb-BlockBasedTable-Format).
- **[scylladb/scylladb](https://github.com/scylladb/scylladb)**: Low-latency NoSQL columnar database written in C++.
- **[google/leveldb](https://github.com/google/leveldb)**: Foundational LSM-Tree storage engine written by Sanjay Ghemawat and Jeff Dean.
- **[duckdb/duckdb](https://github.com/duckdb/duckdb)**: High-performance embedded analytical columnar database.
- **[ClickHouse/ClickHouse](https://github.com/ClickHouse/ClickHouse)**: High-performance columnar database management system for real-time analytics.
- **[apache/parquet-format](https://github.com/apache/parquet-format)**: Open-source columnar storage format for deep nested and flat record data.

### Historical Documentation & Version Tracking
- **[v0.1.1 Documentation Archive](docs/history/README_v0.1.1.md)**: Preserves the initial v0.1.1 documentation, focusing strictly on point query lookups (`find`) against `BTreeMap` prior to the introduction of `slice::binary_search` baselines, `lower_bound`, and range scans.


---

## License

Licensed under the **Apache License, Version 2.0** ([LICENSE](LICENSE) or [http://www.apache.org/licenses/LICENSE-2.0](http://www.apache.org/licenses/LICENSE-2.0)).

