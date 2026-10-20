# Theory: CDF Approximation in Learned Indexes

## Summary Extract

Traditional database indexes treat search keys as opaque values and navigate through pointer hierarchies ($B\text{-Tree}$, $B^+\text{-Tree}$, LSM block indexes), incurring $O(\log N)$ sequential pointer dereferences across memory.

**Learned Index Structures** reframe indexing as a regression problem:
Given a sorted sequence of keys $K = (k_0, k_1, \dots, k_{N-1})$, the memory position of any key $k$ is proportional to the Empirical Cumulative Distribution Function (CDF):

$$\text{Pos}(k) \approx N \times \text{CDF}(k)$$

By fitting continuous mathematical models to the CDF, keys are mapped directly to memory addresses via fast in-register ALU arithmetic instead of pointer chasing across cache lines.

---

## Full Research Papers & Resources

- **Seminal Paper**: Kraska, T., Beutel, A., Chi, E. H., Dean, J., & Polyzotis, N. (2018). *"The Case for Learned Index Structures"*, ACM SIGMOD.
  - [Direct PDF Download (arXiv)](https://arxiv.org/pdf/1712.01208)
  - [arXiv:1712.01208 (Abstract & Citation)](https://arxiv.org/abs/1712.01208)
- **MIT Data Systems Group**: [The Case for Learned Index Structures Overview](https://dsg.csail.mit.edu/projects/learnedindexes/)
