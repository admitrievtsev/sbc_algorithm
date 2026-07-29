use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;

use chunkfs::chunkers::{FastChunker, SizeParams};
use chunkfs::hashers::Sha256Hasher;
use chunkfs::{Chunker, DataContainer, FileSystem, Hasher};
use marline_palantir::encoder::GdeltaEncoder;
use marline_palantir::lifecycle_manager::LifecycleManager;
use marline_palantir::mock_rocksdb::MockRocksDBMap;
use marline_palantir::palantir_scrubber::PalantirScrubber;
use marline_palantir::sf_generator::PalantirHasher;
use marline_palantir::types::TierConfig;

struct Config {
    name: &'static str,
    tier_list: Vec<u32>,
    features_num_override: Option<usize>,
}

fn configs() -> Vec<Config> {
    vec![
        Config { name: "N1_G2",            tier_list: vec![2],      features_num_override: None },
        Config { name: "ODESS",    tier_list: vec![6],      features_num_override: Some(12) },
        Config { name: "N2_G3-2",          tier_list: vec![3, 2],   features_num_override: None },
        Config { name: "N3_G4-3-2",        tier_list: vec![4, 3, 2], features_num_override: None },
        Config { name: "N3_G8-4-2",        tier_list: vec![8, 4, 2], features_num_override: None },
        Config { name: "N4_G6-4-3-2",      tier_list: vec![6, 4, 3, 2], features_num_override: None },
        Config { name: "N5_G12-6-4-3-2",   tier_list: vec![12, 6, 4, 3, 2], features_num_override: None },
    ]
}

fn collect_files(dir: &Path, files: &mut Vec<Vec<u8>>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_file() {
                if let Ok(content) = std::fs::read(&p) {
                    files.push(content);
                }
            } else if p.is_dir() {
                collect_files(&p, files);
            }
        }
    }
}

fn track_cdc_chunks(
    data: &[u8],
    chunk_size: SizeParams,
    cdc_sizes: &mut HashMap<[u8; 32], usize>,
    hasher: &mut Sha256Hasher,
) {
    let mut chunker = FastChunker::new(chunk_size);
    let chunks = chunker.chunk_data(data, Vec::new());
    for c in &chunks {
        let chunk_data = &data[c.offset()..c.offset() + c.length()];
        let hash = hasher.hash(chunk_data);
        cdc_sizes.entry(hash).or_insert(c.length());
    }
}

fn run_metrics(
    name: &str,
    scrubber: impl chunkfs::Scrub<[u8; 32], HashMap<[u8; 32], DataContainer<[u8; 32]>>, [u8; 32], MockRocksDBMap> + 'static,
    kernel_files: &[Vec<Vec<u8>>],
    kernel_labels: &[&str],
) {
    let database: HashMap<[u8; 32], DataContainer<[u8; 32]>> = HashMap::default();
    let target_map = MockRocksDBMap::new();
    let hasher = Sha256Hasher::default();

    let mut fs = FileSystem::new_with_scrubber(
        database,
        target_map,
        Box::new(scrubber),
        hasher,
    );

    let chunk_size = SizeParams::new(8192, 32768, 65536);
    let mut original_total = 0u64;
    let total_start = Instant::now();
    let mut file_id = 0u64;
    let mut cdc_sizes: HashMap<[u8; 32], usize> = HashMap::new();

    for (idx, files) in kernel_files.iter().enumerate() {
        let kernel_orig: usize = files.iter().map(|d| d.len()).sum();
        original_total += kernel_orig as u64;

        for data in files {
            let mut inner_hasher = Sha256Hasher::default();
            track_cdc_chunks(data, chunk_size, &mut cdc_sizes, &mut inner_hasher);

            let write_chunker = FastChunker::new(chunk_size);
            let mut handle = fs.create_file(format!("f{}", file_id), write_chunker).unwrap();
            fs.write_to_file(&mut handle, data).unwrap();
            fs.close_file(handle).unwrap();
            file_id += 1;
        }

        let cdc_stored: usize = cdc_sizes.values().sum();
        let cdc_ratio = original_total as f64 / cdc_stored as f64;

        fs.scrub().unwrap();

        let total_ratio = fs.total_dedup_ratio();
        let orig_mb = original_total as f64 / (1024.0 * 1024.0);
        let stored_mb = if total_ratio > 0.0 { orig_mb / total_ratio } else { 0.0 };
        let total_elapsed = total_start.elapsed();

        println!(
            "{:<20} {:<8} cdc={:<7.4} total_dedup={:<7.4} stored={:<9.3} orig={:<9.3} elapsed={:<5.2}",
            if idx == 0 { name } else { "" },
            kernel_labels[idx],
            cdc_ratio,
            total_ratio,
            stored_mb,
            orig_mb,
            total_elapsed.as_secs_f64(),
        );
    }
    println!("\n")
}

fn main() {
    let kernel_dirs = [
        "/home/mak/RustroverProjects/marline/linux-3.4.5",
        "/home/mak/RustroverProjects/marline/linux-3.4.6",
        "/home/mak/RustroverProjects/marline/linux-3.4.7",
    ];

    let kernel_labels = ["3.4.5", "3.4.6", "3.4.7"];

    let mut kernel_files: Vec<Vec<Vec<u8>>> = Vec::new();
    for dir in &kernel_dirs {
        let mut files = Vec::new();
        collect_files(Path::new(dir), &mut files);
        eprintln!("  {}: {} files, {:.2} MB", dir, files.len(), files.iter().map(|d| d.len()).sum::<usize>() as f64 / (1024.0 * 1024.0));
        kernel_files.push(files);
    }

    println!(
        "{:<20} {:<8} {:<12} {:<20} {:<15} {:<15} {:<9}",
        "config", "kernel", "cdc_ratio", "total_dedup", "stored_mb", "orig_mb", "elapsed"
    );

    for cfg in configs() {
        let sf_gen = if let Some(fn_val) = cfg.features_num_override {
            PalantirHasher::with_features_num(7, cfg.tier_list.clone(), fn_val)
        } else {
            PalantirHasher::new(7, cfg.tier_list.clone())
        };

        match cfg.tier_list.len() {
            1 => {
                let arr: [u32; 1] = cfg.tier_list.as_slice().try_into().unwrap();
                let tier_config = if let Some(fn_val) = cfg.features_num_override {
                    TierConfig::with_features_num(arr, fn_val)
                } else {
                    TierConfig::new(arr)
                };
                let scrubber = PalantirScrubber::new(
                    sf_gen, GdeltaEncoder, tier_config,
                    LifecycleManager::<1>::default_configs(),
                );
                run_metrics(cfg.name, scrubber, &kernel_files, &kernel_labels);
            }
            2 => {
                let arr: [u32; 2] = cfg.tier_list.as_slice().try_into().unwrap();
                let tier_config = TierConfig::new(arr);
                let scrubber = PalantirScrubber::new(
                    sf_gen, GdeltaEncoder, tier_config,
                    LifecycleManager::<2>::default_configs(),
                );
                run_metrics(cfg.name, scrubber, &kernel_files, &kernel_labels);
            }
            3 => {
                let arr: [u32; 3] = cfg.tier_list.as_slice().try_into().unwrap();
                let tier_config = TierConfig::new(arr);
                let scrubber = PalantirScrubber::new(
                    sf_gen, GdeltaEncoder, tier_config,
                    LifecycleManager::<3>::default_configs(),
                );
                run_metrics(cfg.name, scrubber, &kernel_files, &kernel_labels);
            }
            4 => {
                let arr: [u32; 4] = cfg.tier_list.as_slice().try_into().unwrap();
                let tier_config = TierConfig::new(arr);
                let scrubber = PalantirScrubber::new(
                    sf_gen, GdeltaEncoder, tier_config,
                    LifecycleManager::<4>::default_configs(),
                );
                run_metrics(cfg.name, scrubber, &kernel_files, &kernel_labels);
            }
            5 => {
                let arr: [u32; 5] = cfg.tier_list.as_slice().try_into().unwrap();
                let tier_config = TierConfig::new(arr);
                let scrubber = PalantirScrubber::new(
                    sf_gen, GdeltaEncoder, tier_config,
                    LifecycleManager::<5>::default_configs(),
                );
                run_metrics(cfg.name, scrubber, &kernel_files, &kernel_labels);
            }
            _ => unreachable!(),
        }
    }
}
