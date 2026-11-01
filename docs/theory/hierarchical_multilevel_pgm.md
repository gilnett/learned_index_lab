# Hierarchical Multilevel PGM-Index

## Theoretical Foundation

In a standard single-level `PgmIndex`, identifying the segment responsible for a query key requires binary search across $S$ segments, incurring $O(\log S)$ comparisons. While $S \ll N$, for massive datasets ($N > 10^7$) or highly irregular key distributions, $S$ can grow large enough to exceed L1/L2 cache capacities.

The Hierarchical Multilevel PGM-Index (Ferragina & Vinciguerra, PVLDB 2020) solves this by recursively building PGM levels over the start keys of the level below:

1. **Level 0 (Base Level)**: Directly indexes the $N$ raw sorted data keys using error tolerance $\epsilon_0$, yielding $S_0$ linear segments.
2. **Recursive Upper Levels**: For level $i \ge 1$, an upper PGM index is built over the $S_{i-1}$ keys of level $i-1$ using recursive tolerance $\epsilon_{\text{rec}}$.
3. **Root Level**: Construction terminates when the top level contains $\le 2 \cdot \epsilon_{\text{rec}}$ segments, which fit in a single CPU cache line.

## Complexity & Guarantees

* **Tree Height**: $h \approx \log_{2\epsilon_{\text{rec}}} S_0$.
* **Query Time**: $O(h \cdot \log \epsilon_{\text{rec}} + \log \epsilon_0)$, completely replacing the global binary search over segments.
* **Deterministic Bounds**: Every descent step stays within $[\text{pos} - \epsilon_{\text{rec}}, \text{pos} + \epsilon_{\text{rec}}]$, ensuring all intermediate index lookups remain cache-resident.

## Academic Reference

* **Paolo Ferragina & Giorgio Vinciguerra (2020)**: *"The PGM-index: a fully-dynamic compressed learned index with provable worst-case bounds"*, Proceedings of the VLDB Endowment (PVLDB), Vol. 13, No. 8, pp. 1162–1175.
* DOI: [10.14778/3389133.3389135](https://doi.org/10.14778/3389133.3389135) · [PDF on arXiv](https://arxiv.org/pdf/1910.06169)

