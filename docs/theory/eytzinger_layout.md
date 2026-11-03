# Theory: Cache-Conscious Eytzinger (Breadth-First) Layout

## Summary Extract

Standard binary search over a contiguous sorted array jumps across disparate memory offsets (e.g. `N/2`, `N/4`, `N/8`), incurring recurring L1/L2/L3 cache misses and branch mispredictions.

The **Eytzinger layout** (also known as breadth-first or level-order array layout) reorganizes pre-sorted keys into a complete binary tree stored in an array indexed from 1:
- Root is at index `1`.
- Node `k` has left child at `2k` and right child at `2k + 1`.

### Microarchitectural Advantages
1. **Cache Locality**: The top levels of the search tree (which are accessed on every single query) reside permanently in CPU L1/L2 cache lines.
2. **Branchless Traversal**: Traversal uses arithmetic index advancement:
   $$k = 2k + (\text{target} > \text{tree}[k])$$
   This compiles into conditional move instructions (`cmov`), completely eliminating branch misprediction stalls.

---

## 1. Operational Modes: In-Situ Storage vs. Secondary Index

An essential design question when evaluating Eytzinger layouts is whether the structure operates as **primary data storage** or as a **secondary index**:

| Dimension | In-Situ Mode (`from_sorted_vec`) | Secondary Index Mode (`build`) |
|:---|:---|:---|
| **Primary Data Buffer** | Reordered directly into level order | Preserved in contiguous sorted order |
| **Auxiliary Index Memory** | **0.00 KB** (the array *is* the storage) | 7.63 MB (auxiliary tree array) |
| **Total Resident Memory (1M keys)** | **7.63 MB** (identical to flat slice) | 15.26 MB (2× data size) |
| **Point Lookups (`find`, `lower_bound`)** | **~119 ns (L1 pinned, branchless)** | **~119 ns (L1 pinned, branchless)** |
| **Contiguous Range Queries (`&[u64]`)** | Non-contiguous (requires tree traversal) | **Instant O(1) contiguous slice return** |

### Trade-Off Analysis
* **In-Situ Primary Storage**: In workloads focused strictly on point lookups (`find`, membership checks, or single-key predecessor queries), permuting the input array directly provides the lowest overall latency in the benchmark suite (~119 ns) with zero metadata overhead.
* **Secondary Index**: In storage engines requiring contiguous range scans (e.g. SSTable block iterators), preserving the original linear array while building an auxiliary Eytzinger tree allows O(1) zero-copy slice slicing (`&data[start..end]`) at the expense of doubling total resident RAM.

---

## 2. Layout Flavors: Binary vs. B-ary / SIMD Cache-Line Blocking

The literature on cache-conscious array layouts (Khuong & Morin, 2017) distinguishes two primary flavors:

1. **Binary Eytzinger Layout (Implemented in this Lab)**:
   - 1-based complete binary tree: left child at `2k`, right child at `2k + 1`.
   - Traversal compiles to branchless conditional moves (`cmov`).
   - Final index resolution uses bit-twiddling: `res = k >> (k.trailing_ones() + 1)`.
2. **B-ary / SIMD-Vectorized Eytzinger**:
   - Instead of 2 children per node, each node packs `B - 1` keys into a single 64-byte cache line (e.g. `B = 8` or `B = 16` with AVX2 / AVX-512 SIMD comparisons).
   - Reduces tree height by a factor of log₂(B), at the cost of requiring explicit vector comparison instructions.

---

## Full Research Papers & Resources

* **Foundational Paper**: Khuong, P.-V., & Morin, P. (2017). *"Array Layouts for Comparison-Based Searching"*, ACM Journal of Experimental Algorithmics (JEA), 22(1): 1–39.
  * [Direct PDF Download (arXiv)](https://arxiv.org/pdf/1509.05053)
  * [arXiv:1509.05053 (Abstract & Citation)](https://arxiv.org/abs/1509.05053)
  * [ACM Digital Library](https://dl.acm.org/doi/10.1145/3053370)
* **Algorithmica Tutorial**: [Eytzinger Binary Search Guide](https://en.algorithmica.org/hpc/data-structures/binary-search/)
