# Roadmap: learned_index_lab

This document outlines planned improvements, architectural documentation, and experimental benchmarks for upcoming releases.

---

## v0.4.0: Real-World Datasets, Hierarchy & Architecture Guide

- [x] **Code Architecture & Memory Layout Guide**:
  - Detailed walkthrough of the internal Rust implementation (`PgmIndex`, `Segment`, search algorithms).
  - Explicit layout diagrams showing cache line boundaries, alignment, and data buffer locality.
  - Explanation of engineering trade-offs (e.g. linear segments vs cubic splines: why ALU simplicity and streaming PLA win over complex curve fitting).
  - Available at: [`docs/architecture/code_architecture_and_memory_layout.md`](docs/architecture/code_architecture_and_memory_layout.md).
- [x] **Hierarchical Multilevel PGM-Index**:
  - Recursive segment indexing ($O(\log N)$ to $O(\log S / \epsilon)$ reduction) for large-scale datasets.
  - Implemented in `src/multilevel.rs` (`MultilevelPgmIndex`) and integrated into benchmark targets (`--target multilevel`).
- [x] **SOSD Benchmark Suite Integration**:
  - Real-world distribution profiles from Marcus et al. (OpenStreetMap `osm_cellids`, `wiki_ts`, `books`).
  - Stress-tests irregular, non-smooth CDFs, bursts, duplicates, and key hotspots via `--suite osm,wiki,books,sosd`.
- [x] **Batch Ingestion & Rebuild Benchmarking**:
  - Evaluates streaming PLA 2-way batch merge & rebuild throughput against dynamic in-place `BTreeMap` mutations.
  - Supported via `--batch` and `--batch-size <N>`.

---

## Current Status (v0.4.0)

- Single-pass Optimal Streaming Piecewise Linear Approximation (PLA) with strict $\epsilon$ error bounds.
- Hierarchical recursive indexing (`MultilevelPgmIndex`) with $O(h \cdot \log \epsilon)$ lookup latency.
- Modular multi-target benchmark harness comparing 5 search paradigms (`BTreeMap`, `Flat slice`, `EytzingerSearch`, `PgmIndex`, `MultilevelPgmIndex`).
- 6 workload distributions: 3 synthetic (Dense Time-Series, Uniform Sparse, Clustered Lognormal) and 3 real-world SOSD benchmarks (OSM CellIDs, Wiki Timestamps, Book ISBNs).
- Batch ingestion & rebuild benchmarking comparing 2-way streaming PLA merges to dynamic in-place `BTreeMap` inserts.
- Range query and predecessor search support via `lower_bound` and `range` with full $u64$ limit parity.

---

## Planned for v0.5.0: SIMD Acceleration & Massive Scale Ingestion

- [ ] **SIMD-Vectorized Local Search & In-Register Filtering**:
  - AVX2 / ARM NEON vectorization for local bounded scans within $\pm \epsilon$ corridors.
  - Vectorized evaluation of adjacent segment keys using explicit SIMD comparison instructions.
- [ ] **Memory-Mapped Dataset Loader (Zero-Copy)**:
  - Zero-copy memory-mapped file reader for massive external SOSD binary datasets (> 100M keys) without double-buffering heap memory.
- [ ] **Adaptive Epsilon Auto-Tuning**:
  - Dynamic local $\epsilon$ tuning based on segment curvature and slope variance to minimize memory-latency Pareto frontier.
- [ ] **Concurrent Read-Write Hierarchy**:
  - Latch-free multi-version segment updates allowing concurrent point queries during background batch ingestion.
