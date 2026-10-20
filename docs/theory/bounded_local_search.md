# Theory: Bounded Local Search & Error Corridors

## Summary Extract

In a Piecewise Geometric Model (PGM-Index), lookup combines global mathematical interpolation with strictly bounded local binary search:

1. **Global Segment Search ($O(\log S)$)**:
   Given a target key $k$, find the responsible segment by binary search over segment origin keys $k_0$. Because $S \ll N$, segment headers fit entirely within L1/L2 cache.
2. **ALU Arithmetic Prediction**:
   The position estimate is calculated via in-register ALU arithmetic:
   $$\text{Pos}_{\text{approx}} = \text{round}(y_0 + s \cdot (k - x_0))$$
3. **Bounded Local Verification**:
   The exact key position is guaranteed to reside in the bounded window:
   $$[\max(0, \text{Pos}_{\text{approx}} - \epsilon), \min(N - 1, \text{Pos}_{\text{approx}} + \epsilon)]$$
   A local binary search over this window ($\le 2\epsilon + 1$ elements) takes $O(\log \epsilon)$ time and hits at most 1–4 contiguous cache lines.

### Guarantees
- **Lookup Complexity**: $O(\log S + \log \epsilon)$
- **Exactness**: 100% exact (zero false positives, zero false negatives).

---

## Full Research Papers & Resources

- **Foundational Paper**: Ferragina, P., & Vinciguerra, G. (2020). *"The PGM-index: a fully-dynamic compressed learned index with provable worst-case bounds"*, PVLDB 13(8): 1162–1175.
  - [PVLDB Publication (PDF)](https://www.vldb.org/pvldb/vol13/p1162-ferragina.pdf)
  - [Direct arXiv PDF Download](https://arxiv.org/pdf/1910.06169)
  - [arXiv:1910.06169 (Preprint Abstract)](https://arxiv.org/abs/1910.06169)
