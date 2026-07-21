use crate::types::Chunk;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SuperFeature {
    tier_id: u8,
    hash: u64,
}

impl SuperFeature {
    pub fn new(tier_id: u8, hash: u64) -> Self {
        Self { tier_id, hash }
    }
    pub fn tier_id(&self) -> u8 {
        self.tier_id
    }
    pub fn hash(&self) -> u64 {
        self.hash
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TierConfig {
    pub tier_list: Vec<u32>,
}

impl TierConfig {
    pub fn new(tier_list: Vec<u32>) -> Self {
        Self { tier_list }
    }
}

pub trait SuperFeatureGenerator {
    fn generate(&self, chunk: &Chunk) -> Vec<SuperFeature>;
}
