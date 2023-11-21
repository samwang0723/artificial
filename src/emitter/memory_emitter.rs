use dashmap::DashMap;
use std::sync::Arc;
use warp::Filter;

use super::queue::FixedSizeQueue;

pub type Memory = Arc<MemoryEmitter>;

pub struct MemoryEmitter {
    inner: DashMap<String, FixedSizeQueue<String>>,
}

impl MemoryEmitter {
    pub fn new() -> Self {
        MemoryEmitter {
            inner: DashMap::new(),
        }
    }

    /// Combine strings in queue from a uuid
    pub fn get(&self, uuid: &str) -> String {
        self.inner
            .get(uuid)
            .map_or_else(String::new, |mem_store| mem_store.compose())
    }

    pub fn insert(&self, uuid: Arc<String>, reply: Arc<String>) {
        // Attempt to unwrap the Arc<String> for uuid
        let uuid = match Arc::try_unwrap(uuid) {
            Ok(uuid) => uuid,
            Err(arc) => {
                // Handle the case where the Arc cannot be unwrapped
                // For example, you can clone the inner String
                // Note: Cloning the String here, since we can't take ownership
                Arc::as_ref(&arc).clone()
            }
        };

        // Attempt to unwrap the Arc<String> for reply
        let reply = match Arc::try_unwrap(reply) {
            Ok(reply) => reply,
            Err(arc) => {
                // Handle the case where the Arc cannot be unwrapped
                // For example, you can clone the inner String
                // Note: Cloning the String here, since we can't take ownership
                Arc::as_ref(&arc).clone()
            }
        };
        let mut entry = self.inner.entry(uuid).or_insert(FixedSizeQueue::new(3));
        entry.handle_incoming(reply);
    }
}

pub fn create_memory() -> Memory {
    Arc::new(MemoryEmitter::new())
}

pub fn with_memory(
    mem: Memory,
) -> impl Filter<Extract = (Memory,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || mem.clone())
}

pub async fn record(mem: Memory, uuid: Arc<String>, message: Arc<String>) {
    mem.insert(uuid, message);
}

pub async fn get_memory(mem: Memory, uuid: Arc<String>) -> String {
    mem.get(&uuid)
}
