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

pub trait SuperFeatureGenerator {
    fn generate(&self, chunk: &Chunk) -> Vec<SuperFeature>;
}
