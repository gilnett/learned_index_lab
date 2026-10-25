# Roadmap: learned_index_lab

This document outlines planned improvements, architectural documentation, and experimental benchmarks for upcoming releases.

---

## v0.4.0: Real-World Datasets, Hierarchy & Architecture Guide

- [ ] **Code Architecture & Memory Layout Guide**:
  - Detailed walkthrough of the internal Rust implementation (`PgmIndex`, `Segment`, search algorithms).
  - Explicit layout diagrams showing cache line boundaries, alignment, and data buffer locality.
  - Explanation of engineering trade-offs (e.g. linear segments vs cubic splines: why ALU simplicity and streaming PLA win over complex curve fitting).
- [ ] **Hierarchical Multilevel PGM-Index**:
  - Recursive segment indexing ($O(\log N)$ to $O(\log S / \epsilon)$ reduction) for large-scale datasets.
  - Replaces flat binary search over segments with top-level PGM layers.
- [ ] **SOSD Benchmark Suite Integration**:
  - Ingest real-world datasets from Marcus et al. (OpenStreetMap `osm_cellids`, `wiki_ts`, `books`).
  - Stress-test irregular, non-smooth CDFs, key hotspots, and duplicate values.
  - Measure segment count inflation and cache residency on irregular distributions.
- [ ] **Batch Ingestion & Rebuild Benchmarking**:
  - Evaluate streaming PLA rebuild time after append/batch inserts against dynamic `BTreeMap` in-place inserts.
  - Profile memory throughput during batch merges in immutable SSTable-like workloads.

---

## Current Status (v0.3.x)

- Single-pass Optimal Streaming Piecewise Linear Approximation (PLA) with strict $\epsilon$ error bounds.
- Multi-target statistical harness comparing `BTreeMap`, `Flat slice`, `EytzingerSearch`, and `PgmIndex`.
- 3 synthetic workloads: Sequential Time-Series, Uniform Sparse, and Clustered Lognormal.
- Range scan support via `lower_bound`.
