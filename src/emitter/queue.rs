use std::collections::VecDeque;

pub struct FixedSizeQueue<T> {
    deque: VecDeque<T>,
    capacity: usize,
}

impl<T> FixedSizeQueue<T> {
    // Create a new FixedSizeQueue with the given capacity
    pub fn new(capacity: usize) -> Self {
        FixedSizeQueue {
            deque: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    // Push an element to the queue, and if the capacity is exceeded, pop the oldest element
    pub fn push(&mut self, item: T) {
        if self.deque.len() == self.capacity {
            self.deque.pop_front(); // Remove the oldest item if we're at capacity
            self.deque.pop_front(); // Remove the 2nd oldest item
        }
        self.deque.push_back(item);
    }
}

impl FixedSizeQueue<String> {
    // Check if the last element is a stop token
    fn last_is_stop(&self) -> bool {
        if let Some(last) = self.deque.back() {
            last.as_str().contains("[[stop]]")
        } else {
            false
        }
    }

    // Append a string slice to the latest element in the queue
    fn push_to_latest(&mut self, slice: &str) {
        if let Some(latest) = self.deque.back_mut() {
            latest.push_str(slice);
        }
    }

    // Handle incoming string from the channel
    pub fn handle_incoming(&mut self, string: String) {
        if self.deque.is_empty() || self.last_is_stop() {
            // If the queue is empty or the last element is a stop token, push a new string
            self.push(string);
        } else {
            // Otherwise, append to the latest string
            self.push_to_latest(&string);
        }
    }

    pub fn compose(&self) -> String {
        self.deque.iter().fold(String::new(), |acc, x| acc + x)
    }
}
