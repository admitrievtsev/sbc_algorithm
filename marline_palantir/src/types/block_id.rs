use super::fingerprint::Fingerprint;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BlockID {
    pub fingerprint: Fingerprint,
    pub version: u64,
}

impl BlockID {
    pub fn new(fingerprint: Fingerprint, version: u64) -> Self {
        Self { fingerprint, version }
    }
}
