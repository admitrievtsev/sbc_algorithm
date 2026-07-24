use super::chunk::Chunk;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SuperFeature {
    tier_id: u8,
    value: u32,
}

impl SuperFeature {
    pub fn new(tier_id: u8, value: u32) -> Self {
        Self { tier_id, value }
    }
    pub fn tier_id(&self) -> u8 {
        self.tier_id
    }
    pub fn value(&self) -> u32 {
        self.value
    }
}

pub trait SuperFeatureGenerator {
    fn generate(&self, chunk: &Chunk) -> Vec<SuperFeature>;
}
