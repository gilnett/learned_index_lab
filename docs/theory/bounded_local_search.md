# Theory: Bounded Local Search & Error Corridors

## Summary Extract

In a Piecewise Geometric Model (PGM-Index), lookup combines global mathematical interpolation with strictly bounded local search in a 3-step pipeline:

1. **Global Segment Resolution**:
   Given a target key $k$, the algorithm locates the responsible segment whose starting key satisfies $s.\text{key} \le k$. In single-level PGM, this is performed via binary search over the $S$ segment headers ($O(\log S)$). In multilevel PGM, it is resolved via recursive arithmetic descent ($O(h \cdot \log \epsilon_{\text{rec}})$).

2. **ALU Arithmetic Prediction**:
   Once the responsible segment $s = (k_0, \text{slope}, \text{intercept})$ is identified, the predicted ordinal offset is computed via scalar registers:
   $$\text{Pos}_{\text{approx}} = \text{round}(\text{intercept} + \text{slope} \times (k - k_0))$$

3. **Bounded Local Verification**:
   The optimal streaming PLA construction guarantees that if $k$ exists in the dataset, its true index resides strictly within:
   $$[\max(0, \text{Pos}_{\text{approx}} - \epsilon), \min(N - 1, \text{Pos}_{\text{approx}} + \epsilon)]$$
   A local search over this bounded window ($\le 2\epsilon + 1$ elements) takes $O(\log \epsilon)$ time and accesses at most 1–4 contiguous 64-byte cache lines.

---

## Microarchitectural Analysis of the Local Window

* **Cache Footprint**: For $\epsilon = 16$, the maximum search span is $2\epsilon + 1 = 33$ elements ($33 \times 8 = 264\text{ bytes}$). Because this slice is contiguous, fetching the first element causes the hardware prefetcher to pull the remaining 3 cache lines directly into L1 cache.
* **Local Search Strategies**:
  * **Binary Search (`slice::binary_search`)**: Minimizes comparison count ($\approx \lceil \log_2(33) \rceil = 6$ comparisons), but each step incurs a conditional branch that may trigger CPU branch mispredictions.
  * **Linear / SIMD Scan**: For very small $\epsilon$ ($\le 16$), unrolled linear comparisons or vector SIMD instructions (e.g. AVX2 / ARM NEON) can evaluate multiple keys simultaneously without branch misprediction stalls, representing an optimization path for high-throughput engines.

---

## Guarantees & Trade-offs

* **Complexity**: $O(\log S + \log \epsilon)$ for single-level PGM; $O(h \log \epsilon_{\text{rec}} + \log \epsilon_0)$ for multilevel PGM.
* **Exactness**: 100% exact (zero false positives, zero false negatives).
* **Sensitivity to Segment Count**: If $S$ is small (smooth CDF), Step 1 takes negligible time. If $S$ is large (severe skew or step functions), Step 1 dominates the entire query latency, eliminating the speedup over classical search trees.

---

## Full Research Papers & Resources

- **Foundational Paper**: Ferragina, P., & Vinciguerra, G. (2020). *"The PGM-index: a fully-dynamic compressed learned index with provable worst-case bounds"*, PVLDB 13(8): 1162–1175.
  - [PVLDB Publication (PDF)](https://www.vldb.org/pvldb/vol13/p1162-ferragina.pdf)
  - [Direct arXiv PDF Download](https://arxiv.org/pdf/1910.06169)
  - [arXiv:1910.06169 (Preprint Abstract)](https://arxiv.org/abs/1910.06169)
