# Theory: Cache-Conscious Eytzinger (Breadth-First) Layout

## Summary Extract

Standard binary search over a contiguous sorted array jumps across disparate memory offsets (e.g. $N/2$, $N/4$, $N/8$), incurring recurring L1/L2/L3 cache misses and branch mispredictions.

The **Eytzinger layout** (also known as breadth-first or level-order array layout) reorganizes pre-sorted keys into a complete binary tree stored in an array indexed from 1:
- Root is at index `1`.
- Node `k` has left child at `2k` and right child at `2k + 1`.

### Microarchitectural Advantages
1. **Cache Locality**: The top levels of the search tree (which are accessed on every single query) reside permanently in CPU L1/L2 cache lines.
2. **Branchless Traversal**: Traversal uses arithmetic index advancement:
   $$k = 2k + (\text{target} > \text{tree}[k])$$
   This compiles into conditional move instructions (`cmov`), completely eliminating branch misprediction stalls.

---

## Full Research Papers & Resources

- **Foundational Paper**: Khuong, P.-V., & Morin, P. (2017). *"Array Layouts for Comparison-Based Searching"*, ACM Journal of Experimental Algorithmics (JEA), 22(1): 1–39.
  - [arXiv:1509.05053 (Full PDF)](https://arxiv.org/abs/1509.05053)
- **Algorithmica Tutorial**: [Eytzinger Binary Search Guide](https://en.algorithmica.org/hpc/data-structures/binary-search/)
