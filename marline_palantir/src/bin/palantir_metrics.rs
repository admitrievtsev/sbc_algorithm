use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;

use chunkfs::chunkers::{FastChunker, SizeParams};
use chunkfs::{Chunker, DataContainer, Scrub};
use marline_palantir::encoder::GdeltaEncoder;
use marline_palantir::lifecycle_manager::LifecycleManager;
use marline_palantir::mock_rocksdb::MockRocksDBMap;
use marline_palantir::palantir_scrubber::PalantirScrubber;
use marline_palantir::sf_generator::PalantirHasher;
use marline_palantir::types::TierConfig;
use sha2::Digest;

struct Config {
    name: &'static str,
    tier_list: Vec<u32>,
    features_num_override: Option<usize>,
}

fn configs() -> Vec<Config> {
    vec![
        Config { name: "N1_G2",            tier_list: vec![2],      features_num_override: None },
        Config { name: "N1_odess_like",    tier_list: vec![2],      features_num_override: Some(12) },
        Config { name: "N2_G3-2",          tier_list: vec![3, 2],   features_num_override: None },
        Config { name: "N3_G4-3-2",        tier_list: vec![4, 3, 2], features_num_override: None },
        Config { name: "N3_G8-4-2",        tier_list: vec![8, 4, 2], features_num_override: None },
        Config { name: "N4_G6-4-3-2",      tier_list: vec![6, 4, 3, 2], features_num_override: None },
        Config { name: "N5_G12-6-4-3-2",   tier_list: vec![12, 6, 4, 3, 2], features_num_override: None },
    ]
}

fn read_dir_into_buf(path: &Path, buf: &mut Vec<u8>) {
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file() {
                if let Ok(content) = std::fs::read(&p) {
                    buf.extend_from_slice(&content);
                }
            } else if p.is_dir() {
                read_dir_into_buf(&p, buf);
            }
        }
    }
}

struct KernelBlob {
    data: Vec<u8>,
}

fn run_metrics(
    name: &str,
    tier_list: &[u32],
    features_num_override: Option<usize>,
    kernels: &[KernelBlob],
) {
    let chunk_size = SizeParams::new(8192, 32768, 65536);
    let mut target = MockRocksDBMap::new();
    let mut original_total = 0u64;

    let start = Instant::now();
    let (delta_stored, fp_size, sf_size) = match tier_list.len() {
        1 => {
            let arr: [u32; 1] = tier_list.try_into().unwrap();
            let tc = if let Some(fn_val) = features_num_override {
                TierConfig::with_features_num(arr, fn_val)
            } else {
                TierConfig::new(arr)
            };
            let sf_gen = if let Some(fn_val) = features_num_override {
                PalantirHasher::with_features_num(7, tier_list.to_vec(), fn_val)
            } else {
                PalantirHasher::new(7, tier_list.to_vec())
            };
            let mut scrubber = PalantirScrubber::new(
                sf_gen, GdeltaEncoder, tc,
                LifecycleManager::<1>::default_configs(),
            );
            for kb in kernels {
                let mut chunker = FastChunker::new(chunk_size);
                let chunks = chunker.chunk_data(&kb.data, Vec::new());
                if chunks.is_empty() { continue; }
                let mut db: HashMap<Vec<u8>, DataContainer<Vec<u8>>> =
                    HashMap::with_capacity(chunks.len());
                for c in &chunks {
                    let chunk_data = kb.data[c.offset()..c.offset() + c.length()].to_vec();
                    let hash = sha2::Sha256::digest(&chunk_data).to_vec();
                    original_total += chunk_data.len() as u64;
                    db.insert(hash, DataContainer::from(chunk_data));
                }
                scrubber.scrub(&mut db, &mut target).unwrap();
            }
            (scrubber.delta_stored(), scrubber.fp_table_size(), scrubber.sf_table_size())
        }
        2 => {
            let arr: [u32; 2] = tier_list.try_into().unwrap();
            let tc = TierConfig::new(arr);
            let sf_gen = PalantirHasher::new(7, tier_list.to_vec());
            let mut scrubber = PalantirScrubber::new(
                sf_gen, GdeltaEncoder, tc,
                LifecycleManager::<2>::default_configs(),
            );
            for kb in kernels {
                let mut chunker = FastChunker::new(chunk_size);
                let chunks = chunker.chunk_data(&kb.data, Vec::new());
                if chunks.is_empty() { continue; }
                let mut db: HashMap<Vec<u8>, DataContainer<Vec<u8>>> =
                    HashMap::with_capacity(chunks.len());
                for c in &chunks {
                    let chunk_data = kb.data[c.offset()..c.offset() + c.length()].to_vec();
                    let hash = sha2::Sha256::digest(&chunk_data).to_vec();
                    original_total += chunk_data.len() as u64;
                    db.insert(hash, DataContainer::from(chunk_data));
                }
                scrubber.scrub(&mut db, &mut target).unwrap();
            }
            (scrubber.delta_stored(), scrubber.fp_table_size(), scrubber.sf_table_size())
        }
        3 => {
            let arr: [u32; 3] = tier_list.try_into().unwrap();
            let tc = TierConfig::new(arr);
            let sf_gen = PalantirHasher::new(7, tier_list.to_vec());
            let mut scrubber = PalantirScrubber::new(
                sf_gen, GdeltaEncoder, tc,
                LifecycleManager::<3>::default_configs(),
            );
            for kb in kernels {
                let mut chunker = FastChunker::new(chunk_size);
                let chunks = chunker.chunk_data(&kb.data, Vec::new());
                if chunks.is_empty() { continue; }
                let mut db: HashMap<Vec<u8>, DataContainer<Vec<u8>>> =
                    HashMap::with_capacity(chunks.len());
                for c in &chunks {
                    let chunk_data = kb.data[c.offset()..c.offset() + c.length()].to_vec();
                    let hash = sha2::Sha256::digest(&chunk_data).to_vec();
                    original_total += chunk_data.len() as u64;
                    db.insert(hash, DataContainer::from(chunk_data));
                }
                scrubber.scrub(&mut db, &mut target).unwrap();
            }
            (scrubber.delta_stored(), scrubber.fp_table_size(), scrubber.sf_table_size())
        }
        4 => {
            let arr: [u32; 4] = tier_list.try_into().unwrap();
            let tc = TierConfig::new(arr);
            let sf_gen = PalantirHasher::new(7, tier_list.to_vec());
            let mut scrubber = PalantirScrubber::new(
                sf_gen, GdeltaEncoder, tc,
                LifecycleManager::<4>::default_configs(),
            );
            for kb in kernels {
                let mut chunker = FastChunker::new(chunk_size);
                let chunks = chunker.chunk_data(&kb.data, Vec::new());
                if chunks.is_empty() { continue; }
                let mut db: HashMap<Vec<u8>, DataContainer<Vec<u8>>> =
                    HashMap::with_capacity(chunks.len());
                for c in &chunks {
                    let chunk_data = kb.data[c.offset()..c.offset() + c.length()].to_vec();
                    let hash = sha2::Sha256::digest(&chunk_data).to_vec();
                    original_total += chunk_data.len() as u64;
                    db.insert(hash, DataContainer::from(chunk_data));
                }
                scrubber.scrub(&mut db, &mut target).unwrap();
            }
            (scrubber.delta_stored(), scrubber.fp_table_size(), scrubber.sf_table_size())
        }
        5 => {
            let arr: [u32; 5] = tier_list.try_into().unwrap();
            let tc = TierConfig::new(arr);
            let sf_gen = PalantirHasher::new(7, tier_list.to_vec());
            let mut scrubber = PalantirScrubber::new(
                sf_gen, GdeltaEncoder, tc,
                LifecycleManager::<5>::default_configs(),
            );
            for kb in kernels {
                let mut chunker = FastChunker::new(chunk_size);
                let chunks = chunker.chunk_data(&kb.data, Vec::new());
                if chunks.is_empty() { continue; }
                let mut db: HashMap<Vec<u8>, DataContainer<Vec<u8>>> =
                    HashMap::with_capacity(chunks.len());
                for c in &chunks {
                    let chunk_data = kb.data[c.offset()..c.offset() + c.length()].to_vec();
                    let hash = sha2::Sha256::digest(&chunk_data).to_vec();
                    original_total += chunk_data.len() as u64;
                    db.insert(hash, DataContainer::from(chunk_data));
                }
                scrubber.scrub(&mut db, &mut target).unwrap();
            }
            (scrubber.delta_stored(), scrubber.fp_table_size(), scrubber.sf_table_size())
        }
        _ => unreachable!(),
    };

    let elapsed = start.elapsed();
    let stored_total = target.total_bytes() as u64;
    let chunks_total = target.len();
    let dedup_ratio = if original_total > 0 {
        stored_total as f64 / original_total as f64
    } else {
        1.0
    };
    let throughput_mbps = if elapsed.as_secs_f64() > 0.0 {
        (original_total as f64 / elapsed.as_secs_f64()) / (1024.0 * 1024.0)
    } else {
        0.0
    };

    println!(
        "{:<20} chunks={:<7} dedup_ratio={:<10.6} stored_mb={:<10.3} orig_mb={:<10.3} delta_cnt={:<6} fp_size={:<8} sf_size={:<8} elapsed_s={:<10.2} throughput_mbps={:.2}",
        name,
        chunks_total,
        dedup_ratio,
        stored_total as f64 / (1024.0 * 1024.0),
        original_total as f64 / (1024.0 * 1024.0),
        delta_stored,
        fp_size,
        sf_size,
        elapsed.as_secs_f64(),
        throughput_mbps,
    );
}

fn main() {
    let kernel_dirs = [
        "/home/mak/RustroverProjects/marline/linux-3.4.5",
        "/home/mak/RustroverProjects/marline/linux-3.4.6",
        "/home/mak/RustroverProjects/marline/linux-3.4.7",
    ];

    let mut kernels: Vec<KernelBlob> = Vec::new();
    for dir in &kernel_dirs {
        let mut data = Vec::new();
        read_dir_into_buf(Path::new(dir), &mut data);
        eprintln!("  {}: {:.2} MB", dir, data.len() as f64 / (1024.0 * 1024.0));
        kernels.push(KernelBlob { data });
    }
    let total_mb: f64 = kernels.iter().map(|k| k.data.len() as f64 / (1024.0 * 1024.0)).sum();
    eprintln!("Total: {:.2} MB across 3 kernels", total_mb);

    println!(
        "{:<20} {:<8} {:<13} {:<11} {:<11} {:<9} {:<9} {:<9} {:<12} {:<12}",
        "config", "chunks", "dedup_ratio", "stored_mb", "orig_mb",
        "delta_cnt", "fp_size", "sf_size", "elapsed_s", "mbps"
    );

    for cfg in configs() {
        run_metrics(cfg.name, &cfg.tier_list, cfg.features_num_override, &kernels);
    }
}
