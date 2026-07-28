use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use marline_index::{
    index::{store::IndexStorage, InvertedSketchIndex, SketchIndexApi},
    simple_storage::LinearSearchIndex,
    sketch::U32Sketch,
};

type BenchIndex<const N: usize> = InvertedSketchIndex<u64, U32Sketch<N>, IndexStorage<u64, u32>>;
type BenchLinearIndex<const N: usize> = LinearSearchIndex<u64, U32Sketch<N>>;

fn make_sketch<const N: usize>(seed: u32) -> U32Sketch<N> {
    let mut items = [0u32; N];
    for i in 0..N {
        items[i] = seed.wrapping_mul(31).wrapping_add(i as u32 * 17);
    }
    U32Sketch::new(items).unwrap()
}

fn fill_index<const N: usize>(count: usize) -> BenchIndex<N> {
    let index = BenchIndex::<N>::new(IndexStorage::new());
    for i in 0..count {
        let sketch = make_sketch::<N>(i as u32);
        index.put(&(i as u64), sketch).unwrap();
    }
    index
}

fn fill_index_linear<const N: usize>(count: usize) -> BenchLinearIndex<N> {
    let index = BenchLinearIndex::<N>::new();
    for i in 0..count {
        let sketch = make_sketch::<N>(i as u32);
        index.put(&(i as u64), sketch).unwrap();
    }
    index
}

fn bench_top_k<const N: usize>(c: &mut Criterion, group_name: &str) {
    let mut group = c.benchmark_group(group_name);
    let sizes = [100, 1_000, 10_000, 100_000];
    let ks = [1, 3, 5, 10, 20, 50, 100];
    for &size in &sizes {
        let index = fill_index::<N>(size);
        let query = make_sketch::<N>(999_999);
        for &k in &ks {
            let id = BenchmarkId::new(format!("size_{size}_k_{k}"), N);
            group.bench_with_input(id, &query, |b, q| {
                b.iter(|| index.top_k(q, k).unwrap());
            });
        }
    }
    group.finish();
}

fn bench_top_k_linear<const N: usize>(c: &mut Criterion, group_name: &str) {
    let mut group = c.benchmark_group(group_name);
    let sizes = [100, 1_000, 10_000, 100_000];
    let ks = [1, 3, 5, 10, 20, 50, 100];
    for &size in &sizes {
        let index = fill_index_linear::<N>(size);
        let query = make_sketch::<N>(999_999);
        for &k in &ks {
            let id = BenchmarkId::new(format!("size_{size}_k_{k}"), N);
            group.bench_with_input(id, &query, |b, q| {
                b.iter(|| index.top_k(q, k).unwrap());
            });
        }
    }
    group.finish();
}

fn bench_top_k_3(c: &mut Criterion) {
    bench_top_k::<3>(c, "top_k/N=3");
}
fn bench_top_k_4(c: &mut Criterion) {
    bench_top_k::<4>(c, "top_k/N=4");
}

fn bench_top_k_6(c: &mut Criterion) {
    bench_top_k::<6>(c, "top_k/N=6");
}

fn bench_top_k_12(c: &mut Criterion) {
    bench_top_k::<12>(c, "top_k/N=12");
}

fn bench_top_k_linear_3(c: &mut Criterion) {
    bench_top_k_linear::<3>(c, "top_k_linear/N=3");
}
fn bench_top_k_linear_4(c: &mut Criterion) {
    bench_top_k_linear::<4>(c, "top_k_linear/N=4");
}
fn bench_top_k_linear_6(c: &mut Criterion) {
    bench_top_k_linear::<6>(c, "top_k_linear/N=6");
}
fn bench_top_k_linear_12(c: &mut Criterion) {
    bench_top_k_linear::<12>(c, "top_k_linear/N=12");
}

criterion_group!(
    benches,
    bench_top_k_3,
    bench_top_k_4,
    bench_top_k_6,
    bench_top_k_12,
    bench_top_k_linear_3,
    bench_top_k_linear_4,
    bench_top_k_linear_6,
    bench_top_k_linear_12,
);

criterion_main!(benches);
