// Copyright (c) Learned Index
// Implementation of Piecewise Geometric Model Index (PGM-Index)
// Optimal Piecewise Linear Approximation (PLA) in single streaming pass

#[derive(Debug, Clone, Copy)]
pub struct Segment {
    pub key: u64,
    pub slope: f64,
    pub intercept: f64,
}

#[derive(Debug, Clone)]
pub struct PgmIndex {
    pub epsilon: usize,
    pub segments: Vec<Segment>,
    pub data: Vec<u64>,
}

impl PgmIndex {
    /// Builds the PGM index over sorted data with maximum error epsilon.
    pub fn build(data: Vec<u64>, epsilon: usize) -> Self {
        assert!(epsilon >= 1, "Epsilon must be >= 1");
        if data.is_empty() {
            return Self {
                epsilon,
                segments: Vec::new(),
                data,
            };
        }

        debug_assert!(
            data.windows(2).all(|w| w[0] <= w[1]),
            "PGM-Index requires pre-sorted data in non-decreasing order"
        );

        let eps = epsilon as f64;
        let mut segments = Vec::new();

        let mut x0 = data[0];
        let mut y0 = 0.0f64;

        let compute_slope = |s_min: f64, s_max: f64| -> f64 {
            if s_max.is_finite() {
                (s_min + s_max) / 2.0
            } else {
                s_min
            }
        };

        let mut s_min = 0.0f64;
        let mut s_max = f64::INFINITY;

        for (i, &xi) in data.iter().enumerate().skip(1) {
            let yi = i as f64;
            let dx = (xi - x0) as f64;

            if dx == 0.0 {
                // Duplicate key
                if (yi - y0).abs() > eps {
                    // Force segment completion
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

            let low_slope = ((yi - eps) - y0) / dx;
            let high_slope = ((yi + eps) - y0) / dx;

            let next_s_min = s_min.max(low_slope);
            let next_s_max = s_max.min(high_slope);

            if next_s_min <= next_s_max {
                s_min = next_s_min;
                s_max = next_s_max;
            } else {
                // Current point cannot be added to this segment within epsilon
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
        }

        // Push the final active segment
        let slope = compute_slope(s_min, s_max);
        segments.push(Segment {
            key: x0,
            slope,
            intercept: y0,
        });

        Self {
            epsilon,
            segments,
            data,
        }
    }

    /// Finds the exact index of target key if present.
    #[inline]
    pub fn find(&self, target: u64) -> Option<usize> {
        if self.data.is_empty() {
            return None;
        }

        let n = self.data.len();
        if target < self.data[0] || target > self.data[n - 1] {
            return None;
        }

        // Binary search to find responsible segment: segments[idx].key <= target
        let seg_idx = match self.segments.binary_search_by_key(&target, |s| s.key) {
            Ok(idx) => idx,
            Err(0) => 0,
            Err(idx) => idx - 1,
        };

        let seg = &self.segments[seg_idx];
        let dx = (target.saturating_sub(seg.key)) as f64;
        let approx = (seg.intercept + seg.slope * dx).round() as isize;

        let eps = self.epsilon as isize;
        let max_idx = (n - 1) as isize;
        let low = (approx - eps).clamp(0, max_idx) as usize;
        let high = (approx + eps).clamp(0, max_idx) as usize;

        if low > high {
            return None;
        }

        let slice = &self.data[low..=high];
        match slice.binary_search(&target) {
            Ok(offset) => Some(low + offset),
            Err(_) => None,
        }
    }

    /// Returns the index of the first element that is greater than or equal to `target`.
    ///
    /// If all elements in the dataset are strictly less than `target`, returns `self.data.len()`.
    ///
    /// # Complexity
    /// `O(log S + log epsilon)` where `S` is the number of linear segments.
    #[inline]
    pub fn lower_bound(&self, target: u64) -> usize {
        let n = self.data.len();
        if n == 0 {
            return 0;
        }
        if target <= self.data[0] {
            return 0;
        }
        if target > self.data[n - 1] {
            return n;
        }

        let seg_idx = match self.segments.binary_search_by_key(&target, |s| s.key) {
            Ok(idx) => idx,
            Err(0) => 0,
            Err(idx) => idx - 1,
        };

        let seg = &self.segments[seg_idx];
        let dx = target.saturating_sub(seg.key) as f64;
        let approx = (seg.intercept + seg.slope * dx).round() as isize;

        let eps = self.epsilon as isize;
        let max_idx = (n - 1) as isize;
        let low = (approx - eps).clamp(0, max_idx) as usize;
        let high = (approx + eps).clamp(0, max_idx) as usize;

        if low <= high {
            let slice = &self.data[low..=high];
            let offset = slice.partition_point(|&x| x < target);
            if offset > 0 && offset < slice.len() {
                return low + offset;
            }
            if offset == 0 && (low == 0 || self.data[low - 1] < target) {
                return low;
            }
            if offset == slice.len() && high == n - 1 {
                return n;
            }
        }

        debug_assert!(
            false,
            "Invariant violation: bounded window [{}, {}] missed lower_bound target {} in PGM segment",
            low, high, target
        );

        self.data.partition_point(|&x| x < target)
    }

    /// Returns a slice of keys within the specified range bounds.
    ///
    /// # Example
    /// ```rust
    /// use learned_index::PgmIndex;
    ///
    /// let data = vec![10, 20, 30, 40, 50, 60, 70];
    /// let pgm = PgmIndex::build(data, 16);
    /// assert_eq!(pgm.range(25..=55), &[30, 40, 50]);
    /// assert_eq!(pgm.range(..30), &[10, 20]);
    /// ```
    pub fn range<R: std::ops::RangeBounds<u64>>(&self, bounds: R) -> &[u64] {
        if self.data.is_empty() {
            return &[];
        }

        let start_idx = match bounds.start_bound() {
            std::ops::Bound::Included(&start) => self.lower_bound(start),
            std::ops::Bound::Excluded(&start) => {
                let lb = self.lower_bound(start);
                let mut idx = lb;
                while idx < self.data.len() && self.data[idx] == start {
                    idx += 1;
                }
                idx
            }
            std::ops::Bound::Unbounded => 0,
        };

        let end_idx = match bounds.end_bound() {
            std::ops::Bound::Included(&end) => {
                let lb = self.lower_bound(end);
                let mut idx = lb;
                while idx < self.data.len() && self.data[idx] == end {
                    idx += 1;
                }
                idx
            }
            std::ops::Bound::Excluded(&end) => self.lower_bound(end),
            std::ops::Bound::Unbounded => self.data.len(),
        };

        if start_idx >= end_idx || start_idx >= self.data.len() {
            &[]
        } else {
            &self.data[start_idx..end_idx.min(self.data.len())]
        }
    }

    /// Total memory footprint in bytes (data buffer + segments + struct overhead)
    pub fn memory_footprint_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + (self.segments.capacity() * std::mem::size_of::<Segment>())
            + (self.data.capacity() * std::mem::size_of::<u64>())
    }

    /// Memory of the index metadata only (without raw data)
    pub fn index_only_bytes(&self) -> usize {
        (self.segments.capacity() * std::mem::size_of::<Segment>())
            + std::mem::size_of::<PgmIndex>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_and_single() {
        let pgm = PgmIndex::build(vec![], 16);
        assert_eq!(pgm.find(42), None);

        let pgm = PgmIndex::build(vec![100], 16);
        assert_eq!(pgm.find(100), Some(0));
        assert_eq!(pgm.find(99), None);
        assert_eq!(pgm.find(101), None);
    }

    #[test]
    fn test_perfect_linear() {
        let data: Vec<u64> = (0..10_000).map(|x| x * 10).collect();
        let pgm = PgmIndex::build(data.clone(), 32);

        // A strictly linear sequence should fit in very few segments (often 1 or 2)
        assert!(pgm.segments.len() <= 2);

        for (i, &v) in data.iter().enumerate() {
            assert_eq!(pgm.find(v), Some(i));
            assert_eq!(pgm.find(v + 1), None);
        }
    }

    #[test]
    fn test_edge_cases_and_gaps() {
        let data = vec![1, 5, 10, 15, 20, 100, 200, 500, 1000, 5000, 10000];
        let pgm = PgmIndex::build(data.clone(), 4);

        for (i, &v) in data.iter().enumerate() {
            assert_eq!(pgm.find(v), Some(i));
        }

        assert_eq!(pgm.find(0), None);
        assert_eq!(pgm.find(25), None);
        assert_eq!(pgm.find(99999), None);
    }

    #[test]
    fn test_duplicates_and_identical_elements() {
        let data = vec![5, 5, 5, 5, 5, 5, 5, 5, 5, 5];
        let pgm = PgmIndex::build(data.clone(), 2);
        assert!(pgm.find(5).is_some());
        assert_eq!(pgm.find(4), None);
        assert_eq!(pgm.find(6), None);

        let mixed = vec![1, 2, 2, 2, 3, 5, 5, 10, 10, 10, 12];
        let pgm_mixed = PgmIndex::build(mixed.clone(), 2);
        for &v in &mixed {
            let idx = pgm_mixed.find(v);
            assert!(idx.is_some());
            assert_eq!(mixed[idx.unwrap()], v);
        }
    }

    #[test]
    fn test_minimal_epsilon_one() {
        let data = vec![10, 20, 30, 40, 50];
        let pgm = PgmIndex::build(data.clone(), 1);
        for (i, &v) in data.iter().enumerate() {
            assert_eq!(pgm.find(v), Some(i));
        }
    }

    #[test]
    fn test_extreme_u64_limits() {
        let data = vec![
            u64::MAX - 1000,
            u64::MAX - 500,
            u64::MAX - 100,
            u64::MAX - 1,
            u64::MAX,
        ];
        let pgm = PgmIndex::build(data.clone(), 2);
        for (i, &v) in data.iter().enumerate() {
            assert_eq!(pgm.find(v), Some(i));
        }
        assert_eq!(pgm.find(u64::MAX - 2000), None);
    }

    #[test]
    fn test_memory_accounting() {
        let data = vec![1, 2, 3, 4, 5];
        let pgm = PgmIndex::build(data, 4);
        assert!(pgm.memory_footprint_bytes() > 0);
        assert!(pgm.index_only_bytes() > 0);
        assert!(pgm.memory_footprint_bytes() > pgm.index_only_bytes());
    }

    #[test]
    #[should_panic(expected = "Epsilon must be >= 1")]
    fn test_zero_epsilon_panics() {
        let _ = PgmIndex::build(vec![1, 2, 3], 0);
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "PGM-Index requires pre-sorted data")]
    fn test_unsorted_data_panics_in_debug() {
        let _ = PgmIndex::build(vec![3, 1, 2], 4);
    }

    #[test]
    fn test_lower_bound_comprehensive() {
        let data = vec![10, 20, 30, 40, 50, 60, 70, 80, 90, 100];
        let pgm = PgmIndex::build(data.clone(), 4);

        assert_eq!(pgm.lower_bound(5), 0);
        assert_eq!(pgm.lower_bound(10), 0);
        assert_eq!(pgm.lower_bound(15), 1);
        assert_eq!(pgm.lower_bound(20), 1);
        assert_eq!(pgm.lower_bound(95), 9);
        assert_eq!(pgm.lower_bound(100), 9);
        assert_eq!(pgm.lower_bound(101), 10);
        assert_eq!(pgm.lower_bound(9999), 10);

        // Duplicates test
        let dup_data = vec![10, 20, 20, 20, 30];
        let pgm_dup = PgmIndex::build(dup_data.clone(), 2);
        assert_eq!(pgm_dup.lower_bound(20), 1);
    }

    #[test]
    fn test_range_queries() {
        let data = vec![10, 20, 30, 40, 50, 60, 70, 80, 90, 100];
        let pgm = PgmIndex::build(data.clone(), 4);

        // Bounded inclusive
        assert_eq!(pgm.range(25..=65), &[30, 40, 50, 60]);
        assert_eq!(pgm.range(20..=60), &[20, 30, 40, 50, 60]);

        // Half-open
        assert_eq!(pgm.range(20..60), &[20, 30, 40, 50]);

        // Unbounded ends
        assert_eq!(pgm.range(..40), &[10, 20, 30]);
        assert_eq!(pgm.range(80..), &[80, 90, 100]);
        assert_eq!(pgm.range(..), &data[..]);

        // Out-of-bounds and inverted ranges
        assert_eq!(pgm.range(150..200), &[]);
        assert_eq!(pgm.range(..5), &[]);
        assert_eq!(pgm.range(50..50), &[]);
        let (inv_start, inv_end) = (80, 20);
        assert_eq!(pgm.range(inv_start..inv_end), &[]);
    }

    #[test]
    fn test_lower_bound_and_range_empty_and_single() {
        let empty_pgm = PgmIndex::build(vec![], 4);
        assert_eq!(empty_pgm.lower_bound(10), 0);
        assert_eq!(empty_pgm.range(..), &[]);
        assert_eq!(empty_pgm.range(10..20), &[]);

        let single_pgm = PgmIndex::build(vec![42], 4);
        assert_eq!(single_pgm.lower_bound(10), 0);
        assert_eq!(single_pgm.lower_bound(42), 0);
        assert_eq!(single_pgm.lower_bound(100), 1);
        assert_eq!(single_pgm.range(40..=50), &[42]);
        assert_eq!(single_pgm.range(43..50), &[]);
        assert_eq!(single_pgm.range(..42), &[]);
        assert_eq!(single_pgm.range(42..), &[42]);
    }

    #[test]
    fn test_lower_bound_partition_point_parity() {
        // Deterministic sequence with non-linear intervals and duplicates
        let mut data = Vec::with_capacity(1_000);
        let mut current = 10u64;
        for i in 0..1_000 {
            current += (i % 7) as u64;
            data.push(current);
        }
        let pgm = PgmIndex::build(data.clone(), 8);

        // Verify across all contained keys and out-of-bounds keys
        for probe in (0..current + 20).step_by(3) {
            let expected = data.partition_point(|&x| x < probe);
            let actual = pgm.lower_bound(probe);
            assert_eq!(
                actual, expected,
                "Mismatch for probe {probe}: expected index {expected}, got {actual}"
            );
        }
    }
}
