use chunkfs::{Database, IterableDatabase};
use std::cell::Cell;
use std::collections::HashMap;
use std::io;

pub struct MockRocksDBMap {
    inner: HashMap<Vec<u8>, Vec<u8>>,
    pub get_count: Cell<usize>,
    pub insert_count: Cell<usize>,
    pub clear_count: Cell<usize>,
}

impl Database<Vec<u8>, Vec<u8>> for MockRocksDBMap {
    fn insert(&mut self, key: Vec<u8>, value: Vec<u8>) -> io::Result<()> {
        self.insert_count.set(self.insert_count.get() + 1);
        self.inner.insert(key, value);
        Ok(())
    }

    fn get(&self, key: &Vec<u8>) -> io::Result<Vec<u8>> {
        self.get_count.set(self.get_count.get() + 1);
        self.inner
            .get(key)
            .cloned()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "key not found"))
    }

    fn contains(&self, key: &Vec<u8>) -> bool {
        self.inner.contains_key(key)
    }
}

impl IterableDatabase<Vec<u8>, Vec<u8>> for MockRocksDBMap {
    fn iterator(&self) -> Box<dyn Iterator<Item = (&Vec<u8>, &Vec<u8>)> + '_> {
        Box::new(self.inner.iter())
    }

    fn iterator_mut(&mut self) -> Box<dyn Iterator<Item = (&Vec<u8>, &mut Vec<u8>)> + '_> {
        Box::new(self.inner.iter_mut())
    }

    fn clear(&mut self) -> io::Result<()> {
        self.clear_count.set(self.clear_count.get() + 1);
        self.inner.clear();
        Ok(())
    }
}

impl MockRocksDBMap {
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
            get_count: Cell::new(0),
            insert_count: Cell::new(0),
            clear_count: Cell::new(0),
        }
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
    pub fn total_bytes(&self) -> usize {
        self.inner.values().map(|v| v.len()).sum()
    }
}
