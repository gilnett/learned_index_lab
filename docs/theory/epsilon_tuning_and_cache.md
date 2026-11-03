# Epsilon ($\epsilon$) Tuning & Hardware Cache Hierarchy Guide

## Overview

In the Piecewise Geometric Model (PGM-Index), the parameter **$\epsilon$** (epsilon) is the single most critical configuration knob. It defines the **maximum absolute error bound** guaranteed by the streaming Piecewise Linear Approximation (PLA) algorithm:

$$|\text{Pos}_{\text{actual}}(k) - \text{Pos}_{\text{predicted}}(k)| \le \epsilon$$

Understanding how $\epsilon$ interacts with CPU cache line sizes (64 bytes), L1/L2 data cache capacities, and algorithmic search complexities is essential for achieving optimal lookup throughput and memory efficiency.

---

## 1. Microarchitectural Alignment: Cache Lines & Window Spans

During the third phase of a PGM lookup, the algorithm performs a local verification scan within the bounded interval:

$$[\text{Pos}_{\text{approx}} - \epsilon, \text{Pos}_{\text{approx}} + \epsilon]$$

This window contains at most **$2\epsilon + 1$ elements**. For 64-bit unsigned integer keys (`u64`, 8 bytes per key), contiguous memory layout directly determines how many 64-byte CPU cache lines are touched:

| $\epsilon$ | Window Elements ($2\epsilon + 1$) | Window Size (Bytes) | 64-Byte Cache Lines Touched | Cache Residency Profile |
| :---: | :---: | :---: | :---: | :--- |
| **4** | 9 keys | 72 B | ~1.1 lines | 100% L1-resident, but segments $S$ explode |
| **8** | 17 keys | 136 B | ~2.1 lines | Fits across 2–3 L1 cache lines |
| **16** | **33 keys** | **264 B** | **~4.1 lines** | **Sweet spot for uniform/skewed data (100% L1)** |
| **32** | **65 keys** | **520 B** | **~8.1 lines** | **Sweet spot for sequential/dense data (L1-resident)** |
| **64** | 129 keys | 1,032 B | ~16.1 lines | Exceeds 1 KB; prefetcher saturation risk |
| **128** | 257 keys | 2,056 B | ~32.1 lines | Local search starts incurring L2 cache stalls |
| **256** | 513 keys | 4,104 B | ~64.1 lines | Local binary search branch mispredictions dominate |

### Why $\epsilon \in [16, 32]$ is the Empirical Sweet Spot
1. **L1 Cache Line Density**: A 33-element window ($264\text{ bytes}$) spans roughly 4 contiguous cache lines. Modern CPU spatial prefetchers fetch adjacent cache lines upon the first memory request.
2. **Branch Misprediction Minimization**: A local binary search over 33 elements requires at most $\lceil \log_2(33) \rceil = 6$ branch comparisons, keeping pipeline flush penalties low.
3. **Metadata Control**: At $\epsilon = 16$ or $32$, the number of generated segments $S$ is compressed by orders of magnitude relative to $N$, allowing the entire segment table to fit into L1/L2 cache lines.

---

## 2. The Fundamental Pareto Trade-Off: Segment Count ($S$) vs. Window Size ($\epsilon$)

The total lookup latency of a single-level PGM-Index is governed by the sum of two distinct search phases:

$$T_{\text{lookup}} = \underbrace{T_{\text{segment}}(S)}_{O(\log S)} + \underbrace{T_{\text{ALU}}}_{O(1)} + \underbrace{T_{\text{local}}(\epsilon)}_{O(\log \epsilon)}$$

* **Increasing $\epsilon$**:
  * Widens the slope corridor $[s_{\min}, s_{\max}]$, allowing each segment to cover a wider span of keys.
  * Decreases total segment count $S \propto \frac{1}{\epsilon}$.
  * Reduces index metadata memory footprint.
  * Speeds up Step 1 (fewer segments to search).
  * **Downside**: Widens the local verification window, increasing Step 3 latency.
* **Decreasing $\epsilon$**:
  * Tightens the prediction corridor, narrowing the local search window ($2\epsilon + 1$).
  * Speeds up Step 3 (fewer elements to scan).
  * **Downside**: Causes segment fragmentation ($S$ increases). If $S$ grows from hundreds to tens of thousands (as seen in heavy Pareto distributions), the global binary search over segments spills out of L2 cache into L3/DRAM, degrading overall performance.

### Trade-off Regimes Across the $\epsilon$ Spectrum

| Regime | Segment Count ($S$) | Step 1 (Segment Lookup) | Step 3 (Local Corridor Scan) | Overall Latency Result |
| :--- | :--- | :--- | :--- | :--- |
| **Undersized ($\epsilon \le 4$)** | Explodes ($S \gg 10^4$, spills out of L2/L3) | **Bottleneck**: slow binary search over large segment table | Fast ($< 9$ keys, 1 cache line) | **High**: segment lookup dominates |
| **Sweet Spot ($\epsilon = 16\text{–}32$)** | Compact ($S \approx 100\text{–}1,500$, L1/L2-resident) | Fast ($O(\log S)$ in cache) | Fast ($33\text{–}65$ keys, 4–8 L1 lines) | **Optimal**: balanced Step 1 and Step 3 |
| **Oversized ($\epsilon \ge 128$)** | Minimal ($S < 50$, $< 2\text{ KB}$) | Instant ($O(\log S) \le 5$ checks) | **Bottleneck**: wide scan ($> 257$ keys, $> 32$ cache lines) | **High**: degrades toward raw slice search |

---

## 3. Tuning $\epsilon$ in Hierarchical Indexing (`MultilevelPgmIndex`)

In `MultilevelPgmIndex`, the base layer and upper recursive layers have distinct memory profiles and can be parameterized with independent epsilons:

```rust
let ml_pgm = MultilevelPgmIndex::build_with_epsilons(
    data, 
    epsilon_base,       // eps_0 (e.g. 16 or 32)
    epsilon_recursive   // eps_rec (e.g. 8 or 16)
);
```

### Decoupling $\epsilon_0$ and $\epsilon_{\text{rec}}$
1. **Base Epsilon ($\epsilon_0$)**:
   - Operates on the raw data buffer ($N$ keys).
   - Controls the trade-off between total raw segment count $S_0$ and the final local data slice scan.
2. **Recursive Epsilon ($\epsilon_{\text{rec}}$)**:
   - Operates on segment start keys ($S_0, S_1, \dots$).
   - Determines the fan-out and tree height $h \approx \lceil \log_{2\epsilon_{\text{rec}}} S_0 \rceil$.
   - Because upper level keys are purely segment boundaries already resident in CPU cache, keeping $\epsilon_{\text{rec}} = 16$ guarantees that each descent step accesses at most 4 cache lines while reducing the number of segments by a factor of $\approx 2\epsilon_{\text{rec}} = 32\times$ per level.

---

## 4. Practical Workload Tuning Matrix

| Workload Profile | Recommended $\epsilon$ | Rationale |
| :--- | :---: | :--- |
| **Dense Sequential / Monotonic Logs** | **$\epsilon = 32$ or $64$** | High linearity allows extreme compression (e.g. 93 segments for 1M keys). Local scan over 65 elements remains 100% L1-resident. |
| **Uniform Sparse Random Keys** | **$\epsilon = 16$** | Balances moderate slope variance against local verification speed, yielding $\sim 1,400$ segments ($48\text{ KB}$). |
| **Clustered / Skewed (Lognormal, Geospatial OSM)** | **$\epsilon = 16$** | Restricts local verification latency in dense cluster pockets while preventing excessive segment fragmentation in sparse tails. |
| **Micro-Bursts & Duplicate Heavy (Wiki-TS)** | **$\epsilon = 32$** | Prevents identical key plateaus from constantly triggering corridor resets and segment splits. |
| **Extreme Pareto Skew (Book ISBNs)** | **Classical B-Tree** | Non-linear step functions produce excessive segments ($\sim 55,000$ segments / 1.5 MB). `BTreeMap` is 2.87x faster than PGM under this regime. |

---

## 5. Academic Foundations & Theoretical Proofs

* **Optimal Segment Cardinality**:
  Ferragina & Vinciguerra (PVLDB 2020) proved that greedy streaming PLA produces the minimum possible number of linear segments covering $N$ sorted points for any fixed $\epsilon$ in $O(N)$ time and $O(1)$ auxiliary space.
* **Worst-Case Lookup Bounds**:
  Lookup in a single-level PGM-Index is strictly bounded by $O(\log S + \log \epsilon)$, with zero heuristic fallbacks or unbounded scan penalties.

### References
- Ferragina, P., & Vinciguerra, G. (2020). *"The PGM-index: a fully-dynamic compressed learned index with provable worst-case bounds"*, PVLDB, 13(8): 1162–1175.
- Marcus, R., et al. (2019). *"Benchmarking Learned Indexes"*, Proceedings of the VLDB Endowment / AIDB 2019.
