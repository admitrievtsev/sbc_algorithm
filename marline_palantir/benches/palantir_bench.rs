use std::collections::HashMap;

use chunkfs::chunkers::{FastChunker, SizeParams};
use chunkfs::{Chunker, DataContainer, Scrub};
use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use marline_palantir::encoder::GdeltaEncoder;
use marline_palantir::lifecycle_manager::LifecycleManager;
use marline_palantir::mock_rocksdb::MockRocksDBMap;
use marline_palantir::palantir_scrubber::PalantirScrubber;
use marline_palantir::sf_generator::PalantirHasher;
use marline_palantir::types::TierConfig;
use sha2::Digest;

type ChunkEntry = (Vec<u8>, Vec<u8>);

fn read_dir_recursive(path: &std::path::Path, buf: &mut Vec<u8>) {
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file() {
                if let Ok(content) = std::fs::read(&p) {
                    buf.extend_from_slice(&content);
                }
            } else if p.is_dir() {
                read_dir_recursive(&p, buf);
            }
        }
    }
}

fn read_kernel_data() -> Vec<u8> {
    let dirs = [
        "/home/mak/RustroverProjects/marline/linux-3.4.5",
        "/home/mak/RustroverProjects/marline/linux-3.4.6",
        "/home/mak/RustroverProjects/marline/linux-3.4.7",
    ];
    let mut all_data = Vec::new();
    for dir in &dirs {
        read_dir_recursive(std::path::Path::new(dir), &mut all_data);
    }
    if all_data.is_empty() {
        eprintln!("WARNING: no kernel data found, using synthetic 100MB");
        all_data = vec![0u8; 100 * 1024 * 1024];
    }
    all_data
}

fn pre_chunk(data: &[u8]) -> Vec<ChunkEntry> {
    let chunk_size = SizeParams::new(8192, 32768, 65536);
    let mut chunker = FastChunker::new(chunk_size);
    let chunks = chunker.chunk_data(data, Vec::new());
    let mut entries = Vec::with_capacity(chunks.len());
    for c in &chunks {
        let chunk_data = data[c.offset()..c.offset() + c.length()].to_vec();
        let hash = sha2::Sha256::digest(&chunk_data).to_vec();
        entries.push((hash, chunk_data));
    }
    entries
}

fn build_db(entries: &[ChunkEntry]) -> HashMap<Vec<u8>, DataContainer<Vec<u8>>> {
    let mut db = HashMap::with_capacity(entries.len());
    for (hash, data) in entries {
        db.insert(hash.clone(), DataContainer::from(data.clone()));
    }
    db
}

fn bench_n1(c: &mut Criterion, entries: &[ChunkEntry]) {
    let mut group = c.benchmark_group("palantir_scrub/N1");

    // N1_G2: 1 tier, group_size=2, features_num=2 (default)
    {
        let tier_array = [2u32];
        let tc = TierConfig::new(tier_array);
        let sf_gen = PalantirHasher::new(7, vec![2]);
        group.bench_with_input("G2", &(), |b, _| {
            b.iter_batched(
                || build_db(entries),
                |mut db| {
                    let mut target = MockRocksDBMap::new();
                    let mut scrubber = PalantirScrubber::new(
                        sf_gen.clone(),
                        GdeltaEncoder,
                        tc.clone(),
                        LifecycleManager::<1>::default_configs(),
                    );
                    scrubber.scrub(&mut db, &mut target).unwrap();
                },
                BatchSize::SmallInput,
            );
        });
    }

    // Odess-like: 1 tier, group_size=2, features_num=12 (override) → 6 SFs
    {
        let tier_array = [2u32];
        let tc = TierConfig::with_features_num(tier_array, 12);
        let sf_gen = PalantirHasher::with_features_num(7, vec![2], 12);
        group.bench_with_input("odess_like", &(), |b, _| {
            b.iter_batched(
                || build_db(entries),
                |mut db| {
                    let mut target = MockRocksDBMap::new();
                    let mut scrubber = PalantirScrubber::new(
                        sf_gen.clone(),
                        GdeltaEncoder,
                        tc.clone(),
                        LifecycleManager::<1>::default_configs(),
                    );
                    scrubber.scrub(&mut db, &mut target).unwrap();
                },
                BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

fn bench_n2(c: &mut Criterion, entries: &[ChunkEntry]) {
    let mut group = c.benchmark_group("palantir_scrub/N2");

    let tier_array = [3u32, 2];
    let tc = TierConfig::new(tier_array);
    let sf_gen = PalantirHasher::new(7, vec![3, 2]);
    group.bench_with_input("G3-2", &(), |b, _| {
        b.iter_batched(
            || build_db(entries),
            |mut db| {
                let mut target = MockRocksDBMap::new();
                let mut scrubber = PalantirScrubber::new(
                    sf_gen.clone(),
                    GdeltaEncoder,
                    tc.clone(),
                    LifecycleManager::<2>::default_configs(),
                );
                scrubber.scrub(&mut db, &mut target).unwrap();
            },
            BatchSize::SmallInput,
        );
    });

    group.finish();
}

fn bench_n3(c: &mut Criterion, entries: &[ChunkEntry]) {
    let mut group = c.benchmark_group("palantir_scrub/N3");

    // G4-3-2: baseline
    {
        let tier_array = [4u32, 3, 2];
        let tc = TierConfig::new(tier_array);
        let sf_gen = PalantirHasher::new(7, vec![4, 3, 2]);
        group.bench_with_input("G4-3-2", &(), |b, _| {
            b.iter_batched(
                || build_db(entries),
                |mut db| {
                    let mut target = MockRocksDBMap::new();
                    let mut scrubber = PalantirScrubber::new(
                        sf_gen.clone(),
                        GdeltaEncoder,
                        tc.clone(),
                        LifecycleManager::<3>::default_configs(),
                    );
                    scrubber.scrub(&mut db, &mut target).unwrap();
                },
                BatchSize::SmallInput,
            );
        });
    }

    // G8-4-2: coarse groups
    {
        let tier_array = [8u32, 4, 2];
        let tc = TierConfig::new(tier_array);
        let sf_gen = PalantirHasher::new(7, vec![8, 4, 2]);
        group.bench_with_input("G8-4-2", &(), |b, _| {
            b.iter_batched(
                || build_db(entries),
                |mut db| {
                    let mut target = MockRocksDBMap::new();
                    let mut scrubber = PalantirScrubber::new(
                        sf_gen.clone(),
                        GdeltaEncoder,
                        tc.clone(),
                        LifecycleManager::<3>::default_configs(),
                    );
                    scrubber.scrub(&mut db, &mut target).unwrap();
                },
                BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

fn bench_n4(c: &mut Criterion, entries: &[ChunkEntry]) {
    let mut group = c.benchmark_group("palantir_scrub/N4");

    let tier_array = [6u32, 4, 3, 2];
    let tc = TierConfig::new(tier_array);
    let sf_gen = PalantirHasher::new(7, vec![6, 4, 3, 2]);
    group.bench_with_input("G6-4-3-2", &(), |b, _| {
        b.iter_batched(
            || build_db(entries),
            |mut db| {
                let mut target = MockRocksDBMap::new();
                let mut scrubber = PalantirScrubber::new(
                    sf_gen.clone(),
                    GdeltaEncoder,
                    tc.clone(),
                    LifecycleManager::<4>::default_configs(),
                );
                scrubber.scrub(&mut db, &mut target).unwrap();
            },
            BatchSize::SmallInput,
        );
    });

    group.finish();
}

fn bench_n5(c: &mut Criterion, entries: &[ChunkEntry]) {
    let mut group = c.benchmark_group("palantir_scrub/N5");

    let tier_array = [12u32, 6, 4, 3, 2];
    let tc = TierConfig::new(tier_array);
    let sf_gen = PalantirHasher::new(7, vec![12, 6, 4, 3, 2]);
    group.bench_with_input("G12-6-4-3-2", &(), |b, _| {
        b.iter_batched(
            || build_db(entries),
            |mut db| {
                let mut target = MockRocksDBMap::new();
                let mut scrubber = PalantirScrubber::new(
                    sf_gen.clone(),
                    GdeltaEncoder,
                    tc.clone(),
                    LifecycleManager::<5>::default_configs(),
                );
                scrubber.scrub(&mut db, &mut target).unwrap();
            },
            BatchSize::SmallInput,
        );
    });

    group.finish();
}

fn bench_all(c: &mut Criterion) {
    let data = read_kernel_data();
    let entries = pre_chunk(&data);
    eprintln!("Pre-chunked: {} chunks, {} MB total", entries.len(), data.len() / 1024 / 1024);

    bench_n1(c, &entries);
    bench_n2(c, &entries);
    bench_n3(c, &entries);
    bench_n4(c, &entries);
    bench_n5(c, &entries);
}

criterion_group!(benches, bench_all);
criterion_main!(benches);
