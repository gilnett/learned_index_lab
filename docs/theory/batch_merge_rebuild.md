# Batch Merge & Streaming PLA Rebuild

## Overview

A fundamental trade-off exists between dynamic in-place mutation structures (like `std::collections::BTreeMap` or B+Trees) and immutable learned structures relying on batch rebuilds.

* **Dynamic Trees (`BTreeMap`)**: Support granular, low-latency point insertions ($O(\log N)$ per key). Keys are inserted into multi-key nodes without rewriting the rest of the dataset.
* **Learned Structures (`PgmIndex`)**: Are strictly immutable once built. Updates require a two-way merge of the existing sorted data with the new batch, followed by a full single-pass streaming PLA reconstruction ($O(N + M)$).

This design mirrors the immutable SSTable compaction architecture used in LSM-tree storage engines (e.g. RocksDB, LevelDB, ScyllaDB), where writes are buffered in memory and flushed periodically in large sequential runs.

---

## Algorithmic Mechanics

1. **Two-Way Sorted Merge ($O(N + M)$)**:
   Given existing sorted data of size $N$ and an incoming sorted batch of size $M$, two pointers merge both inputs into a single contiguous destination buffer. Sequential traversal allows the CPU prefetcher to operate at full memory bus bandwidth.

2. **Single-Pass Streaming PLA ($O(N + M)$)**:
   The merged buffer is streamed into the optimal convex hull slope corridor algorithm. Each element is evaluated once, producing the minimal optimal set of segments satisfying error tolerance $\pm \epsilon$.

---

## Engineering Trade-Offs: When Batch Merge Wins vs. When B-Tree Wins

| Workload Dimension | Dynamic In-Place B-Tree (`BTreeMap`) | Batch Merge + Streaming PLA (`PgmIndex`) |
| :--- | :--- | :--- |
| **Point Updates ($M = 1$ key)** | **$O(\log N)$ ($\approx 20\text{–}25$ ops)** | $OO(N) full buffer rewrite (massive write amplification) |
| **Small Batches ($M \ll N$)** | **$O(M \log N)$ (localized node edits)** | $O(N + M)$ (rewrites all $N$ elements) |
| **Large Compactions ($M \approx N$)** | $OO(M log(N + M)) pointer hops | **$O(N + M)$ strictly contiguous stream** |
| **Transient Memory During Write** | **Zero extra buffer (node-by-node allocation)** | **$2 \times (N + M)$ double-buffering during merge** |
| **Post-Merge Total Memory (1M keys)** | ~45.78 MB (nodes + data embedded) | **~7.65 MB – 9.13 MB** (7.63 MB data + index metadata) |
| **Concurrent Read Interference** | Requires locks / latching or tree cow | **Lock-free immutable snapshot reads** |

---

## Key Architectural Takeaways

1. **Write Amplification on Small Batches**:
   If an application requires frequent single-key insertions or random updates, **`BTreeMap` is overwhelmingly superior**. Rebuilding a PGM index for small $M$ wastes millions of CPU cycles re-approximating unchanged regions.

2. **Transient Double-Buffering Overhead**:
   Merging a dataset of size $N$ with a batch $M$ requires allocating a contiguous buffer of size $(N + M) \times 8\text{ bytes}$. During the merge, both the old buffer and the new buffer reside simultaneously in memory. In memory-constrained environments, this transient peak can cause out-of-memory errors unless disk spillover is implemented.

3. **Sweet Spot**:
   Batch merge and streaming rebuild is optimal specifically for **write-rare, read-heavy analytical workloads (OLAP)** or **LSM-tree compaction stages** where updates naturally occur in massive, pre-sorted blocks.
