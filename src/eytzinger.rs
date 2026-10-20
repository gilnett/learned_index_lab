// Copyright (c) Learned Index
// Implementation of cache-conscious Eytzinger (breadth-first) binary search layout
// Reference: Khuong & Morin (2017) "Array Layouts for Comparison-Based Searching"

/// Cache-conscious static search structure using the Eytzinger (breadth-first) array layout.
///
/// Reorganizes a sorted slice of keys into a complete binary tree stored in level order:
/// - The root node resides at 1-based index 1.
/// - The left child of node `k` is at index `2k`.
/// - The right child of node `k` is at index `2k + 1`.
///
/// ### Microarchitectural Advantages
/// - **Cache Locality**: Upper levels of the binary tree reside permanently in L1/L2 cache lines.
/// - **Branchless Traversal**: Search proceeds with arithmetic conditional steps:
///   `k = 2 * k + (target > tree[k])`
///   compiled into conditional move (`cmov`) instructions, eliminating branch mispredictions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EytzingerSearch {
    tree: Vec<u64>,
}

impl EytzingerSearch {
    /// Builds an Eytzinger search layout from pre-sorted data.
    ///
    /// # Panics
    /// In debug mode, panics if the input slice is not sorted.
    ///
    /// # Example
    /// ```rust
    /// use learned_index::EytzingerSearch;
    ///
    /// let data = vec![10, 20, 30, 40, 50, 60, 70];
    /// let eyt = EytzingerSearch::build(&data);
    /// assert!(eyt.contains(40));
    /// assert!(!eyt.contains(99));
    /// ```
    pub fn build(sorted: &[u64]) -> Self {
        debug_assert!(
            sorted.windows(2).all(|w| w[0] <= w[1]),
            "EytzingerSearch requires pre-sorted data"
        );

        let n = sorted.len();
        if n == 0 {
            return Self { tree: Vec::new() };
        }

        let mut tree = vec![0u64; n + 1];
        let mut in_order_idx = 0;

        fn inorder(k: usize, n: usize, in_order_idx: &mut usize, sorted: &[u64], tree: &mut [u64]) {
            if k <= n {
                inorder(2 * k, n, in_order_idx, sorted, tree);
                tree[k] = sorted[*in_order_idx];
                *in_order_idx += 1;
                inorder(2 * k + 1, n, in_order_idx, sorted, tree);
            }
        }

        inorder(1, n, &mut in_order_idx, sorted, &mut tree);
        Self { tree }
    }

    /// Checks if the target key is present in the layout using branchless search.
    #[inline]
    pub fn contains(&self, target: u64) -> bool {
        self.find(target).is_some()
    }

    /// Performs a branchless search for `target`.
    ///
    /// Returns the 1-based index within the Eytzinger tree if found, or `None` if absent.
    #[inline]
    pub fn find(&self, target: u64) -> Option<usize> {
        let n = self.len();
        if n == 0 {
            return None;
        }

        let mut k = 1;
        while k <= n {
            let bit = (target > self.tree[k]) as usize;
            k = 2 * k + bit;
        }

        let res_idx = k >> (k.trailing_ones() + 1);
        if res_idx > 0 && self.tree[res_idx] == target {
            Some(res_idx)
        } else {
            None
        }
    }

    /// Finds the first element in the tree that is greater than or equal to `target` (lower bound).
    ///
    /// Returns `Some(value)` if found, or `None` if `target` is greater than all elements.
    #[inline]
    pub fn lower_bound(&self, target: u64) -> Option<u64> {
        let n = self.len();
        if n == 0 {
            return None;
        }

        let mut k = 1;
        while k <= n {
            let bit = (target > self.tree[k]) as usize;
            k = 2 * k + bit;
        }

        let res_idx = k >> (k.trailing_ones() + 1);
        if res_idx > 0 {
            Some(self.tree[res_idx])
        } else {
            None
        }
    }

    /// Returns the number of elements in the search structure.
    #[inline]
    pub fn len(&self) -> usize {
        self.tree.len().saturating_sub(1)
    }

    /// Returns true if the search structure contains no elements.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the total heap allocation in bytes.
    #[inline]
    pub fn memory_bytes(&self) -> usize {
        self.tree.capacity() * std::mem::size_of::<u64>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_and_single() {
        let empty = EytzingerSearch::build(&[]);
        assert_eq!(empty.len(), 0);
        assert!(empty.is_empty());
        assert_eq!(empty.find(10), None);
        assert_eq!(empty.lower_bound(10), None);

        let single = EytzingerSearch::build(&[42]);
        assert_eq!(single.len(), 1);
        assert!(!single.is_empty());
        assert!(single.contains(42));
        assert!(!single.contains(41));
        assert!(!single.contains(43));
        assert_eq!(single.lower_bound(40), Some(42));
        assert_eq!(single.lower_bound(42), Some(42));
        assert_eq!(single.lower_bound(43), None);
    }

    #[test]
    fn test_find_and_parity_across_sizes() {
        for n in 1..=65 {
            let sorted: Vec<u64> = (0..n).map(|x| (x * 3 + 5) as u64).collect();
            let eyt = EytzingerSearch::build(&sorted);
            assert_eq!(eyt.len(), n);

            // Existing keys
            for &k in &sorted {
                assert!(eyt.contains(k), "Missing key {} in size {}", k, n);
                assert_eq!(eyt.lower_bound(k), Some(k));
            }

            // Gaps and boundaries
            let max_val = *sorted.last().unwrap();
            for probe in 0..=(max_val + 5) {
                let expected_find = sorted.binary_search(&probe).is_ok();
                assert_eq!(
                    eyt.contains(probe),
                    expected_find,
                    "Mismatch probe {} in size {}",
                    probe,
                    n
                );

                let expected_lb = sorted.partition_point(|&x| x < probe);
                let expected_lb_val = sorted.get(expected_lb).copied();
                assert_eq!(
                    eyt.lower_bound(probe),
                    expected_lb_val,
                    "LB mismatch probe {} in size {}",
                    probe,
                    n
                );
            }
        }
    }

    #[test]
    fn test_memory_accounting() {
        let data = vec![1, 2, 3, 4, 5, 6, 7];
        let eyt = EytzingerSearch::build(&data);
        assert!(eyt.memory_bytes() >= (data.len() + 1) * std::mem::size_of::<u64>());
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "EytzingerSearch requires pre-sorted data")]
    fn test_unsorted_panics_in_debug() {
        let _ = EytzingerSearch::build(&[3, 1, 2]);
    }
}
