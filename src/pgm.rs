// Copyright (c) Learned Index Lab
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
    #[should_panic(expected = "PGM-Index requires pre-sorted data")]
    fn test_unsorted_data_panics_in_debug() {
        let _ = PgmIndex::build(vec![3, 1, 2], 4);
    }
}
