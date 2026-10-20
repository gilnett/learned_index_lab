# Production Architecture & Real-World Applications

This document details practical integration patterns, performance trade-offs, and target use cases for learned index structures.

---

## Decision Matrix: Selecting the Right Index Structure

| Workload Characteristic | Recommended Structure | Rationale |
| :--- | :--- | :--- |
| **Frequent in-place mutations / OLTP** | **`BTreeMap` / `B+Tree`** | Better suited for dynamic concurrent writes, page splits, and transaction management. |
| **Immutable / Read-heavy sorted datasets** | **`PgmIndex`** | Ideal for SSTables in LSM engines, Parquet columnar row groups, and static lookup tables. |
| **Memory-constrained environments** | **`PgmIndex`** | Reduces metadata footprint by up to 14,000x compared to pointer-based trees. |
| **Cache-friendly binary search** | **`EytzingerSearch`** | Replaces standard `slice::binary_search` with branchless level-order traversal when duplicate data buffer is tolerable. |
| **Zero extra memory allowed** | **`slice::binary_search`** | In-situ search over pre-sorted data with 0 KB metadata overhead. |

---

## Production Applications

### 1. Columnar OLAP Engines ([ClickHouse](https://github.com/ClickHouse/ClickHouse), [DuckDB](https://github.com/duckdb/duckdb), [Apache Parquet](https://github.com/apache/parquet-format))
Analytical columnar engines process sorted row groups containing millions of records. Replacing conventional coarse zone maps with PGM linear segments enables sub-microsecond tuple filtering without disk or memory amplification.

### 2. LSM-Tree Storage Engines ([RocksDB](https://github.com/facebook/rocksdb), [ScyllaDB](https://github.com/scylladb/scylladb), [LevelDB](https://github.com/google/leveldb))
SSTables written to NVMe storage are strictly immutable. Replacing sparse block index tables with compact PGM models allows billions of key offsets to reside resident in CPU cache or RAM, ensuring exactly 1 physical I/O per query.

### 3. OS File System Cataloging & Forensic Indexing
Fast file catalogers (e.g. NTFS Master File Table / USN journal indexers) can index tens of millions of records in only a few kilobytes of RAM.

### 4. High-Throughput Network Routing (Forwarding Information Base)
At line rates exceeding 100 Gbps, IP forwarding tables must guarantee bounded lookup latency without saturating CPU cache lines.
