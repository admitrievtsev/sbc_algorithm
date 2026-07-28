use std::collections::HashMap;

use chunkfs::{Data, DataContainer, Scrub};
use marline_palantir::encoder::GdeltaEncoder;
use marline_palantir::palantir_scrubber::{Index, PalantirScrubber};
use marline_palantir::sf_generator::PalantirHasher;
use marline_palantir::types::{Chunk, SuperFeatureGenerator, TierConfig};

#[test]
fn identical_chunks() {
    let base: Vec<u8> = b"This is a base chunk for the Palantir similarity pipeline. \
                           It has enough bytes for the gear-hash rolling hash to produce \
                           stable super-features across multiple tiers."
        .to_vec();
    let identical: Vec<u8> = base.clone();

    let tier_cfg = TierConfig::new(vec![4, 3, 2]);
    let index: Index<Vec<u8>> = Index::new(&tier_cfg);
    let sf_gen = PalantirHasher::new(7, tier_cfg.tier_list.clone());

    let chunk1 = Chunk::new(base.clone());
    let sfs1 = sf_gen.generate(&chunk1);
    let hash1 = base.clone();
    index.insert(&sfs1, hash1.clone());

    let chunk2 = Chunk::new(identical.clone());
    let sfs2 = sf_gen.generate(&chunk2);

    let found = index.search(&sfs2);

    assert_eq!(found, Some(hash1), "Identical chunks is not found!!!");
}

#[test]
fn similar_chunks() {
    use sha2::Digest;

    let base: Vec<u8> = (0..8192).map(|i| (i % 256) as u8).collect();
    let mut similar = base.clone();

    //change ~10%
    for i in 4096..4916 {
        similar[i] = similar[i].wrapping_add(7);
    }

    //(SHA-256)
    let base_hash: [u8; 32] = sha2::Sha256::digest(&base).into();
    let similar_hash: [u8; 32] = sha2::Sha256::digest(&similar).into();

    let tier_cfg = TierConfig::new(vec![3, 2]);
    let index = Index::new(&tier_cfg);
    let sf_gen = PalantirHasher::new(7, tier_cfg.tier_list.clone());
    let encoder = GdeltaEncoder;
    let mut scrubber = PalantirScrubber::new(sf_gen, index, encoder);

    let mut database = HashMap::new();
    database.insert(base_hash.clone(), DataContainer::from(base.clone()));
    database.insert(similar_hash.clone(), DataContainer::from(similar.clone()));

    let mut target_map: HashMap<[u8; 32], Vec<u8>> = HashMap::new();
    let result = scrubber.scrub(&mut database, &mut target_map).unwrap();

    assert_eq!(result.processed_data, base.len() + similar.len());
    assert_eq!(target_map.len(), 2);
    assert_eq!(
        target_map.get(&base_hash).unwrap(),
        &base,
        "Base: {:?}, Similar: {:?}",
        target_map.get(&base_hash).unwrap(),
        target_map.get(&similar_hash).unwrap()
    );

    let delta = target_map.get(&similar_hash).unwrap();
    assert!(
        delta.len() < similar.len(),
        "Delta must be compressed: delta={}, original={}",
        delta.len(),
        similar.len()
    );

    let delta_compressed = zstd::encode_all(delta.as_slice(), 0).unwrap();
    let original_compressed = zstd::encode_all(similar.as_slice(), 0).unwrap();
    let ratio = delta_compressed.len() as f64 / original_compressed.len() as f64;
    assert!(ratio < 0.9, "The filter was supposed to preserve the delta, ratio={:.2}", ratio);

    // Check if Target Chunk
    for hash in [&base_hash, &similar_hash] {
        match database.get(hash).unwrap().extract() {
            Data::TargetChunk(keys) => assert_eq!(keys.as_slice(), &[hash.clone()]),
            _ => panic!("Expected TargetChunk after scrub"),
        }
    }
}
