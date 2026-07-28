use std::collections::HashMap;

use chunkfs::{Data, DataContainer, Scrub};
use marline_palantir::encoder::GdeltaEncoder;
use marline_palantir::lifecycle_manager::LifecycleManager;
use marline_palantir::metadata_manager::MetadataManager;
use marline_palantir::palantir_scrubber::PalantirScrubber;
use marline_palantir::sf_generator::PalantirHasher;
use marline_palantir::types::{BlockID, Chunk, SuperFeatureGenerator, TierConfig};

#[test]
fn identical_chunks() {
    let sf_gen = PalantirHasher::new(7, vec![4, 3, 2]);
    let tier_config = TierConfig::new([4, 3, 2]);
    let lifecycle_configs = LifecycleManager::<3>::default_configs();
    let mut mm = MetadataManager::<Vec<u8>, 3>::new(tier_config, lifecycle_configs);

    let base: Vec<u8> = b"This is a base chunk for the Palantir similarity pipeline. \
                           It has enough bytes for the gear-hash rolling hash to produce \
                           stable super-features across multiple tiers."
        .to_vec();
    let hash = base.clone();

    let chunk = Chunk::new(base);
    let sfs = sf_gen.generate(&chunk);
    mm.add_block(hash.clone(), &sfs, BlockID::new(hash.clone()));

    let found = mm.lookup_fingerprint(&hash);
    assert!(found.is_some(), "Identical chunk not found via fingerprint lookup");
}

#[test]
fn similar_chunks() {
    let sf_gen = PalantirHasher::new(7, vec![4, 3, 2]);
    let encoder = GdeltaEncoder;
    let tier_config = TierConfig::new([4, 3, 2]);
    let lifecycle_configs = LifecycleManager::<3>::default_configs();
    let mut scrubber = PalantirScrubber::new(sf_gen, encoder, tier_config, lifecycle_configs);

    let base: Vec<u8> = (0..8192).map(|i| (i % 256) as u8).collect();
    let mut similar = base.clone();

    for i in 4096..4916 {
        similar[i] = similar[i].wrapping_add(7);
    }

    let mut database = HashMap::new();
    database.insert(base.clone(), DataContainer::from(base.clone()));
    database.insert(similar.clone(), DataContainer::from(similar.clone()));

    let mut target_map: HashMap<Vec<u8>, Vec<u8>> = HashMap::new();
    let result = scrubber.scrub(&mut database, &mut target_map).unwrap();

    assert_eq!(result.processed_data, base.len() + similar.len());
    assert_eq!(target_map.len(), 2);

    let delta_count = [&base, &similar]
        .iter()
        .filter(|k| target_map.get(k.as_slice()).unwrap().len() < k.len())
        .count();
    assert!(
        delta_count >= 1,
        "At least one chunk should be delta-encoded, got {} deltas",
        delta_count
    );

    for k in [&base, &similar] {
        let v = target_map.get(k.as_slice()).unwrap();
        if v.len() < k.len() {
            let delta_zstd = zstd::encode_all(v.as_slice(), 0).unwrap();
            let raw_zstd = zstd::encode_all(k.as_slice(), 0).unwrap();
            let ratio = delta_zstd.len() as f64 / raw_zstd.len() as f64;
            assert!(
                ratio < 0.9,
                "Filter should preserve delta, ratio={:.2}",
                ratio
            );
            break;
        }
    }

    for hash in [&base, &similar] {
        match database.get(hash).unwrap().extract() {
            Data::TargetChunk(keys) => assert_eq!(keys.as_slice(), &[hash.clone()]),
            _ => panic!("Expected TargetChunk after scrub"),
        }
    }
}
