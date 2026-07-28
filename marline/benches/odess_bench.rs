use std::collections::HashMap;

use chunkfs::chunkers::{FastChunker, SizeParams};
use chunkfs::hashers::Sha256Hasher;
use chunkfs::FileSystem;
use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use marline_scrub::encoder::GdeltaEncoder;
use marline_scrub::{clusterer, decoder};
use marline_scrub::{SBCMap, SBCScrubber};
use marline_sketcher::OdessHasher;

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
        eprintln!("WARNING: no kernel data found, using synthetic 10MB");
        all_data = vec![0u8; 10 * 1024 * 1024];
    }
    all_data
}

fn bench_odess(c: &mut Criterion) {
    let data = read_kernel_data();
    eprintln!(
        "Odess bench: {} MB",
        data.len() / 1024 / 1024
    );

    let chunk_size = SizeParams::new(16 * 1024, 32 * 1024, 64 * 1024);

    c.bench_function("odess_scrub/full", |b| {
        b.iter_batched(
            || {
                let mut fs = FileSystem::new_with_scrubber(
                    HashMap::default(),
                    SBCMap::new(decoder::GdeltaDecoder::new(false)),
                    Box::new(SBCScrubber::new(
                        OdessHasher::default(),
                        clusterer::EqClusterer::new(6),
                        GdeltaEncoder::new(false),
                    )),
                    Sha256Hasher::default(),
                );
                let mut handle = fs
                    .create_file("file".to_string(), FastChunker::new(chunk_size))
                    .unwrap();
                fs.write_to_file(&mut handle, &data).unwrap();
                fs.close_file(handle).unwrap();
                fs
            },
            |mut fs| {
                fs.scrub().unwrap();
            },
            BatchSize::LargeInput,
        );
    });
}

criterion_group!(benches, bench_odess);
criterion_main!(benches);
