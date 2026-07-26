use std::collections::HashMap;

use chunkfs::{Data, DataContainer, Scrub};
use marline_palantir::encoder::GdeltaEncoder;
use marline_palantir::palantir_scrubber::{Index, PalantirScrubber};
use marline_palantir::sf_generator::PalantirHasher;

#[test]
fn test_empty_database() {
    let sf_gen = PalantirHasher::new(7, vec![4, 3, 2]);
    let index: Index<Vec<u8>> = Index::new();
    let encoder = GdeltaEncoder;
    let mut scrubber = PalantirScrubber::new(sf_gen, index, encoder);

    let mut database: HashMap<Vec<u8>, DataContainer<Vec<u8>>> = HashMap::new();
    let mut target_map: HashMap<Vec<u8>, Vec<u8>> = HashMap::new();

    let result = scrubber.scrub(&mut database, &mut target_map).unwrap();

    assert_eq!(result.processed_data, 0);
    assert!(target_map.is_empty());
}

#[test]
fn test_single_chunk_stored_raw() {
    let sf_gen = PalantirHasher::new(7, vec![4, 3, 2]);
    let index: Index<Vec<u8>> = Index::new();
    let encoder = GdeltaEncoder;
    let mut scrubber = PalantirScrubber::new(sf_gen, index, encoder);

    let data: Vec<u8> =
        b"Hello, World! This is a moderately sized chunk for super-feature generation.".to_vec();

    let mut database = HashMap::new();
    database.insert(data.clone(), DataContainer::from(data.clone()));
    let mut target_map: HashMap<Vec<u8>, Vec<u8>> = HashMap::new();

    let result = scrubber.scrub(&mut database, &mut target_map).unwrap();

    assert_eq!(result.processed_data, data.len());
    assert_eq!(target_map.len(), 1);
    assert_eq!(target_map.get(&data), Some(&data));

    match database.get(&data).unwrap().extract() {
        Data::TargetChunk(keys) => assert_eq!(keys.as_slice(), &[data]),
        _ => panic!("expected TargetChunk after scrub"),
    }
}

#[test]
fn test_identical_chunks_delta_encoded() {
    let sf_gen = PalantirHasher::new(7, vec![4, 3, 2]);
    let index: Index<Vec<u8>> = Index::new();
    let encoder = GdeltaEncoder;
    let mut scrubber = PalantirScrubber::new(sf_gen, index, encoder);

    let data: Vec<u8> = b"This chunk appears twice under distinct keys. \
                           The second-chunk processed will find the first via the index \
                           and be delta-encoded. The first chunk is always stored raw."
        .to_vec();

    let key1: Vec<u8> = b"\x00-key".to_vec();
    let key2: Vec<u8> = b"\x01-key".to_vec();

    let mut database = HashMap::new();
    database.insert(key1.clone(), DataContainer::from(data.clone()));
    database.insert(key2.clone(), DataContainer::from(data.clone()));
    let mut target_map: HashMap<Vec<u8>, Vec<u8>> = HashMap::new();

    let result = scrubber.scrub(&mut database, &mut target_map).unwrap();

    assert_eq!(result.processed_data, data.len() * 2);
    assert_eq!(target_map.len(), 2);

    assert!(target_map.get(&key1).unwrap().len() <= data.len());
    assert!(target_map.get(&key2).unwrap().len() <= data.len());

    let delta_count = [&key1, &key2]
        .iter()
        .filter(|k| target_map.get(k.as_slice()).unwrap().len() < data.len())
        .count();
    assert_eq!(delta_count, 1);

    for k in [&key1, &key2] {
        match database.get(k).unwrap().extract() {
            Data::TargetChunk(keys) => assert_eq!(keys.as_slice(), &[k.clone()]),
            _ => panic!("expected TargetChunk after scrub"),
        }
    }
}

#[test]
fn test_similar_chunks_delta_encoded() {
    let sf_gen = PalantirHasher::new(7, vec![4, 3, 2]);
    let index: Index<Vec<u8>> = Index::new();
    let encoder = GdeltaEncoder;
    let mut scrubber = PalantirScrubber::new(sf_gen, index, encoder);

    let base: Vec<u8> = b"This is a base chunk for the Palantir similarity pipeline. \
                           It has enough bytes for the gear-hash rolling hash to produce \
                           stable super-features across multiple tiers."
        .to_vec();
    let similar: Vec<u8> = b"This is a MODIFIED chunk for the Palantir similarity pipeline. \
                             It has enough bytes for the gear-hash rolling hash to produce \
                             stable super-features across multiple tiers."
        .to_vec();

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
    assert_eq!(delta_count, 1);

    for k in [&base, &similar] {
        match database.get(k).unwrap().extract() {
            Data::TargetChunk(keys) => assert_eq!(keys.as_slice(), &[k.clone()]),
            _ => panic!("expected TargetChunk after scrub"),
        }
    }
}

#[test]
fn test_dissimilar_chunks_stored_raw() {
    let sf_gen = PalantirHasher::new(7, vec![4, 3, 2]);
    let index: Index<Vec<u8>> = Index::new();
    let encoder = GdeltaEncoder;
    let mut scrubber = PalantirScrubber::new(sf_gen, index, encoder);

    let chunk_a: Vec<u8> = b"AAAA AAAA AAAA AAAA AAAA AAAA AAAA AAAA AAAA AAAA AAAA AAAA AAAA AAAA AAAA AAAA AAAA AAAA AAAA AAAA AAAA AAAA AAAA AAAA AAAA AAAA AAAA AAAA AAAA AAAA".to_vec();
    let chunk_b: Vec<u8> = b"BBBB BBBB BBBB BBBB BBBB BBBB BBBB BBBB BBBB BBBB BBBB BBBB BBBB BBBB BBBB BBBB BBBB BBBB BBBB BBBB BBBB BBBB BBBB BBBB BBBB BBBB BBBB BBBB BBBB BBBB".to_vec();

    let mut database = HashMap::new();
    database.insert(chunk_a.clone(), DataContainer::from(chunk_a.clone()));
    database.insert(chunk_b.clone(), DataContainer::from(chunk_b.clone()));
    let mut target_map: HashMap<Vec<u8>, Vec<u8>> = HashMap::new();

    let result = scrubber.scrub(&mut database, &mut target_map).unwrap();

    assert_eq!(result.processed_data, chunk_a.len() + chunk_b.len());
    assert_eq!(target_map.len(), 2);

    assert_eq!(target_map.get(&chunk_a), Some(&chunk_a));
    assert_eq!(target_map.get(&chunk_b), Some(&chunk_b));

    for k in [&chunk_a, &chunk_b] {
        match database.get(k).unwrap().extract() {
            Data::TargetChunk(keys) => assert_eq!(keys.as_slice(), &[k.clone()]),
            _ => panic!("expected TargetChunk after scrub"),
        }
    }
}

#[test]
fn test_mixed_similarity_chunks() {
    let sf_gen = PalantirHasher::new(7, vec![4, 3, 2]);
    let index: Index<Vec<u8>> = Index::new();
    let encoder = GdeltaEncoder;
    let mut scrubber = PalantirScrubber::new(sf_gen, index, encoder);

    let group1_base: Vec<u8> = b"Alpha base chunk. The rolling hash requires sufficient data to produce stable \
                                 super-features across multiple tiers for similarity-based deduplication. \
                                 The gear-hash rolling hash samples features when the fingerprint has trailing \
                                 zero bits, so most features between near-identical chunks should overlap. \
                                 This chunk is long enough to test the Palantir pipeline end to end with \
                                 meaningful super-feature generation and the multi-tier index lookup.".to_vec();
    let group1_similar: Vec<u8> = b"Alpha base chunk. The rolling hash requires sufficient data to produce stable \
                                    super-features across multiple tiers for similarity-based deduplication. \
                                    The gear-hash rolling hash samples features when the fingerprint has trailing \
                                    zero bits, so most features between near-identical chunks should overlap. \
                                    This chunk is MODIFIED FOR TESTING to verify delta encoding STILL WORKS.".to_vec();
    let unique: Vec<u8> = b"AAAAA BBBBB CCCCC DDDDD EEEEE FFFFF GGGGG HHHHH IIIII JJJJJ KKKKK LLLLL MMMMM NNNNN OOOOO PPPPP QQQQQ RRRRR SSSSS TTTTT UUUUU VVVVV WWWWW XXXXX YYYYY ZZZZZ".to_vec();

    let mut database = HashMap::new();
    database.insert(group1_base.clone(), DataContainer::from(group1_base.clone()));
    database.insert(group1_similar.clone(), DataContainer::from(group1_similar.clone()));
    database.insert(unique.clone(), DataContainer::from(unique.clone()));
    let mut target_map: HashMap<Vec<u8>, Vec<u8>> = HashMap::new();

    let result = scrubber.scrub(&mut database, &mut target_map).unwrap();

    let total_size = group1_base.len() + group1_similar.len() + unique.len();
    assert_eq!(result.processed_data, total_size);
    assert_eq!(target_map.len(), 3);

    let delta_count = [&group1_base, &group1_similar]
        .iter()
        .filter(|k| target_map.get(k.as_slice()).unwrap().len() < k.len())
        .count();
    assert_eq!(delta_count, 1);

    assert_eq!(target_map.get(&unique), Some(&unique));

    for k in [&group1_base, &group1_similar, &unique] {
        match database.get(k).unwrap().extract() {
            Data::TargetChunk(keys) => assert_eq!(keys.as_slice(), &[k.clone()]),
            _ => panic!("expected TargetChunk after scrub"),
        }
    }
}

#[test]
fn test_many_chunks_pipeline_throughput() {
    let sf_gen = PalantirHasher::new(7, vec![4, 3, 2]);
    let index: Index<Vec<u8>> = Index::new();
    let encoder = GdeltaEncoder;
    let mut scrubber = PalantirScrubber::new(sf_gen, index, encoder);

    let base: Vec<u8> = b"Template chunk for generating a family of similar chunks. \
                           Each variant introduces a small edit so the super-features \
                           remain close enough for tiered similarity search to match."
        .to_vec();

    let mut database: HashMap<Vec<u8>, DataContainer<Vec<u8>>> = HashMap::new();
    for i in 0..10 {
        let mut data = base.clone();
        data.push(i);
        let key = format!("key-{:02}", i).into_bytes();
        database.insert(key, DataContainer::from(data));
    }

    let mut target_map: HashMap<Vec<u8>, Vec<u8>> = HashMap::new();
    let result = scrubber.scrub(&mut database, &mut target_map).unwrap();

    assert!(result.processed_data > 0);
    assert_eq!(target_map.len(), 10);
}

#[test]
fn test_target_chunks_are_skipped() {
    let sf_gen = PalantirHasher::new(7, vec![4, 3, 2]);
    let index: Index<Vec<u8>> = Index::new();
    let encoder = GdeltaEncoder;
    let mut scrubber = PalantirScrubber::new(sf_gen, index, encoder);

    let mut database: HashMap<Vec<u8>, DataContainer<Vec<u8>>> = HashMap::new();

    let mut dc = DataContainer::from(b"some data".to_vec());
    dc.make_target(vec![b"t-key".to_vec()]);
    database.insert(b"skipped".to_vec(), dc);

    let mut target_map: HashMap<Vec<u8>, Vec<u8>> = HashMap::new();
    let result = scrubber.scrub(&mut database, &mut target_map).unwrap();

    assert_eq!(result.processed_data, 0);
    assert!(target_map.is_empty());
}
