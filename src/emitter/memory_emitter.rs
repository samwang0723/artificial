use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use tokio::sync::Mutex;
use warp::Filter;

use super::queue::FixedSizeQueue;

pub type Memory = Arc<Mutex<MemoryEmitter>>;

pub struct MemoryEmitter {
    inner: HashMap<String, FixedSizeQueue<String>>,
}

impl MemoryEmitter {
    pub fn new() -> Self {
        MemoryEmitter {
            inner: HashMap::new(),
        }
    }

    /// Combine strings in queue from a uuid
    pub fn get(&self, uuid: Arc<String>) -> String {
        match self.inner.get(uuid.as_str()) {
            Some(mem_store) => mem_store.compose(),
            None => String::new(),
        }
    }

    fn insert(&mut self, uuid: Arc<String>, reply: Arc<String>) {
        let uuid = uuid.to_string();
        // if hashmap key not found, create new fixed size queue
        if !self.inner.contains_key(uuid.as_str()) {
            self.inner.insert(uuid.clone(), FixedSizeQueue::new(3));
        }

        let mem_store = self.inner.get_mut(uuid.as_str()).unwrap();
        mem_store.handle_incoming(reply.to_string());
    }
}

impl Deref for MemoryEmitter {
    type Target = HashMap<String, FixedSizeQueue<String>>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for MemoryEmitter {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

pub fn create_memory() -> Memory {
    Arc::new(Mutex::new(MemoryEmitter::new()))
}

pub fn with_memory(
    mem: Memory,
) -> impl Filter<Extract = (Memory,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || mem.clone())
}

pub async fn record(mem: Memory, uuid: Arc<String>, message: Arc<String>) {
    let mut m = mem.lock().await;
    m.insert(uuid, message);
}

pub async fn get_memory(mem: Memory, uuid: Arc<String>) -> String {
    let m = mem.lock().await;
    m.get(uuid)
}
