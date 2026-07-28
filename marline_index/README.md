# marline_index

`marline_index` is a Rust library for similarity search over fixed-size
sketches. It provides an in-memory inverted index and a simple linear-search
implementation for comparison and testing.

## Quick Start

```rust
use marline_index::index::store::IndexStorage;
use marline_index::index::{InvertedSketchIndex, SketchIndexApi};
use marline_index::sketch::U32Sketch;

let storage = IndexStorage::new();
let index = InvertedSketchIndex::new(storage);
let sketch = U32Sketch::<6>::new([1, 2, 3, 4, 5, 6]);

index.put(&42_u64, sketch).unwrap();
assert_eq!(index.get(&sketch).unwrap(), Some(42));
```

`FixedSketch::new` sorts its input and panics for zero-sized sketches.
`SimilarityScore` values can only be created through validated constructors.

`put` replaces an existing key and `remove` deletes it. The in-memory inverted
storage removes a key by scanning all posting lists, so removal and overwrite
are intended for relatively infrequent operations.

## Main API

- `sketch::FixedSketch` stores sorted, fixed-size feature sets.
- `index::InvertedSketchIndex` provides posting-list-backed similarity search.
- `simple_storage::LinearSearchIndex` provides a linear-search baseline.

The crate currently provides in-memory storage only; persistence and recovery
are outside its scope.
