# Statistical Benchmarks & Comparative Analysis (v0.3.0)

This document contains the comprehensive evaluation methodology, workload distributions, and per-scenario statistical results for the 4 indexed structures:
1. **`std::collections::BTreeMap`**: Rust standard library pointer-based dynamic search tree.
2. **`Flat slice::binary_search`**: In-situ contiguous sorted slice binary search.
3. **`EytzingerSearch`**: 1-based complete binary tree array layout with branchless traversal.
4. **`PgmIndex`**: Learned piecewise geometric model with bounded linear corridors.

---

## Benchmark Methodology

- **Hardware Platform**: x86_64, Windows, Rust `--release` mode.
- **Dataset Size**: 1,000,000 keys (64-bit unsigned integers).
- **Queries**: 100,000 lookups per run (50% hits, 50% internal misses within $[k_0, k_{N-1}]$).
- **Repetitions**: 10 consecutive isolated runs per target with distinct pseudorandom seeds.
- **Cache Isolation**: 32MB L1-L3 cache eviction barrier (`clear_cpu_cache()`) executed before every construction and lookup phase.
- **Thread Safety**: Single-threaded execution (`--test-threads=1`) to eliminate core scheduling jitter.

---

## Detailed Results by Distribution

### 1. Dense Sequential Time-Series Data ($\epsilon = 32$)
Simulates monotonically increasing telemetry timestamps, sequential log offsets, or auto-increment primary keys.

| Cible / Target | Build Time | Hit Latency | Miss Latency | Lower Bound | Index Memory | Moyenne Totale Latence | Speedup vs BTree |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **BTreeMap** | 82.01 ms | 305.4 ns | 313.9 ns | 325.7 ns | 45.78 MB | 315.0 ns | 1.00x |
| **Flat Slice** | 0.00 ms | 224.4 ns | 205.3 ns | 219.7 ns | 0.00 KB | 216.5 ns | 1.45x |
| **Eytzinger** | 6.62 ms | 144.2 ns | 144.3 ns | 150.5 ns | 7.63 MB | **146.3 ns** | **2.15x** |
| **PGM-Index** | 8.47 ms | 215.5 ns | 221.0 ns | 278.8 ns | **3.1 KB** | 238.5 ns | 1.32x |
| **MOYENNE GLOBALE** | **24.27 ms** | **222.4 ns** | **221.1 ns** | **243.7 ns** | **13.35 MB** | **229.1 ns** | — |

- **PGM Compression**: 83 segments (12,048x reduction in index metadata vs BTree).

---

### 2. Uniform Sparse Random Keys ($\epsilon = 16$)
Simulates randomly distributed 64-bit cryptographic hashes, UUIDs, or sparse identifiers.

| Cible / Target | Build Time | Hit Latency | Miss Latency | Lower Bound | Index Memory | Moyenne Totale Latence | Speedup vs BTree |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **BTreeMap** | 91.00 ms | 334.0 ns | 360.4 ns | 375.2 ns | 45.78 MB | 356.5 ns | 1.00x |
| **Flat Slice** | 0.00 ms | 232.4 ns | 257.4 ns | 235.3 ns | 0.00 KB | 241.7 ns | 1.47x |
| **Eytzinger** | 6.10 ms | 176.5 ns | 165.1 ns | 168.3 ns | 7.63 MB | 170.0 ns | 2.10x |
| **PGM-Index** | 6.42 ms | **136.5 ns** | **137.5 ns** | 192.8 ns | **48.1 KB** | **155.6 ns** | **2.29x** |
| **MOYENNE GLOBALE** | **25.88 ms** | **219.9 ns** | **230.1 ns** | **242.9 ns** | **13.36 MB** | **230.9 ns** | — |

- **PGM Compression**: 1,405 segments (951x reduction in index metadata vs BTree).

---

### 3. Clustered Heavy-Tailed Lognormal Distribution ($\epsilon = 16$)
Simulates power-law distributions (web traffic requests, financial transaction sizes, document lengths).

| Cible / Target | Build Time | Hit Latency | Miss Latency | Lower Bound | Index Memory | Moyenne Totale Latence | Speedup vs BTree |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **BTreeMap** | 79.34 ms | 303.9 ns | 320.0 ns | 349.2 ns | 45.78 MB | 324.4 ns | 1.00x |
| **Flat Slice** | 0.00 ms | 391.2 ns | 356.0 ns | 371.0 ns | 0.00 KB | 372.7 ns | 0.87x |
| **Eytzinger** | 6.18 ms | 170.5 ns | 173.3 ns | 188.7 ns | 7.63 MB | 177.5 ns | 1.83x |
| **PGM-Index** | 5.55 ms | **106.8 ns** | **103.7 ns** | **154.1 ns** | **12.1 KB** | **121.5 ns** | **2.67x** |
| **MOYENNE GLOBALE** | **22.77 ms** | **243.1 ns** | **238.3 ns** | **265.7 ns** | **13.35 MB** | **249.0 ns** | — |

- **PGM Compression**: 455 segments (3,780x reduction in index metadata vs BTree).

---

## Multi-Scenario Grand Summary (30 Tests Per Target)

Aggregated performance across all 30 independent benchmark runs:

| Cible / Target | Build Time | Hit Latency | Miss Latency | Lower Bound | Index Memory | Moyenne Totale Latence | Speedup vs BTree |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **BTreeMap** | 84.12 ms | 314.4 ns | 331.4 ns | 350.0 ns | 45.78 MB | 331.9 ns | 1.00x |
| **Flat Slice** | 0.00 ms | 282.7 ns | 272.9 ns | 275.3 ns | 0.00 KB | 277.0 ns | 1.20x |
| **Eytzinger** | 6.30 ms | 163.7 ns | 160.9 ns | **169.2 ns** | 7.63 MB | **164.6 ns** | **2.02x** |
| **PGM-Index** | 6.82 ms | **153.0 ns** | **154.1 ns** | 208.6 ns | **21.1 KB** | 171.9 ns | **1.93x** |
| **MOYENNE GLOBALE** | **24.31 ms** | **228.4 ns** | **229.8 ns** | **250.8 ns** | **13.36 MB** | **236.3 ns** | — |

---

## Architectural Findings

1. **L1/L2 Cache Residency**: PGM segment headers consume between 3.1 KB and 48.1 KB for $10^6$ keys, residing entirely inside L1/L2 cache lines throughout query loops.
2. **Branch Elimination**: Eytzinger's branchless `k = 2k + (target > tree[k])` traversal eliminates branch misprediction penalties on modern CPUs, achieving superior hit speed under sequential data.
3. **Local Search Efficiency**: When key distribution exhibits clustering or sparsity, PGM's arithmetic CDF projection reduces the search corridor to a 32-element local binary search, outperforming flat binary search by up to 2.9x.
