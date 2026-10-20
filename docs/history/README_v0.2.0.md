# Learned Index: Piecewise Geometric Model (PGM-Index) in Rust (v0.2.0 Archive)

[![CI](https://github.com/gilnett/learned_index_lab/actions/workflows/ci.yml/badge.svg)](https://github.com/gilnett/learned_index_lab/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/learned_index_lab.svg)](https://crates.io/crates/learned_index_lab)
[![Docs.rs](https://docs.rs/learned_index_lab/badge.svg)](https://docs.rs/learned_index_lab)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Ko-fi](https://img.shields.io/badge/Ko--fi-gilnet-F16061?style=flat&logo=ko-fi&logoColor=white)](https://ko-fi.com/gilnet)
[![GitHub Sponsors](https://img.shields.io/badge/Sponsor-gilnett-EA4AAA?style=flat&logo=githubsponsors&logoColor=white)](https://github.com/sponsors/gilnett)

> [!NOTE]
> **Historical Archive**: This document preserves the state of the documentation for **v0.2.0** of `learned-index`, which introduced comparison against `Flat slice::binary_search`, `lower_bound`, and range scans.

---

A zero-dependency, production-grade implementation of **Piecewise Geometric Model (PGM) Learned Index Structures** in pure Rust.

This repository demonstrates the paradigm shift away from traditional comparison-based pointer hierarchies (such as B-Trees and LSM block indexes) to **mathematical function approximations** of Empirical Cumulative Distribution Functions (CDFs) with provable $\epsilon$ error bounds.

> [!NOTE]
> **Academic Attribution & Acknowledgments**: I am **not** the original creator of Learned Indexes or the Piecewise Geometric Model (PGM-index).
> - The foundational concept of **Learned Index Structures** was originated by **Tim Kraska, Alex Beutel, Ed H. Chi, Jeffrey Dean, and Neoklis Polyzotis** (MIT / Google) in their landmark paper [*"The Case for Learned Index Structures"* (SIGMOD 2018)](https://arxiv.org/abs/1712.01208).
> - The **Piecewise Geometric Model Index (PGM-index)** and its optimal streaming piecewise linear approximation (PLA) algorithm were invented and proven by **Paolo Ferragina and Giorgio Vinciguerra** (University of Pisa) in [*"The PGM-index: a fully-dynamic compressed learned index with provable worst-case bounds"* (PVLDB 2020)](https://www.vldb.org/pvldb/vol13/p1162-ferragina.pdf) ([arXiv:1910.06169](https://arxiv.org/abs/1910.06169), official site: [pgm.di.unipi.it](https://pgm.di.unipi.it/), C++ repository: [gvinciguerra/PGM-index](https://github.com/gvinciguerra/PGM-index)).
> 
> This repository (`learned-index`) is an independent, clean-room educational and production-oriented implementation in pure safe Rust, built to explore, benchmark, and evaluate learned indexing mechanics within the modern systems ecosystem.

---

## Architectural Comparison & Hardware Dynamics

Starting with v0.2.0, `learned-index` evaluates PGM against both common baseline approaches:
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

## Statistical Benchmark Suite (10-Run Rigorous Analysis)

All benchmarks were measured locally on x86_64 hardware using Rust in `--release` mode.
Each scenario was executed over **10 consecutive runs** with distinct randomized seeds to eliminate CPU scheduling noise and warm-cache anomalies.

- **Dataset Size**: 1,000,000 keys (64-bit integers)
- **Queries per Run**: 100,000 queries (50% existing keys / hits, 50% non-existent keys / misses)

### 1. Dense Sequential Time-Series Data

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

---

### 3. Clustered Heavy-Tailed Lognormal Distribution

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
