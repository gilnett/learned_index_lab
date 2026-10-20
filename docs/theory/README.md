# Theoretical Foundations Index

This directory provides concise extracts of each foundational theory behind `learned-index`, paired with direct links to the seminal research publications and full PDF papers:

| Theory / Topic | Summary Extract | Seminal Research & Full Paper |
| :--- | :--- | :--- |
| **CDF Approximation in Learned Indexes** | Models sorted keys as Empirical CDF regressions ($Pos(k) \approx N \times CDF(k)$) | [CDF Theory Extract](cdf_learned_index.md) · [Kraska et al. (SIGMOD 2018)](https://arxiv.org/abs/1712.01208) · [PDF](https://arxiv.org/pdf/1712.01208) |
| **Optimal Streaming PLA** | Single-pass $O(N)$ streaming slope corridor $[s_{\min}, s_{\max}]$ under bounded tolerance $\pm \epsilon$ | [Streaming PLA Extract](streaming_pla.md) · [Ferragina & Vinciguerra (PVLDB 2020)](https://www.vldb.org/pvldb/vol13/p1162-ferragina.pdf) · [PDF](https://arxiv.org/pdf/1910.06169) |
| **Eytzinger (Breadth-First) Layout** | Complete 1-based binary tree array layout with branchless `cmov` traversal | [Eytzinger Extract](eytzinger_layout.md) · [Khuong & Morin (ACM JEA 2017)](https://arxiv.org/abs/1509.05053) · [PDF](https://arxiv.org/pdf/1509.05053) |
| **Bounded Local Search** | Combines global segment binary search ($O(\log S)$) with local corridor scans ($O(\log \epsilon)$) | [Bounded Search Extract](bounded_local_search.md) · [Ferragina & Vinciguerra (2020)](https://arxiv.org/abs/1910.06169) · [PDF](https://arxiv.org/pdf/1910.06169) |
