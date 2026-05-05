pub mod http;
pub mod router;

use smart_cache_core::Cache;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NodeId(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub id: NodeId,
    pub base_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConsistencyPolicy {
    LocalOnly,
    PrimaryReplica,
    Quorum {
        read_quorum: usize,
        write_quorum: usize,
    },
}

#[derive(Debug, Clone)]
pub struct ClusterConfig {
    pub self_node: Node,
    pub nodes: Vec<Node>,
    pub replication_factor: usize,
    pub consistency: ConsistencyPolicy,
}

#[derive(Clone)]
pub struct Cluster {
    pub config: ClusterConfig,
    pub local_cache: Arc<RwLock<Cache>>,
    pub router: router::Router,
}

impl Cluster {
    pub fn new(config: ClusterConfig, local_cache: Cache) -> Self {
        let router = router::Router::new(config.nodes.clone(), config.replication_factor);

        Self {
            config,
            local_cache: Arc::new(RwLock::new(local_cache)),
            router,
        }
    }

    pub fn owners_for_key(&self, key: &str) -> Vec<Node> {
        self.router.owners(key)
    }

    pub fn primary_owner(&self, key: &str) -> Option<Node> {
        self.owners_for_key(key).into_iter().next()
    }

    pub fn is_local_owner(&self, key: &str) -> bool {
        self.owners_for_key(key)
            .iter()
            .any(|node| node.id == self.config.self_node.id)
    }
}
