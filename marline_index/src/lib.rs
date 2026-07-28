//! A sketch-based similarity search index.
//!
//! `marline_index` provides data structures and algorithms for building
//! key-value indexes that support nearest-neighbor search via sketches.
//! A sketch is a compact, fixed-size representation of an item's content
//! that preserves similarity information.
//!
//! # Overview
//!
//! - [`sketch`]: Defines the [`Sketch`] trait and [`FixedSketch<F, N>`]
//!   implementation for representing fixed-size feature sets.
//! - [`index`]: Contains the [`SketchIndexApi`] trait and the
//!   [`InvertedSketchIndex`] implementation.
//! - [`heuristic_index`]: Contains the [`HeuristicInvertedSketchIndex`] with
//!   heuristic search support.
//!
//! # Quick Start
//!
//! ```rust
//! use marline_index::index::store::IndexStorage;
//! use marline_index::index::{InvertedSketchIndex, SketchIndexApi};
//! use marline_index::sketch::U32Sketch;
//!
//! let storage = IndexStorage::new();
//! let idx = InvertedSketchIndex::new(storage);
//!
//! let sk = U32Sketch::<6>::new([1, 2, 3, 4, 5, 6]);
//! idx.put(&42_u64, sk).unwrap();
//! ```
//!
//! [`Sketch`]: sketch::Sketch
//! [`FixedSketch<F, N>`]: sketch::FixedSketch
//! [`SketchIndexApi`]: index::SketchIndexApi
//! [`InvertedSketchIndex`]: index::InvertedSketchIndex
//! [`HeuristicInvertedSketchIndex`]: heuristic_index::HeuristicInvertedSketchIndex

pub mod heuristic_index;
pub mod index;
pub mod simple_storage;
pub mod sketch;
