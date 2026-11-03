# Hierarchical Multilevel PGM-Index

## Theoretical Foundation

In a single-level `PgmIndex`, locating the responsible segment for a query key requires a binary search across the $S$ segment headers, taking $O(\log S)$ comparisons. While $S \ll N$ on smooth distributions, the segment array can grow large when dataset size exceeds tens of millions ($N > 10^7$).

The Hierarchical Multilevel PGM-Index (Ferragina & Vinciguerra, PVLDB 2020) eliminates flat binary search across segments by recursively constructing PGM index layers over the start keys of lower levels:

1. **Level 0 (Base Layer)**: Directly indexes the $N$ raw sorted data keys using base error tolerance ε₀, producing $S_0$ linear segments.
2. **Recursive Upper Levels**: For level $i \ge 1$, an upper PGM index is constructed over the $S_{i-1}$ segment start keys of level $i-1$ using recursive tolerance ε_rec.
3. **Root Level**: Construction terminates when the top level contains $\le 2 \cdot \varepsilon_{\text{rec}}$ segments, which fit in a single CPU cache line.

---

## Complexity & Deterministic Guarantees

* **Tree Height**: $h \approx \lceil \log_{2\varepsilon_{\text{rec}}} S_0 \rceil$.
* **Query Time**: $O(h \cdot \log \varepsilon_{\text{rec}} + \log \varepsilon_0)$, replacing global binary search over segments with top-down arithmetic projections.
* **Deterministic Bounds**: Every descent step stays within `[pos - ε_rec, pos + ε_rec]`, ensuring intermediate lookups remain tightly bounded.

---

## Practical Scale Considerations: When Multilevel Wins vs. Lags

Empirical benchmarks highlight critical trade-offs between single-level and multilevel PGM structures:

| Dataset Scale | Single-Level `PgmIndex` | `MultilevelPgmIndex` | Architectural Recommendation |
| :--- | :--- | :--- | :--- |
| **Small / Moderate (N ≤ 10⁶)** | **Faster (~123 ns avg)** | Slower (~165 ns avg) | **Single-Level**: S₀ fits in L2/L3 cache (< 330 KB). Flat binary search is faster than multi-hop arithmetic descent. |
| **Massive (N > 10⁷)** | Slower (segment cache thrashing) | **Faster (predictable cache residency)** | **Multilevel**: S₀ overflows L3 cache (> 10 MB); hierarchical descent prevents large stride memory jumps. |
| **Severe Pareto Skew (SOSD Books)** | Slower (100 ns) | Slower (153 ns) | **`BTreeMap` (35 ns)**: Tree node caching outperforms learned arithmetic when segments explode. |

---

## Academic Reference

* **Paolo Ferragina & Giorgio Vinciguerra (2020)**: *"The PGM-index: a fully-dynamic compressed learned index with provable worst-case bounds"*, Proceedings of the VLDB Endowment (PVLDB), Vol. 13, No. 8, pp. 1162–1175.
* DOI: [10.14778/3389133.3389135](https://doi.org/10.14778/3389133.3389135) · [PDF on arXiv](https://arxiv.org/pdf/1910.06169)
