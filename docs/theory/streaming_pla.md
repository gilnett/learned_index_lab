# Theory: Optimal Streaming Piecewise Linear Approximation (PLA)

## Summary Extract

The Piecewise Geometric Model (PGM-Index) replaces static tree nodes with a minimal set of linear segments $y = ax + b$, guaranteeing that the predicted position of any key $k$ falls within a strict corridor $[y - \epsilon, y + \epsilon]$.

The index builder executes in a single streaming pass ($O(N)$ time, $O(1)$ additional working space):

1. **Slope Corridor**: Starting from origin point $(x_0, y_0)$, each subsequent key $(x_i, y_i)$ restricts the feasible slope range $[s_{\min}, s_{\max}]$:
   $$s_{\min} = \max_{j} \left[ \frac{(y_j - \epsilon) - y_0}{x_j - x_0} \right], \quad s_{\max} = \min_{j} \left[ \frac{(y_j + \epsilon) - y_0}{x_j - x_0} \right]$$
2. **Segment Finalization**: As long as $s_{\min} \le s_{\max}$, the corridor remains feasible. The moment $s_{\min} > s_{\max}$, the current segment is finalized with median slope $s = \frac{s_{\min} + s_{\max}}{2}$, and a new segment begins.

---

## Full Research Papers & Resources

- **Foundational Paper**: Ferragina, P., & Vinciguerra, G. (2020). *"The PGM-index: a fully-dynamic compressed learned index with provable worst-case bounds"*, Proceedings of the VLDB Endowment (PVLDB), 13(8): 1162–1175.
  - [PVLDB Official Publication (PDF)](https://www.vldb.org/pvldb/vol13/p1162-ferragina.pdf)
  - [arXiv:1910.06169 (Preprint)](https://arxiv.org/abs/1910.06169)
- **Official PGM Project Site**: [https://pgm.di.unipi.it/](https://pgm.di.unipi.it/)
- **C++ Reference Implementation**: [GitHub: gvinciguerra/PGM-index](https://github.com/gvinciguerra/PGM-index)
