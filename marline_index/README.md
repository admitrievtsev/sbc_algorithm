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
let sketch = U32Sketch::<6>::new([1, 2, 3, 4, 5, 6]).unwrap();

index.put(&42_u64, sketch).unwrap();
assert_eq!(index.get(&sketch).unwrap(), Some(42));
```

`FixedSketch::new` sorts its input and returns an error for zero-sized sketches.
`SimilarityScore` values can only be created through validated constructors.

`put` inserts a new key and does not update existing entries. `remove` accepts
only the key; the in-memory storage keeps a reverse map solely to locate the
features that must be removed from posting lists.

## Main API

- `sketch::FixedSketch` stores sorted, fixed-size feature sets.
- `index::InvertedSketchIndex` provides posting-list-backed similarity search.
- `simple_storage::LinearSearchIndex` provides a linear-search baseline.

The crate currently provides in-memory storage only; persistence and recovery
are outside its scope.
