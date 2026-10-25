//! # Learned Index: Piecewise Geometric Model (PGM-Index)
//!
//! A zero-dependency, pure Rust implementation of Piecewise Geometric Model (PGM)
//! Learned Index Structures with provable epsilon error guarantees.
//!
//! ## Quickstart
//!
//! ```rust
//! use learned_index_lab::PgmIndex;
//!
//! let data = vec![10, 20, 30, 40, 50, 60, 70, 80, 90, 100];
//! let pgm = PgmIndex::build(data, 16);
//!
//! assert_eq!(pgm.find(50), Some(4));
//! assert_eq!(pgm.find(99), None);
//! ```

pub mod eytzinger;
pub mod multilevel;
pub mod pgm;

pub use eytzinger::EytzingerSearch;
pub use multilevel::MultilevelPgmIndex;
pub use pgm::{PgmIndex, Segment};
