use crate::levenshtein_functions::levenshtein_distance;
use crate::SBCChunk;
use std::collections::HashMap;
use chunkfs::Database;

const MAX_WEIGHT_EDGE: u32 = 1 << 8;

pub struct Vertex {
    pub(crate) parent: u32,
    rank: u32,
}
impl Vertex {
    pub fn new(key : u32) -> Vertex {
        Vertex { parent : key, rank : 1 }
    }
}

struct Edge {
    weight: u32,
    hash_chunk_1: u32,
    hash_chunk_2: u32,
}


pub struct Graph {
    pub(crate) vertices: HashMap<u32, Vertex>,
}

impl Graph {
    pub(crate) fn new() -> Graph {
        Graph {
            vertices : HashMap::new(),
        }
    }


    fn union_set(&mut self, hash_set_1: u32, hash_set_2: u32) {
        let hash_vertex_1 = self.vertices.get_key_value(&hash_set_1).unwrap();
        let hash_vertex_2 = self.vertices.get_key_value(&hash_set_2).unwrap();
        let rank = hash_vertex_2.1.rank + hash_vertex_1.1.rank;
        let hash_1 = *hash_vertex_1.0;
        let hash_2 = *hash_vertex_2.0;

        if hash_vertex_1.1.rank < hash_vertex_2.1.rank {
            self.vertices.insert(
                hash_2,
                Vertex {
                    parent: hash_2,
                    rank,
                },
            );
            self.vertices.insert(
                hash_1,
                Vertex {
                    parent: hash_2,
                    rank,
                },
            );
        } else {
            self.vertices.insert(
                hash_1,
                Vertex {
                    parent: hash_1,
                    rank,
                },
            );
            self.vertices.insert(
                hash_2,
                Vertex {
                    parent: hash_1,
                    rank,
                },
            );
        }
    }

    pub fn find_set(&mut self, hash_set: u32) -> u32 {
        let parent = self.vertices.get(&hash_set).unwrap().parent;
        let rank = self.vertices.get(&hash_set).unwrap().rank;

        if hash_set != parent {
            let parent = self.find_set(parent);
            self.vertices.insert(hash_set, Vertex { parent, rank });
            return self.vertices.get(&hash_set).unwrap().parent;
        }
        hash_set
    }

    pub fn update_graph_based_on_the_kraskal_algorithm(&mut self, pairs : Vec<(u32, Vec<u8>)>) -> HashMap<u32, Vec<u32>>{
        self.add_vertices(pairs.as_slice());
        let edges = self.create_edges(pairs.as_slice());
        for edge in edges {
            let hash_set_1 = self.find_set(edge.hash_chunk_1);
            let hash_set_2 = self.find_set(edge.hash_chunk_2);
            if hash_set_1 != hash_set_2 {
                self.union_set(hash_set_1, hash_set_2);
            }
        }

        let mut clusters = HashMap::new();
        for (hash, _) in pairs  {
            let leader = self.find_set(*hash);
            clusters.entry(leader).or_insert(Vec::new());
        }
        for hash in self.vertices.keys() {
            let leader = self.find_set(*hash);
            if self.vertices.contains_key(&leader) {
                clusters.get_mut(&leader).unwrap().push(*hash);
            }
        }
        clusters
    }


    fn add_vertices(&mut self, pairs : &[(u32, Vec<u8>)]) {
        for (key, _) in pairs {
            self.vertices.insert(*key, Vertex::new(*key));
        }
    }

    fn create_edges(&mut self, pairs : &[(u32, Vec<u8>)]) -> Vec<Edge>{
        let mut edges = Vec::new();
        for (hash_1, _) in pairs  {
            let mut min_dist = u32::MAX;
            let mut hash_2 = 0;
            for other_hash in *hash_1 - std::cmp::min(*hash_1, MAX_WEIGHT_EDGE)
                ..=*hash_1 + std::cmp::min(u32::MAX - *hash_1, MAX_WEIGHT_EDGE)
            {
                if !self.vertices.contains_key(&hash_2) {
                    continue;
                }
                let dist = std::cmp::max(hash_2, *hash_1) - std::cmp::min(hash_2, *hash_1);
                if dist < min_dist {
                    min_dist = dist;
                    hash_2 = other_hash;
                }

            }
            edges.push(Edge {
                weight: min_dist,
                hash_chunk_1: *hash_1,
                hash_chunk_2: hash_2,
            })
        }
        edges
    }


    pub fn set_leaders_in_clusters(&mut self, target_map: &mut Box<dyn Database<u32, SBCChunk>>, clusters : &HashMap<u32, Vec<u32>>) {
        for (parent_hash_past, cluster) in clusters {
            let parent_hash = find_parent_hash_in_cluster(target_map, cluster.as_slice());
            self.vertices.get_mut(&parent_hash).unwrap().rank = self.vertices.get(parent_hash_past).unwrap().rank;
            for hash in cluster.iter() {
                self.vertices.get_mut(hash).unwrap().parent = parent_hash
            }
        }
    }

}
fn find_parent_hash_in_cluster(target_map: &Box<dyn Database<u32, SBCChunk>>, cluster: &[u32]) -> u32 {
    let mut leader_hash = 0;
    let mut min_sum_dist = u32::MAX;

    for chunk_hash_1 in cluster.iter() {
        let mut sum_dist_for_chunk = 0u32;
        let chunk_data_1 = crate::chunkfs_sbc::get_data_chunk(target_map.get(chunk_hash_1).unwrap(), target_map);

        for chunk_hash_2 in cluster.iter() {
            let chunk_data_2 = crate::chunkfs_sbc::get_data_chunk(target_map.get(chunk_hash_2).unwrap(), target_map);

            sum_dist_for_chunk +=
                levenshtein_distance(chunk_data_1.as_slice(), chunk_data_2.as_slice());
        }

        if sum_dist_for_chunk < min_sum_dist {
            leader_hash = *chunk_hash_1;
            min_sum_dist = sum_dist_for_chunk
        }
    }
    leader_hash
}