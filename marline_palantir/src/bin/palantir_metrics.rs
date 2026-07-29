use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;

use chunkfs::chunkers::{FastChunker, SizeParams};
use chunkfs::hashers::Sha256Hasher;
use chunkfs::{DataContainer, FileSystem};
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
        Config { name: "ODESS", tier_list: vec![2], features_num_override: Some(12) },
        Config { name: "N2_G3-2", tier_list: vec![3, 2], features_num_override: None },
        Config { name: "N3_G4-3-2", tier_list: vec![4, 3, 2], features_num_override: None },
        Config { name: "N3_G8-4-2", tier_list: vec![8, 4, 2], features_num_override: None },
        Config { name: "N4_G6-4-3-2", tier_list: vec![6, 4, 3, 2], features_num_override: None },
        Config {
            name: "N5_G12-6-4-3-2",
            tier_list: vec![12, 6, 4, 3, 2],
            features_num_override: None,
        },
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

fn run_metrics(
    name: &str,
    scrubber: impl chunkfs::Scrub<
            [u8; 32],
            HashMap<[u8; 32], DataContainer<[u8; 32]>>,
            [u8; 32],
            MockRocksDBMap,
        > + 'static,
    kernel_files: &[Vec<Vec<u8>>],
) {
    let database: HashMap<[u8; 32], DataContainer<[u8; 32]>> = HashMap::default();
    let target_map = MockRocksDBMap::new();
    let hasher = Sha256Hasher::default();

    let mut fs = FileSystem::new_with_scrubber(database, target_map, Box::new(scrubber), hasher);

    let chunk_size = SizeParams::new(8192, 32768, 65536);
    let original_total: usize = kernel_files.iter().flat_map(|f| f.iter()).map(|d| d.len()).sum();
    let start = Instant::now();

    let mut file_id = 0u64;
    for files in kernel_files {
        for data in files {
            let chunker = FastChunker::new(chunk_size);
            let mut handle = fs.create_file(format!("f{}", file_id), chunker).unwrap();
            fs.write_to_file(&mut handle, data).unwrap();
            fs.close_file(handle).unwrap();
            file_id += 1;
        }
    }

    let cdc_ratio = fs.cdc_dedup_ratio();
    fs.scrub().unwrap();
    let total_ratio = fs.total_dedup_ratio();
    let elapsed = start.elapsed();

    let stored_mb = (original_total as f64 / total_ratio) / (1024.0 * 1024.0);
    let orig_mb = original_total as f64 / (1024.0 * 1024.0);
    let mbps = if elapsed.as_secs_f64() > 0.0 {
        original_total as f64 / elapsed.as_secs_f64() / (1024.0 * 1024.0)
    } else {
        0.0
    };

    println!(
        "{:<20} cdc={:<8.4} total_dedup={:<8.4} stored_mb={:<10.3} orig_mb={:<10.3} elapsed_s={:<10.2} mbps={:.2}",
        name, cdc_ratio, total_ratio, stored_mb, orig_mb, elapsed.as_secs_f64(), mbps,
    );
}

fn main() {
    let kernel_dirs = [
        "/home/maxllon/pornfolder/marline/linux-3.4.5",
        "/home/maxllon/pornfolder/marline/linux-3.4.6",
        "/home/maxllon/pornfolder/marline/linux-3.4.7",
    ];

    let mut kernel_files: Vec<Vec<Vec<u8>>> = Vec::new();
    for dir in &kernel_dirs {
        let mut files = Vec::new();
        collect_files(Path::new(dir), &mut files);
        eprintln!(
            "  {}: {} files, {:.2} MB",
            dir,
            files.len(),
            files.iter().map(|d| d.len()).sum::<usize>() as f64 / (1024.0 * 1024.0)
        );
        kernel_files.push(files);
    }

    println!(
        "{:<20} {:<12} {:<21} {:<21} {:<21} {:<17} {:<12}",
        "config", "cdc_ratio", "total_dedup_ratio", "stored_mb", "orig_mb", "elapsed_s", "mbps"
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
                    sf_gen,
                    GdeltaEncoder,
                    tier_config,
                    LifecycleManager::<1>::default_configs(),
                );
                run_metrics(cfg.name, scrubber, &kernel_files);
            }
            2 => {
                let arr: [u32; 2] = cfg.tier_list.as_slice().try_into().unwrap();
                let tier_config = TierConfig::new(arr);
                let scrubber = PalantirScrubber::new(
                    sf_gen,
                    GdeltaEncoder,
                    tier_config,
                    LifecycleManager::<2>::default_configs(),
                );
                run_metrics(cfg.name, scrubber, &kernel_files);
            }
            3 => {
                let arr: [u32; 3] = cfg.tier_list.as_slice().try_into().unwrap();
                let tier_config = TierConfig::new(arr);
                let scrubber = PalantirScrubber::new(
                    sf_gen,
                    GdeltaEncoder,
                    tier_config,
                    LifecycleManager::<3>::default_configs(),
                );
                run_metrics(cfg.name, scrubber, &kernel_files);
            }
            4 => {
                let arr: [u32; 4] = cfg.tier_list.as_slice().try_into().unwrap();
                let tier_config = TierConfig::new(arr);
                let scrubber = PalantirScrubber::new(
                    sf_gen,
                    GdeltaEncoder,
                    tier_config,
                    LifecycleManager::<4>::default_configs(),
                );
                run_metrics(cfg.name, scrubber, &kernel_files);
            }
            5 => {
                let arr: [u32; 5] = cfg.tier_list.as_slice().try_into().unwrap();
                let tier_config = TierConfig::new(arr);
                let scrubber = PalantirScrubber::new(
                    sf_gen,
                    GdeltaEncoder,
                    tier_config,
                    LifecycleManager::<5>::default_configs(),
                );
                run_metrics(cfg.name, scrubber, &kernel_files);
            }
            _ => unreachable!(),
        }
    }
}
