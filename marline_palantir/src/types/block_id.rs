use chunkfs::ChunkHash;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BlockID<H: ChunkHash> {
    pub hash: H,
    pub version: u64,
}

impl<H: ChunkHash> BlockID<H> {
    pub fn new(hash: H, version: u64) -> Self {
        Self { hash, version }
    }
}
