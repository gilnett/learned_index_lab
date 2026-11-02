# Theory: Optimal Streaming Piecewise Linear Approximation (PLA)

## Summary Extract

The Piecewise Geometric Model (PGM-Index) represents sorted keys using a minimal set of linear segments $y = ax + b$, guaranteeing that the predicted position of any key $k$ falls within a bounded error corridor $[y - \epsilon, y + \epsilon]$.

The index builder executes in a single sequential pass ($O(N)$ time, $O(1)$ auxiliary working space):

1. **Slope Corridor**:
   Starting from origin point $(x_0, y_0)$, each subsequent point $(x_i, y_i)$ (where $x_i$ is key value and $y_i = i$ is the array rank) narrows the feasible slope range $[s_{\min}, s_{\max}]$:
   $$s_{\min} = \max_{j} \left[ \frac{(y_j - \epsilon) - y_0}{x_j - x_0} \right], \quad s_{\max} = \min_{j} \left[ \frac{(y_j + \epsilon) - y_0}{x_j - x_0} \right]$$

2. **Segment Finalization**:
   As long as $s_{\min} \le s_{\max}$, the corridor remains feasible and point $(x_i, y_i)$ can be accommodated within the current segment. The moment $s_{\min} > s_{\max}$, the corridor collapses: the segment is finalized with median slope $s = \frac{s_{\min} + s_{\max}}{2}$, and a new segment begins anchored at $(x_i, y_i)$.

---

## Edge Cases & Practical Engineering Considerations

1. **Duplicate Keys ($dx = 0$)**:
   When identical keys appear sequentially ($x_i = x_0$), the denominator $x_i - x_0$ is zero, causing infinite or undefined slopes. If the run of duplicates exceeds the error tolerance $\epsilon$ ($|y_i - y_0| > \epsilon$), the corridor cannot encompass the vertical step. The builder must finalize the active segment and start a new one to prevent error corridor violations.

2. **Numerical Precision (IEEE-754 `f64`)**:
   Slopes and intercepts are stored as 64-bit floating-point numbers (`f64`). When key coordinates span large integer values ($u64$ near $2^{63}$), finite mantissa precision (53 bits) can introduce minor rounding errors during multiplication. In production implementations, error corridors are designed with conservative bounds to preserve the invariant under floating-point arithmetic.

3. **Optimality Proof**:
   Ferragina & Vinciguerra (2020) proved that this greedy streaming corridor algorithm produces the provably optimal (minimal) number of piecewise linear segments for a given $\epsilon$ in $O(N)$ time, without requiring multi-pass dynamic programming.

---

## Full Research Papers & Resources

- **Foundational Paper**: Ferragina, P., & Vinciguerra, G. (2020). *"The PGM-index: a fully-dynamic compressed learned index with provable worst-case bounds"*, Proceedings of the VLDB Endowment (PVLDB), 13(8): 1162–1175.
  - [PVLDB Official Publication (PDF)](https://www.vldb.org/pvldb/vol13/p1162-ferragina.pdf)
  - [Direct arXiv PDF Download](https://arxiv.org/pdf/1910.06169)
  - [arXiv:1910.06169 (Preprint Abstract)](https://arxiv.org/abs/1910.06169)
- **Official PGM Project Site**: [https://pgm.di.unipi.it/](https://pgm.di.unipi.it/)
- **C++ Reference Implementation**: [GitHub: gvinciguerra/PGM-index](https://github.com/gvinciguerra/PGM-index)
