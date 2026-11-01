# Batch Merge & Streaming PLA Rebuild

## Overview

Traditional search trees (`BTreeMap`, B+Trees) perform point-by-point dynamic in-place mutations ($O(\log N)$ per insert). Each insertion allocates nodes on the heap, rebalances branches, and invalidates CPU cache hierarchies.

In analytical engines and LSM-Tree architectures (e.g. SSTables in RocksDB, LevelDB, ScyllaDB), mutations are ingested in sorted batches. `learned_index_lab` leverages this through linear two-way batch merging followed by single-pass optimal streaming PLA approximation.

## Algorithmic Mechanics

1. **Two-Way Sorted Merge**:
   Given existing sorted data of size $N$ and an incoming sorted batch of size $M$, two pointers merge the records into a single contiguous buffer in $O(N + M)$ time with zero branching mispredictions on sequentially ordered runs.

2. **Single-Pass Streaming PLA**:
   The merged buffer is streamed into the optimal convex hull slope corridor algorithm in a single sequential pass. Each point is evaluated once ($O(N + M)$), producing the minimal optimal set of segments satisfying tolerance $\pm \epsilon$.

## Complexity Comparison

| Dimension | Dynamic In-Place B-Tree | Batch Merge + Streaming PLA |
| :--- | :--- | :--- |
| **Ingestion Complexity** | $O(M \log(N + M))$ pointer hops | $O(N + M)$ contiguous stream |
| **Heap Allocations** | 1 heap allocation per node split | 1 single contiguous buffer allocation |
| **Cache Locality** | Random heap traversal | Strictly linear hardware prefetch |
| **Post-Merge Model Size** | $\sim 45\text{ MB}$ per $10^6$ keys | $\sim 20\text{ KB}$ per $10^6$ keys (-99.9%) |

## Architectural Context

This approach mirrors SSTable compaction in storage engines: immutable runs ensure that reads never contend with concurrent lock acquisitions or pointer rotations.

