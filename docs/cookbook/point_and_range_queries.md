# Recipe 1: Point Lookups, Predecessors & Contiguous Range Scans

This recipe covers standard query patterns for single-level and multi-level learned indexes: initializing the index, exact key lookups, predecessor searches, and zero-copy contiguous range queries.

---

## 1. Initializing & Sizing the Index

Choose between single-level `PgmIndex` and hierarchical `MultilevelPgmIndex` based on dataset magnitude:

```rust
use learned_index_lab::{MultilevelPgmIndex, PgmIndex};

fn build_indexes(keys: Vec<u64>) {
    let n = keys.len();

    if n <= 10_000_000 {
        // Single-level PGM-Index: optimal for datasets up to 10M keys
        // ε = 32 maps the 65-key local window into ~8 CPU cache lines (512 bytes)
        let pgm = PgmIndex::build(keys, 32);
        println!(
            "PgmIndex: {} segments ({:.2} KB metadata)",
            pgm.segments().len(),
            pgm.metadata_bytes() as f64 / 1024.0
        );
    } else {
        // Multilevel PGM-Index: for massive scale (> 10M keys)
        // ε = 32 at base layer, ε_rec = 16 for upper recursive index layers
        let ml_pgm = MultilevelPgmIndex::build_with_epsilons(keys, 32, 16);
        println!(
            "MultilevelPgmIndex: {} levels ({:.2} KB metadata)",
            ml_pgm.height(),
            ml_pgm.metadata_bytes() as f64 / 1024.0
        );
    }
}
```

---

## 2. Exact Point Lookup (`find`)

`find` locates the exact index of a target key in $O(\log S + \varepsilon)$ time. If duplicates exist, it returns an arbitrary matching position.

```rust
use learned_index_lab::PgmIndex;

fn execute_point_lookup(pgm: &PgmIndex, target_key: u64) {
    match pgm.find(target_key) {
        Some(idx) => {
            let val = pgm.data()[idx];
            assert_eq!(val, target_key);
            println!("Key {} found at offset {}", target_key, idx);
        }
        None => println!("Key {} is not present in index", target_key),
    }
}
```

---

## 3. Predecessor Search (`lower_bound`)

`lower_bound` returns the index of the first element greater than or equal to `target_key`, identical to `std::slice::partition_point(|&k| k < target_key)`.

```rust
use learned_index_lab::PgmIndex;

fn execute_predecessor_search(pgm: &PgmIndex, query_key: u64) {
    let idx = pgm.lower_bound(query_key);

    if idx < pgm.data().len() {
        println!(
            "First key >= {}: at index {} (value = {})",
            query_key,
            idx,
            pgm.data()[idx]
        );
    } else {
        println!("All keys in the dataset are strictly smaller than {}", query_key);
    }
}
```

---

## 4. Zero-Copy Contiguous Range Queries (`range`)

`range` evaluates the bounding lower bounds for `[start_key, end_key)` and returns a direct borrowed slice `&[u64]`. Because the underlying data is contiguous in memory, range queries incur zero memory allocations.

```rust
use learned_index_lab::PgmIndex;

fn execute_range_query(pgm: &PgmIndex, start_ts: u64, end_ts: u64) -> &[u64] {
    // Zero heap allocations: returns a direct subslice &[u64]
    let slice = pgm.range(start_ts, end_ts);

    println!(
        "Range [{}, {}) matched {} contiguous records",
        start_ts,
        end_ts,
        slice.len()
    );

    slice
}
```

### Boundary & Edge Case Semantics

- **Inverted Range**: If `start_key >= end_key`, `range` immediately returns an empty slice `&[]`.
- **Out of Bounds**: If `end_key > max_key`, `range` safely truncates to `data.len()`.
- **Extreme Limits**: Fully supports `u64::MIN` (0) and `u64::MAX` without integer overflow or panics.
