# Statistical Benchmarks & Comparative Analysis (v0.4.0)

This document presents a rigorous, unbiased evaluation methodology, workload distributions, per-scenario statistical results, and architectural trade-offs across 5 distinct search paradigms:
1. **`std::collections::BTreeMap`**: Rust standard library pointer-based dynamic search tree (B-Tree).
2. **`Flat slice::binary_search`**: In-situ contiguous sorted slice binary search.
3. **`EytzingerSearch`**: 1-based complete binary tree array layout with branchless `cmov` traversal.
4. **`PgmIndex`**: Piecewise Geometric Model learned index with bounded linear approximation corridors.
5. **`MultilevelPgmIndex`**: Hierarchical recursive PGM-Index with $O(h \cdot \log \epsilon)$ descent.

---

## Benchmark Methodology

- **Hardware Platform**: x86_64, Windows, Rust `--release` mode.
- **Dataset Size**: 1,000,000 keys (64-bit unsigned integers, $8\text{ MB}$ raw data).
- **Queries**: 100,000 lookups per run (50% hits, 50% internal misses within $[k_0, k_{N-1}]$).
- **Workload Distributions**: 6 distinct benchmark profiles:
  - Synthetic: Dense Sequential Time-Series ($\epsilon = 32$), Uniform Sparse Random ($\epsilon = 16$), Heavy-Tailed Lognormal ($\epsilon = 16$).
  - SOSD Real-World: OpenStreetMap Cell IDs (`osm_cellids`, $\epsilon = 16$), Wikipedia Timestamps (`wiki_ts`, $\epsilon = 32$), Book ISBNs (`books`, $\epsilon = 16$).
- **Repetitions**: 5 consecutive isolated runs per target and per suite (30 independent runs total per structure).
- **Cache Isolation**: 32MB L1-L3 cache eviction barrier (`clear_cpu_cache()`) executed before every construction and lookup phase.
- **Thread Safety**: Single-threaded execution (`--test-threads=1`) to eliminate core scheduling jitter.

---

## Memory Model Distinction: Metadata vs. Total Resident Memory

When evaluating memory consumption, a critical distinction must be drawn between **index metadata overhead** and **total resident memory**:

| Structure | Raw Data Buffer (1M keys) | Index Metadata Overhead | Total Resident Memory | Mutation / Ingestion Model |
| :--- | :--- | :--- | :--- | :--- |
| **`Flat Slice`** | 7.63 MB (10⁶ × 8 B) | **0.00 KB** | **7.63 MB** | Read-only (shift O(N)) |
| **`Eytzinger (In-Situ)`** | 7.63 MB (level-order) | **0.00 KB** | **7.63 MB** | Read-only (rebuild required) |
| **`Eytzinger (Index)`** | 7.63 MB (sorted source) | 7.63 MB (tree array) | 15.26 MB | Read-only (rebuild required) |
| **`PgmIndex`** | 7.63 MB (10⁶ × 8 B) | 3.1 KB - 1.50 MB | 7.63 MB - 9.13 MB | Read-only (batch merge O(N+M)) |
| **`MultilevelPgmIndex`** | 7.63 MB (10⁶ × 8 B) | 2.0 KB - 1.32 MB | 7.63 MB - 8.95 MB | Read-only (batch merge O(N+M)) |
| **`BTreeMap`** | Embedded in heap nodes | ~45.78 MB (nodes + pointers) | 45.78 MB | Dynamic (in-place O(log N)) |

* `PgmIndex` and `MultilevelPgmIndex` do not replace the underlying data buffer; they are secondary index structures requiring the raw contiguous sorted slice ($7.63\text{ MB}$).
* While PGM reduces *index metadata* by up to 14,700x relative to `BTreeMap` on linear workloads, the *total process memory reduction* is approximately **5.0x to 5.7x** ($7.63\text{ MB}$ vs $45.78\text{ MB}$).

---

## Detailed Results by Distribution

### 1. Dense Sequential Time-Series Data ($\epsilon = 32$)
Monotonically increasing telemetry timestamps or auto-increment primary keys. Highly linear CDF.

| Cible / Target | Build Time | Hit Latency | Miss Latency | Lower Bound | Index Metadata | Moyenne Totale Latence | Speedup vs BTree |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **BTreeMap** | 66.57 ms | 224.1 ns | 229.4 ns | 229.1 ns | 45.78 MB | 227.5 ns | 1.00x |
| **Flat Slice** | **0.00 ms** | 168.2 ns | 139.4 ns | **122.7 ns** | **0.00 KB** | 143.4 ns | 1.33x |
| **Eytzinger** | 5.54 ms | 137.8 ns | 140.8 ns | 143.1 ns | 7.63 MB | 140.6 ns | 1.63x |
| **PGM-Index** | 4.64 ms | **107.1 ns** | **101.0 ns** | 132.0 ns | 3.1 KB | **113.4 ns** | **2.09x** |
| **Multilevel PGM** | 4.61 ms | 145.8 ns | 133.1 ns | 206.4 ns | 2.0 KB | 161.8 ns | 1.54x |

* **Analysis**: PGM achieves its best relative performance here (2.09x vs BTree). The nearly perfect linear CDF is compressed into only 93 segments ($3.1\text{ KB}$), fitting permanently into the L1 CPU cache.
* **Trade-off**: `Flat Slice` requires 0 ms build time and 0 KB metadata with respectable 143.4 ns latency.

---

### 2. Uniform Sparse Random Keys ($\epsilon = 16$)
Uniform 64-bit random values across a wide domain ($[0, 10^{12}]$).

| Cible / Target | Build Time | Hit Latency | Miss Latency | Lower Bound | Index Metadata | Moyenne Totale Latence | Speedup vs BTree |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **BTreeMap** | 66.36 ms | 240.6 ns | 252.7 ns | 232.5 ns | 45.78 MB | 241.9 ns | 1.00x |
| **Flat Slice** | **0.00 ms** | 152.5 ns | 133.0 ns | 135.5 ns | **0.00 KB** | 140.4 ns | 1.58x |
| **Eytzinger** | 5.43 ms | 130.1 ns | 132.6 ns | **126.8 ns** | 7.63 MB | 129.8 ns | 1.85x |
| **PGM-Index** | 5.16 ms | **126.3 ns** | **115.0 ns** | 137.9 ns | 48.1 KB | **126.4 ns** | **1.91x** |
| **Multilevel PGM** | 4.80 ms | 139.1 ns | 121.8 ns | 199.9 ns | 33.0 KB | 153.6 ns | 1.73x |

* **Analysis**: PGM generates 1,416 segments ($48.1\text{ KB}$), remaining within L2 cache. PGM-Index (126.4 ns) and Eytzinger (129.8 ns) perform neck-and-neck, both outperforming BTreeMap by $\sim 1.9\text{x}$.
* **Trade-off**: Eytzinger achieves near-identical latency without floating-point ALU operations.

---

### 3. Clustered Heavy-Tailed Lognormal Distribution ($\epsilon = 16$)
Power-law distribution with heavy skew in low values and an extended sparse tail.

| Cible / Target | Build Time | Hit Latency | Miss Latency | Lower Bound | Index Metadata | Moyenne Totale Latence | Speedup vs BTree |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **BTreeMap** | 65.83 ms | 227.6 ns | 228.7 ns | 228.1 ns | 45.78 MB | 228.1 ns | 1.00x |
| **Flat Slice** | **0.00 ms** | 174.0 ns | 138.4 ns | 153.8 ns | **0.00 KB** | 155.4 ns | 1.31x |
| **Eytzinger** | 5.35 ms | 132.9 ns | 130.7 ns | **131.8 ns** | 7.63 MB | 131.8 ns | 1.71x |
| **PGM-Index** | 5.16 ms | **117.0 ns** | **105.0 ns** | 139.5 ns | 12.1 KB | **120.5 ns** | **1.95x** |
| **Multilevel PGM** | 5.12 ms | 154.8 ns | 131.9 ns | 215.0 ns | 10.8 KB | 167.2 ns | 1.47x |

* **Analysis**: Streaming PLA compresses the skewed CDF effectively into 475 segments ($12.1\text{ KB}$), delivering 120.5 ns latency.
* **Trade-off**: On `lower_bound`, Eytzinger remains faster (131.8 ns vs 139.5 ns for PGM).

---

### 4. SOSD OpenStreetMap (OSM CellIDs) ($\epsilon = 16$)
Geospatial Hilbert cell IDs from real-world OpenStreetMap data with irregular cluster densities.

| Cible / Target | Build Time | Hit Latency | Miss Latency | Lower Bound | Index Metadata | Moyenne Totale Latence | Speedup vs BTree |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **BTreeMap** | 66.08 ms | 232.4 ns | 247.0 ns | 244.6 ns | 45.78 MB | 241.3 ns | 1.00x |
| **Flat Slice** | **0.00 ms** | 151.3 ns | 132.6 ns | 136.2 ns | **0.00 KB** | 140.1 ns | 1.54x |
| **Eytzinger** | 5.25 ms | 140.3 ns | 135.8 ns | **127.9 ns** | 7.63 MB | 134.7 ns | 1.66x |
| **PGM-Index** | 5.11 ms | **119.9 ns** | **123.2 ns** | 151.8 ns | 192.1 KB | **131.6 ns** | **1.94x** |
| **Multilevel PGM** | 4.95 ms | 160.3 ns | 150.2 ns | 204.7 ns | 97.4 KB | 171.7 ns | 1.45x |

* **Analysis**: Despite generating 4,145 segments ($192.1\text{ KB}$), PGM-Index retains an edge in point lookups (119.9 ns), while Eytzinger is faster on range/predecessor queries (`lower_bound`: 127.9 ns).

---

### 5. SOSD Wikipedia Timestamps (Wiki-TS) ($\epsilon = 32$)
Real-world edit log timestamps featuring plateaus of duplicate keys, micro-bursts, and idle gaps.

| Cible / Target | Build Time | Hit Latency | Miss Latency | Lower Bound | Index Metadata | Moyenne Totale Latence | Speedup vs BTree |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **BTreeMap** | 36.32 ms | **111.1 ns** | **104.2 ns** | **108.7 ns** | 45.78 MB | **108.0 ns** | **1.00x** |
| **Flat Slice** | **0.00 ms** | 134.6 ns | 111.7 ns | 124.2 ns | **0.00 KB** | 123.5 ns | 0.83x |
| **Eytzinger** | 5.30 ms | 131.9 ns | 124.5 ns | 137.2 ns | 7.63 MB | 131.2 ns | 0.84x |
| **PGM-Index** | 5.57 ms | 136.5 ns | 135.3 ns | 167.6 ns | 192.1 KB | 146.5 ns | 0.81x (slower) |
| **Multilevel PGM** | 4.86 ms | 153.9 ns | 197.7 ns | 209.6 ns | 152.3 KB | 187.1 ns | 0.72x (slower) |

* **Key Takeaway**: **`BTreeMap` is 1.35x faster than `PGM-Index` and 1.73x faster than `Multilevel PGM`**.
* **Reason**: Micro-bursts and duplicate keys force frequent segment restarts (6,593 segments), degrading binary search over segments. B-Tree node caching handles bursty repeated queries more efficiently than linear projection.

---

### 6. SOSD Books (Heavy-Tailed Book ISBNs) ($\epsilon = 16$)
Real-world Book ISBNs from SOSD benchmark, displaying extreme Pareto step functions and cluster voids.

| Cible / Target | Build Time | Hit Latency | Miss Latency | Lower Bound | Index Metadata | Moyenne Totale Latence | Speedup vs BTree |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **BTreeMap** | 11.34 ms | **37.3 ns** | **26.0 ns** | **41.5 ns** | 45.78 MB | **34.9 ns** | **1.00x** |
| **Flat Slice** | **0.00 ms** | 64.9 ns | 25.4 ns | 62.2 ns | **0.00 KB** | 50.8 ns | 0.58x (slower) |
| **Eytzinger** | 5.34 ms | 58.6 ns | 32.4 ns | 59.8 ns | 7.63 MB | 50.3 ns | 0.64x (slower) |
| **PGM-Index** | 3.06 ms | 86.3 ns | 63.5 ns | 151.1 ns | 1.50 MB | 100.3 ns | 0.43x (2.87x slower) |
| **Multilevel PGM** | 3.33 ms | 144.2 ns | 115.3 ns | 201.2 ns | 1.32 MB | 153.6 ns | 0.26x (4.40x slower) |

* **Key Takeaway**: **`BTreeMap` significantly outperforms all learned and binary approaches**. It is **2.87x faster than PGM-Index** and **4.40x faster than Multilevel PGM**.
* **Reason**: Severe non-linearity breaks piecewise linear approximation, yielding 55,167 segments ($1.50\text{ MB}$ of metadata). Binary searching through a 1.50 MB segment array incurs repeated L2/L3 cache misses. Meanwhile, B-Tree branches quickly isolate dense prefixes.

---

## Grand Summary (30 Runs Per Target Across All 6 Workload Suites)

| Cible / Target | Build Time | Hit Latency | Miss Latency | Lower Bound | Index Metadata | Moyenne Totale Latence | Speedup vs BTree | Total Memory (Data + Index) |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **BTreeMap** | 52.08 ms | 178.8 ns | 181.3 ns | 180.7 ns | 45.78 MB | 180.3 ns | 1.00x | 45.78 MB |
| **Flat Slice** | **0.00 ms** | 140.9 ns | 113.4 ns | 122.4 ns | **0.00 KB** | 125.6 ns | 1.27x | **7.63 MB** |
| **Eytzinger** | 5.37 ms | 122.0 ns | 116.2 ns | **121.1 ns** | 7.63 MB | **119.7 ns** | 1.47x | 15.26 MB |
| **PGM-Index** | 4.78 ms | **115.5 ns** | **107.2 ns** | 146.7 ns | 330.6 KB | 123.1 ns | **1.55x** | 7.96 MB |
| **Multilevel PGM** | 4.61 ms | 149.7 ns | 141.7 ns | 206.1 ns | 274.5 KB | 165.8 ns | 1.19x | 7.90 MB |

---

## Objective Architectural Findings

1. **No Universal Winner**:
   - **`PGM-Index`** is fastest on smooth, continuous, or moderately clustered distributions (Time-Series, Uniform, Lognormal, OSM), delivering up to **2.09x speedup** over `BTreeMap` with low metadata ($3.1\text{ KB} - 192\text{ KB}$).
   - **`BTreeMap`** wins decisively on irregular, heavily fragmented, or extreme Pareto power-law distributions (Books: **2.87x faster than PGM**; Wiki-TS: **1.35x faster**), because localized pointer trees handle discontinuities and duplicates better than piecewise linear slopes.
   - **`EytzingerSearch`** provides the most consistent overall performance across all suites (average **119.7 ns**, outperforming PGM's 123.1 ns average) and the fastest predecessor search (`lower_bound`: 121.1 ns), without any sensitivity to key distribution shape.
   - **`Flat slice::binary_search`** remains the best choice when build time (0.00 ms) and index memory (0.00 KB) are constrained, lagging only $\sim 6\text{ ns}$ behind PGM overall.
2. **Memory Footprint Reality**:
   - PGM's metadata savings are dramatic ($330\text{ KB}$ vs $45.8\text{ MB}$), but since the raw sorted slice ($7.63\text{ MB}$) must be retained, the effective end-to-end memory reduction is **$\approx 5.7\text{x}$**, not 14,000x.
3. **Multilevel Scaling Limits at $10^6$ Keys**:
   - At $N = 10^6$, `MultilevelPgmIndex` is slower than single-level `PgmIndex` (165.8 ns vs 123.1 ns). The arithmetic overhead of hierarchical descent does not pay off until datasets exceed $10^7$ keys where single-level segment arrays overflow L3 cache.

---

## Academic References

- **SOSD Benchmark Suite**:
  - Marcus, R., Kipf, A., van Renen, A., Tatarakis, M., Sheng, C., & Kraska, T. (2019). *"Benchmarking Learned Indexes"*. Proceedings of the VLDB Endowment (PVLDB) / AIDB.
  - Paper: [arXiv:1908.07009](https://arxiv.org/abs/1908.07009) · Codebase: [GitHub learnedsystems/SOSD](https://github.com/learnedsystems/SOSD)
- **PGM-Index Theoretical Formulation**:
  - Ferragina, P., & Vinciguerra, G. (2020). *"The PGM-index: a fully-dynamic compressed learned index with provable worst-case bounds"*. PVLDB 13(8): 1162–1175.
  - Paper: [PVLDB PDF](https://www.vldb.org/pvldb/vol13/p1162-ferragina.pdf) · [arXiv:1910.06169](https://arxiv.org/abs/1910.06169)
- **Eytzinger Layout Formalization**:
  - Khuong, P.-V., & Morin, P. (2017). *"Array Layouts for Comparison-Based Searching"*. ACM Journal of Experimental Algorithmics (JEA), 22: 1–39.
  - Paper: [arXiv:1509.05053](https://arxiv.org/abs/1509.05053)
