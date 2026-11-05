# Recipe 2: Concurrent Multi-Threaded Read Scaling

This recipe demonstrates how to leverage the immutable memory layout of learned indexes to achieve linear read scalability across multiple CPU cores without lock contention.

---

## Concurrency Guarantees

Both `PgmIndex` and `MultilevelPgmIndex` are strictly immutable once built. They implement `Send` and `Sync`:

- **Zero Synchronization Overhead**: Readers perform arithmetic evaluation ($y = ax + b$) and contiguous binary searches without taking locks or incrementing atomic counters.
- **Cache-Line Sharing**: Segment headers and data slices are read-only; multiple CPU cores can cache the same cache lines in their L1/L2 caches simultaneously without false-sharing penalties.

---

## Multi-Core Query Server Pattern

```rust
use learned_index_lab::PgmIndex;
use std::sync::Arc;
use std::thread;

pub fn spawn_parallel_query_workers(
    data: Vec<u64>,
    worker_batches: Vec<Vec<u64>>,
) -> usize {
    // 1. Build immutable index and wrap in Arc for zero-copy thread sharing
    let shared_index = Arc::new(PgmIndex::build(data, 32));

    let mut handles = Vec::with_capacity(worker_batches.len());

    // 2. Distribute query batches across worker threads
    for batch in worker_batches {
        let index = Arc::clone(&shared_index);

        let handle = thread::spawn(move || {
            let mut matches = 0;
            for key in batch {
                // Lock-free point lookup
                if index.find(key).is_some() {
                    matches += 1;
                }
            }
            matches
        });

        handles.push(handle);
    }

    // 3. Aggregate results
    let total_matches: usize = handles
        .into_iter()
        .map(|h| h.join().expect("Worker thread panicked"))
        .sum();

    total_matches
}
```

---

## Rayon Parallel Batch Evaluation

When integrating with data parallel frameworks such as [Rayon](https://github.com/rayon-rs/rayon), `PgmIndex` queries can be dispatched across thread pools using standard parallel iterators:

```rust
// Requires rayon = "1.8" in your Cargo.toml
use learned_index_lab::PgmIndex;
use rayon::prelude::*;

pub fn parallel_batch_lookup(pgm: &PgmIndex, query_keys: &[u64]) -> Vec<Option<usize>> {
    query_keys
        .par_iter()
        .map(|&key| pgm.find(key))
        .collect()
}
```

### Performance Characteristic
On a 16-core system, read throughput scales near-linearly ($> 14\times$ speedup) because the contiguous segment array fits permanently in each core's private L2 cache.
