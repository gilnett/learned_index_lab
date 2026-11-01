# Code Architecture & Memory Layout Guide

This document provides a deep architectural walkthrough of the `learned_index_lab` Rust implementation, detailing internal data structures, CPU cache alignment, arithmetic search mechanics, and microarchitectural engineering trade-offs.

---

## 1. Core Data Structures & Memory Alignment

### `Segment` Layout & 64-Byte Cache Line Density

In `learned_index_lab`, an index segment is defined as:

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Segment {
    pub key: u64,       // 8 bytes (offset 0)
    pub slope: f64,     // 8 bytes (offset 8)
    pub intercept: f64, // 8 bytes (offset 16)
}
```

* **Size of `Segment`**: 24 bytes of active payload.
* **Alignment**: 8-byte aligned (`align_of::<Segment>() == 8`).
* **Cache packing**: In standard x86_64 / ARM64 architectures with 64-byte L1/L2 cache lines:
  * Two contiguous `Segment` structs occupy **48 bytes** ($2 \times 24\text{ B}$), leaving 16 bytes for the start of a third segment.
  * Over sequential traversals, fetching a single 64-byte cache line brings **2.66 segments** directly into L1 cache without pointer chasing.

```
64-Byte CPU Cache Line:
+------------------------+------------------------+----------------+
| Segment 0 (24 bytes)   | Segment 1 (24 bytes)   | Segment 2 Part |
| key | slope | intercept| key | slope | intercept| key (8 bytes)  |
+------------------------+------------------------+----------------+
```

### Contiguous Data Buffer Locality

Unlike `std::collections::BTreeMap` where keys and values reside on scattered heap nodes connected by 64-bit pointers, `PgmIndex` and `MultilevelPgmIndex` maintain:
1. `segments: Vec<Segment>`: Compact array containing only segment boundaries.
2. `data: Vec<u64>`: Continuous sorted raw data buffer.

```
PgmIndex Memory Model:
[ PgmIndex Struct ] (32 bytes)
  |-- epsilon: usize
  |-- segments: Vec<Segment> ---> [ Seg 0 | Seg 1 | Seg 2 | ... | Seg S-1 ] (In L1/L2 Cache)
  +-- data: Vec<u64>         ---> [ K0 | K1 | K2 | K3 | ... | Kn-1 ]        (Contiguous RAM)
```

---

## 2. Search Mechanics: Two-Tier Arithmetic Lookup

Looking up a key $k$ in `PgmIndex` follows a strict two-stage process:

```text
Target Key k
     |
     v
[1. Global Segment Selection: binary_search(s.key <= k)]
     |
     v
Segment s (key, slope, intercept)
     |
     v
[2. In-Register ALU Prediction: approx = round(intercept + slope * (k - key))]
     |
     v
Corridor Window: [approx - eps, approx + eps]
     |
     v
[3. Local Contiguous Scan: data[low..=high].binary_search(k)]
     |
     v
Exact Index in data (or None)
```

### Step 1: Global Segment Resolution ($O(\log S)$)
* `binary_search_by_key(&target, |s| s.key)` searches over $S$ segments.
* Since $S \ll N$ (typically 100 to 1,500 segments for $10^6$ keys), this entire array (3 KB to 48 KB) resides permanently in L1/L2 CPU cache.

### Step 2: Arithmetic CDF Projection (ALU Operation)
* Once the responsible segment $s$ is loaded:
  $$\Delta x = k - s.\text{key}$$
  $$\text{pos}_{\text{approx}} = \text{round}(s.\text{intercept} + s.\text{slope} \times \Delta x)$$
* This computation requires only **1 integer subtraction**, **1 floating-point multiply-add (FMA)**, and **1 round-to-integer conversion**. Modern x86_64 CPUs execute this in $\sim 4\text{ to }6$ CPU cycles entirely inside registers.

### Step 3: Bounded Local Verification ($O(\log \epsilon)$)
* The mathematical guarantee of streaming PLA ensures:
  $$\text{pos}_{\text{actual}} \in [\text{pos}_{\text{approx}} - \epsilon, \text{pos}_{\text{approx}} + \epsilon]$$
* For $\epsilon = 16$, the verification window spans at most 33 elements ($2\epsilon + 1$).
* Contiguous memory layout guarantees that these 33 elements reside across at most 4 cache lines, minimizing TLB misses and DRAM page activation latency.

---

## 3. Hierarchical Multilevel Indexing (`MultilevelPgmIndex`)

When $N > 10^7$ or under highly irregular distributions, the number of base segments $S$ can grow large. `MultilevelPgmIndex` recursively constructs upper PGM layers over segment start keys:

```text
Level h-1 (Root PGM)      : <= 2 * eps_rec segments
        | (Top-Down Arithmetic Descent)
        v
Level 1 (Intermediate PGM): Indexes Level 0 keys
        | (Arithmetic Projection)
        v
Level 0 (Base PGM)        : Indexes raw data keys
        | (Local Search Window [pos - eps, pos + eps])
        v
Raw Data Vector           : Contiguous Vec<u64>
```

* **Lookup Complexity**: $O(h \cdot \log \epsilon_{\text{rec}} + \log \epsilon_0)$, where $h \approx \log_{2\epsilon} S$.
* Eliminates flat binary search over segments, ensuring that all index levels fit within predictable cache bounds.

---

## 4. Engineering Trade-offs: Linear Segments vs Complex Models

| Design Dimension | Streaming Linear PLA (This Lab) | High-Order Polynomials / Splines | Deep Neural Networks (MLP / RBF) |
| :--- | :--- | :--- | :--- |
| **Error Guarantees** | **Deterministic $\pm \epsilon$ worst-case** | Empirical / hard to bound | No worst-case guarantees |
| **Build Complexity** | **$O(N)$ single streaming pass** | $O(N)$ to $O(N^2)$ matrix solve | $O(\text{epochs} \cdot N)$ gradient descent |
| **ALU Execution** | **1 FMA instruction ($\sim 4$ cycles)** | Multiple multiplies + division | Matrix GEMM / activation vector calls |
| **Hardware Fit** | Standard scalar registers | SIMD / AVX-512 register pressure | GPU / NPU accelerator required |
| **Index Size** | 24 bytes per segment | 48–96 bytes per spline node | Megabytes of weight matrices |
| **Branch Predictor** | 1 branchless local scan | Multiple branch checks | Model evaluation branching |

### Why Linear Segments Win
1. **Memory Wall**: DRAM latency ($\sim 50\text{–}80\text{ ns}$) dominates modern compute. High-order curves do not reduce DRAM reads compared to a narrow $\epsilon = 16$ window, but significantly increase register pressure and ALU instruction count.
2. **Deterministic Guarantees**: A database index must provide guaranteed correctness. Streaming PLA strictly enforces that the key is within the predicted corridor without falling back to full table scans.

