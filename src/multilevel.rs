//! # Hierarchical Multilevel PGM-Index
//!
//! Recursively indexes linear segments to achieve $O(h \cdot \log \epsilon)$ lookup latency,
//! where $h \approx \log_{2\epsilon} S$ is the tree height, eliminating flat binary search over segments.

use crate::pgm::{Segment, build_segments};

/// A hierarchical, multi-level Piecewise Geometric Model Index.
///
/// While a standard single-level `PgmIndex` performs a flat binary search over its $S$ segments ($O(\log S)$),
/// `MultilevelPgmIndex` recursively constructs higher-level indexing layers over the starting keys of the
/// segments at each lower level. This reduces the search overhead from $O(\log S + \log \epsilon)$ to
/// $O(h \cdot \log \epsilon_{\text{rec}} + \log \epsilon_0)$, ensuring all searches remain strictly bounded
/// and L1/L2 cache-resident.
#[derive(Debug, Clone)]
pub struct MultilevelPgmIndex {
    /// Epsilon error bound for the base level 0 indexing the raw data.
    pub epsilon: usize,
    /// Epsilon error bound used for upper recursive levels indexing segment keys.
    pub epsilon_recursive: usize,
    /// Vector of level segments from level 0 (base) to level $h-1$ (root).
    pub levels: Vec<Vec<Segment>>,
    /// Contiguous sorted data buffer.
    pub data: Vec<u64>,
}

impl MultilevelPgmIndex {
    /// Builds a multi-level PGM index with custom base epsilon and recursive epsilon.
    pub fn build_with_epsilons(data: Vec<u64>, epsilon: usize, epsilon_recursive: usize) -> Self {
        assert!(epsilon >= 1, "Base epsilon must be >= 1");
        assert!(epsilon_recursive >= 1, "Recursive epsilon must be >= 1");

        if data.is_empty() {
            return Self {
                epsilon,
                epsilon_recursive,
                levels: Vec::new(),
                data,
            };
        }

        // 1. Build level 0 over data
        let level_0 = build_segments(&data, epsilon);
        let mut levels = vec![level_0];

        // 2. Recursively build upper levels until top level is small enough
        loop {
            let current_level = levels.last().expect("invariant: levels is non-empty");
            if current_level.len() <= 2 * epsilon_recursive || current_level.len() <= 1 {
                break;
            }

            let keys: Vec<u64> = current_level.iter().map(|s| s.key).collect();
            let next_level = build_segments(&keys, epsilon_recursive);

            // Avoid infinite loop if pathological distributions do not shrink
            if next_level.len() >= current_level.len() {
                break;
            }

            levels.push(next_level);
        }

        Self {
            epsilon,
            epsilon_recursive,
            levels,
            data,
        }
    }

    /// Builds a multi-level PGM index using the same epsilon for base and recursive layers.
    pub fn build(data: Vec<u64>, epsilon: usize) -> Self {
        Self::build_with_epsilons(data, epsilon, epsilon)
    }

    /// Tree height (number of levels). Returns 0 for empty dataset.
    #[inline]
    pub fn height(&self) -> usize {
        self.levels.len()
    }

    /// Total count of segments across all levels of the hierarchy.
    #[inline]
    pub fn total_segments(&self) -> usize {
        self.levels.iter().map(|lvl| lvl.len()).sum()
    }

    /// Memory consumed by the segment index hierarchy in bytes.
    #[inline]
    pub fn index_size_in_bytes(&self) -> usize {
        self.levels
            .iter()
            .map(|lvl| lvl.len() * std::mem::size_of::<Segment>())
            .sum()
    }

    /// Memory consumed by the indexed raw data in bytes.
    #[inline]
    pub fn data_size_in_bytes(&self) -> usize {
        self.data.len() * std::mem::size_of::<u64>()
    }

    /// Finds the exact index of the target key if present.
    #[inline]
    pub fn find(&self, target: u64) -> Option<usize> {
        let n = self.data.len();
        if n == 0 || target < self.data[0] || target > self.data[n - 1] {
            return None;
        }

        let seg_idx = self.find_base_segment_index(target);
        let seg = &self.levels[0][seg_idx];

        let dx = target.saturating_sub(seg.key) as f64;
        let approx = (seg.intercept + seg.slope * dx).round() as isize;

        let eps = self.epsilon as isize;
        let max_idx = (n - 1) as isize;
        let low = (approx - eps).clamp(0, max_idx) as usize;
        let high = (approx + eps).clamp(0, max_idx) as usize;

        let slice = &self.data[low..=high];
        if let Ok(offset) = slice.binary_search(&target) {
            return Some(low + offset);
        }

        // Bounded fallback in the rare event of numeric drift
        if target < self.data[low] {
            self.data[..low].binary_search(&target).ok()
        } else if target > self.data[high] {
            self.data[high..]
                .binary_search(&target)
                .ok()
                .map(|off| high + off)
        } else {
            None
        }
    }

    /// Returns the index of the first element that is greater than or equal to `target`.
    ///
    /// If all elements are strictly less than `target`, returns `self.data.len()`.
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

        let seg_idx = self.find_base_segment_index(target);
        let seg = &self.levels[0][seg_idx];

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

        // Exact fallback via partition_point on the whole array in case of extreme numeric drift
        self.data.partition_point(|&x| x < target)
    }

    /// Performs a range query returning a borrowed slice of all keys in `[min_key, max_key]`.
    #[inline]
    pub fn range(&self, min_key: u64, max_key: u64) -> &[u64] {
        if min_key > max_key || self.data.is_empty() {
            return &[];
        }
        let start = self.lower_bound(min_key);
        let end = self.lower_bound(max_key.saturating_add(1));
        if start < end && start < self.data.len() {
            &self.data[start..end.min(self.data.len())]
        } else {
            &[]
        }
    }

    /// Internal top-down descent through the segment hierarchy to find the responsible segment index at level 0.
    #[inline]
    fn find_base_segment_index(&self, target: u64) -> usize {
        let h = self.levels.len();
        if h == 0 {
            return 0;
        }

        // 1. Search top level (at most 2 * epsilon_recursive segments)
        let root_level = &self.levels[h - 1];
        let mut seg_idx = match root_level.binary_search_by_key(&target, |s| s.key) {
            Ok(idx) => idx,
            Err(0) => 0,
            Err(idx) => idx - 1,
        };

        // 2. Descend through each level from h - 1 down to 1
        for level_idx in (1..h).rev() {
            let parent_seg = &self.levels[level_idx][seg_idx];
            let child_level = &self.levels[level_idx - 1];
            let m = child_level.len();

            let dx = target.saturating_sub(parent_seg.key) as f64;
            let approx = (parent_seg.intercept + parent_seg.slope * dx).round() as isize;

            let eps = self.epsilon_recursive as isize;
            let max_idx = (m - 1) as isize;
            let low = (approx - eps - 2).clamp(0, max_idx) as usize;
            let high = (approx + eps + 2).clamp(0, max_idx) as usize;

            if target < child_level[low].key {
                // Target is before window: search in prefix
                seg_idx = match child_level[..low].binary_search_by_key(&target, |s| s.key) {
                    Ok(i) => i,
                    Err(0) => 0,
                    Err(i) => i - 1,
                };
            } else if target >= child_level[high].key {
                // Target is at or after high: search in suffix
                let slice = &child_level[high..];
                seg_idx = match slice.binary_search_by_key(&target, |s| s.key) {
                    Ok(i) => high + i,
                    Err(i) => high + (i - 1),
                };
            } else {
                // Target is within [low, high]: binary search the slice
                let slice = &child_level[low..=high];
                let local_idx = match slice.binary_search_by_key(&target, |s| s.key) {
                    Ok(i) => i,
                    Err(0) => 0,
                    Err(i) => i - 1,
                };
                seg_idx = low + local_idx;
            }
        }

        seg_idx
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multilevel_empty_and_single() {
        let empty: Vec<u64> = vec![];
        let index = MultilevelPgmIndex::build(empty, 4);
        assert_eq!(index.height(), 0);
        assert_eq!(index.find(10), None);
        assert_eq!(index.lower_bound(10), 0);
        assert_eq!(index.range(0, 100), &[]);

        let single = vec![42];
        let index = MultilevelPgmIndex::build(single, 4);
        assert_eq!(index.height(), 1);
        assert_eq!(index.find(42), Some(0));
        assert_eq!(index.find(10), None);
        assert_eq!(index.lower_bound(40), 0);
        assert_eq!(index.lower_bound(42), 0);
        assert_eq!(index.lower_bound(45), 1);
        assert_eq!(index.range(40, 50), &[42]);
    }

    #[test]
    fn test_multilevel_hierarchy_construction() {
        // Construct a dataset large enough with small epsilon to force >= 2 levels
        let data: Vec<u64> = (0..5000).map(|i| (i * i) / 10 + i * 2).collect();
        let index = MultilevelPgmIndex::build_with_epsilons(data.clone(), 4, 4);

        assert!(
            index.height() >= 2,
            "Expected at least 2 levels for 5000 parabolic elements with eps=4"
        );
        assert!(index.levels[0].len() > index.levels[1].len());

        // Verify find parity for every single element in dataset
        for (i, &key) in data.iter().enumerate() {
            let found = index.find(key);
            assert!(
                found.is_some(),
                "Key {key} at index {i} must be found in multilevel index"
            );
            assert_eq!(data[found.unwrap()], key);
        }

        // Verify non-existent keys
        assert_eq!(index.find(data.last().unwrap() + 1), None);
        assert_eq!(index.find(data.last().unwrap() + 100), None);
    }

    #[test]
    fn test_multilevel_lower_bound_parity() {
        let data: Vec<u64> = (0..2000).map(|i| i * 7).collect();
        let index = MultilevelPgmIndex::build(data.clone(), 8);

        for target in 0..14050 {
            let expected = data.partition_point(|&x| x < target);
            let actual = index.lower_bound(target);
            assert_eq!(
                actual, expected,
                "Lower bound mismatch for target {target}: expected {expected}, got {actual}"
            );
        }
    }

    #[test]
    fn test_multilevel_range_query() {
        let data: Vec<u64> = vec![10, 20, 30, 40, 50, 60, 70, 80, 90, 100];
        let index = MultilevelPgmIndex::build(data, 2);

        assert_eq!(index.range(25, 65), &[30, 40, 50, 60]);
        assert_eq!(index.range(0, 9), &[]);
        assert_eq!(index.range(101, 200), &[]);
        assert_eq!(
            index.range(10, 100),
            &[10, 20, 30, 40, 50, 60, 70, 80, 90, 100]
        );
    }
}
