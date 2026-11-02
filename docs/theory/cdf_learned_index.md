# Theory: CDF Approximation in Learned Indexes

## Summary Extract

Traditional database indexes treat search keys as opaque values and navigate through comparison-based pointer hierarchies ($B\text{-Tree}$, $B^+\text{-Tree}$, LSM block indexes), incurring $O(\log N)$ sequential pointer dereferences across memory.

**Learned Index Structures** reframe indexing as a regression problem:
Given a sorted sequence of keys $K = (k_0, k_1, \dots, k_{N-1})$, the ordinal rank and position of any key $k$ is proportional to the Empirical Cumulative Distribution Function (CDF):

$$\text{Pos}(k) \approx N \times \text{CDF}(k)$$

By approximating the CDF with functional models (such as piecewise linear segments), the search algorithm replaces pointer chasing with an in-register arithmetic calculation that narrows the target key's location down to a deterministic corridor $[\text{Pos}_{\text{approx}} - \epsilon, \text{Pos}_{\text{approx}} + \epsilon]$. Final retrieval is then performed via a fast local verification scan within the contiguous data buffer.

---

## Architectural Distinctions & Trade-Offs

* **Not Direct Addressing**: Unlike hash tables or perfect minimal hash functions, learned CDF approximations do not resolve keys to absolute memory addresses in a single step. They act as bounded spatial filters that drastically reduce the search space for a subsequent contiguous memory lookup.
* **Continuous vs. Discontinuous CDFs**:
  * For smooth, monotonically continuous distributions (e.g. sensor timestamps, sequential logs, uniform random sequences), linear CDF approximation compresses millions of keys into a few dozen or hundred parameters.
  * For discontinuous or heavy-tailed distributions with steep plateaus and massive gaps (e.g. power-law ISBNs, repeated transaction IDs), empirical CDF approximation requires high segment densities, diminishing the speed advantage over classical tree structures.

---

## Full Research Papers & Resources

- **Seminal Paper**: Kraska, T., Beutel, A., Chi, E. H., Dean, J., & Polyzotis, N. (2018). *"The Case for Learned Index Structures"*, ACM SIGMOD.
  - [Direct PDF Download (arXiv)](https://arxiv.org/pdf/1712.01208)
  - [arXiv:1712.01208 (Abstract & Citation)](https://arxiv.org/abs/1712.01208)
- **MIT Data Systems Group**: [The Case for Learned Index Structures Overview](https://dsg.csail.mit.edu/projects/learnedindexes/)
