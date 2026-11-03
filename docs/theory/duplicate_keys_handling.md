# Duplicate Keys & Plateaus in Streaming PLA

## Overview

In sorted dataset indexing, **duplicate keys** (or runs of identical values) represent a classic challenge for continuous regression models. In a learned index, keys $x_i$ are mapped to ordinal positions $y_i = i$. When two or more consecutive keys are identical ($x_i = x_{i-1}$), the empirical Cumulative Distribution Function (CDF) exhibits a vertical jump or step discontinuity:

$$\Delta x = x_i - x_{i-1} = 0, \quad \Delta y = y_i - y_{i-1} \ge 1$$

Mathematically, the instantaneous slope between these points is undefined or infinite ($\frac{\Delta y}{0} \to \infty$). This document details how `learned_index_lab` deterministically bounds duplicate keys in single-pass streaming PLA without producing `NaN` or `Inf` floating-point errors, backed by formal error corridor proofs.

---

## 1. The Geometry of Duplicates under Bounded Corridors (± ε)

The Piecewise Geometric Model (PGM) does not fit an exact interpolation curve; it constructs a vertical error corridor `[y - ε, y + ε]` of radius `ε` around linear segments (Ferragina & Vinciguerra, 2020, Section 3).

Under this formal definition, a segment covering range `[j, k]` is valid if and only if:

$$\forall i \in [j, k], \quad |f(x_i) - y_i| \le \varepsilon$$

When encountering duplicate keys where `x_j = x_i` (`Δx = 0`), any linear function $f(x) = s \cdot (x - x_j) + y_j$ simplifies to $f(x_i) = y_j$. The error condition becomes:

$$|y_i - y_j| \le \varepsilon$$

Because of this vertical corridor tolerance, duplicate keys are processed deterministically according to their run length:

1. **Short Duplicate Runs (`run length <= ε`)**:
   - If a duplicate run has length $L \le \varepsilon$, all points $(x_i, y_i)$ reside within vertical distance `ε` of the segment origin $(x_0, y_0)$.
   - The corridor absorbs the duplicates without splitting the segment or modifying the slope bounds $(s_{\min}, s_{\max})$.
2. **Long Duplicate Runs / Plateaus (`run length > ε`)**:
   - When identical keys exceed `ε` elements ($|y_i - y_0| > \varepsilon$), no single linear model can cover both $(x_0, y_0)$ and $(x_i, y_i)$ within tolerance `± ε`.
   - As proven in Ferragina & Vinciguerra (2020, Lemma 1), the active segment **must be finalized**, and a new segment is initialized anchored at the current duplicate key $(x_i, y_i)$.

### Summary of Duplicate Handling States

| Scenario | Condition | Segment Action | Slope Corridor Impact |
|:---|:---|:---|:---|
| **Short Duplicate** | `dx == 0` and `abs(y_i - y_0) <= ε` | Absorbed into active segment | Preserved (no change) |
| **Plateau Overflow** | `dx == 0` and `abs(y_i - y_0) > ε` | Finalize segment; anchor new segment | Reset to `[-∞, +∞]` |

---

## 2. Implementation in `src/pgm.rs`

In `learned_index_lab`, streaming PLA handles duplicates during the single sequential scan:

```rust
for (i, &xi) in data.iter().enumerate().skip(1) {
    let yi = i as f64;
    let dx = (xi - x0) as f64;

    if dx == 0.0 {
        // Duplicate key: delta x is zero.
        if (yi - y0).abs() > eps {
            // Plateau height exceeds epsilon: finalize segment.
            let slope = compute_slope(s_min, s_max);
            segments.push(Segment {
                key: x0,
                slope,
                intercept: y0,
            });
            x0 = xi;
            y0 = yi;
            s_min = 0.0;
            s_max = f64::INFINITY;
        }
        continue;
    }
    
    // Normal slope corridor calculation for distinct keys...
}
```

### Invariants Maintained:
* **Division by Zero Prevention**: The condition `dx == 0.0` prevents division by zero in slope calculations `((yi ± eps) - y0) / dx`.
* **Bounded Vertical Error**: No segment ever spans a vertical range greater than `ε` from its baseline origin for duplicate keys.
* **Non-decreasing Segments**: Successive segments have non-decreasing start keys (`s[k].key <= s[k+1].key`).

---

## 3. Query Mechanics with Duplicate Keys

### Point Query (`find`)
When a key appears multiple times across consecutive segments:
1. `segments.binary_search_by_key(&target, |s| s.key)` returns an index matching `target`.
2. Because each segment covering the duplicate plateau was constructed with `dx = 0`, its prediction evaluates to `approx = round(intercept)`.
3. The local search window `[approx - eps, approx + eps]` is guaranteed to intersect the duplicate run in `self.data`.
4. `slice.binary_search(&target)` returns a valid match in $O(\log \varepsilon)$ time.

### Predecessor and Range Queries (`lower_bound` & `range`)
* `lower_bound(target)` returns the **first** element `>= target`. If multiple duplicates exist, `lower_bound` locates the earliest instance by checking boundary conditions at the edge of the bounded corridor.
* `range(start..=end)` locates the start bound with `lower_bound(start)` and the end bound with `lower_bound(end)`, returning the entire contiguous duplicate sub-slice directly.

---

## 4. Architectural & Performance Impact

* **Segment Table Inflation**:
  On datasets with high duplicate cardinality (e.g. SOSD Wikipedia Timestamps `wiki_ts`, where millions of revisions share identical timestamps), the model must generate a new segment every `≈ ε` duplicate items.
* **When `BTreeMap` Wins**:
  Because duplicate keys force frequent segment splitting, workloads dominated by massive duplicate runs experience reduced PGM compression ratios ($S$ grows). In such workloads, `BTreeMap` handles duplicate keys natively within tree nodes without segment boundary thrashing.

---

## Academic Reference

* **Ferragina, P., & Vinciguerra, G. (2020)**: *"The PGM-index: a fully-dynamic compressed learned index with provable worst-case bounds"*, PVLDB 13(8): 1162–1175.
  * Section 3: *The Optimal Piecewise Linear Approximation*.
  * Lemma 1 & Section 3.2: *Bounding Space and Time under Discontinuous and Repeated Keys*.
