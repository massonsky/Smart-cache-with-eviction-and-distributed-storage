use crate::Node;
use std::collections::hash_map::DefaultHasher;
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};

const VIRTUAL_NODES: u16 = 128;

#[derive(Debug, Clone)]
pub struct Router {
    ring: BTreeMap<u64, Node>,
    replication_factor: usize,
}

impl Router {
    pub fn new(nodes: Vec<Node>, replication_factor: usize) -> Self {
        let mut ring = BTreeMap::new();

        for node in nodes {
            for replica in 0..VIRTUAL_NODES {
                let hash = hash_key(format!("{}:{replica}", node.id.0));
                ring.insert(hash, node.clone());
            }
        }

        Self {
            ring,
            replication_factor: replication_factor.max(1),
        }
    }

    pub fn owners(&self, key: &str) -> Vec<Node> {
        if self.ring.is_empty() {
            return Vec::new();
        }

        let key_hash = hash_key(key);
        let mut owners = Vec::new();

        for (_, node) in self.ring.range(key_hash..).chain(self.ring.range(..)) {
            if owners.iter().any(|owner: &Node| owner.id == node.id) {
                continue;
            }

            owners.push(node.clone());
            if owners.len() == self.replication_factor {
                break;
            }
        }

        owners
    }
}

fn hash_key<T: Hash>(value: T) -> u64 {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NodeId;

    #[test]
    fn returns_distinct_owners_up_to_replication_factor() {
        let router = Router::new(
            vec![
                Node {
                    id: NodeId("a".into()),
                    base_url: "http://a".into(),
                },
                Node {
                    id: NodeId("b".into()),
                    base_url: "http://b".into(),
                },
                Node {
                    id: NodeId("c".into()),
                    base_url: "http://c".into(),
                },
            ],
            2,
        );

        let owners = router.owners("user:1");
        assert_eq!(owners.len(), 2);
        assert_ne!(owners[0].id, owners[1].id);
    }
}
