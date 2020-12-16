#![no_std]
extern crate alloc;

use alloc::collections::VecDeque;
use alloc::rc::Rc;
use core::cell::RefCell;

pub trait Enqueue<T> {
    fn enqueue(&mut self, message: T);
}

pub trait Dequeue<T> {
    fn dequeue(&mut self) -> Option<T>;
}

pub trait QueueMeta {
    /// Returns true if the underlying queue has messages.
    fn has_messages(&self) -> bool;
}

pub trait Queue<T>: Enqueue<T> + Dequeue<T> + QueueMeta {}

/// An in-memory [`Queue`] which stores [`QueueMessage`]s using a [`VecDeque`]. At most
/// `message_limit` messages will be stored. A `message_limit` of 0 disables the limit.
/// TODO revisit docs
pub struct MemQueue<T> {
    messages: VecDeque<T>,
    message_limit: usize,
    dropped_messages: usize,
}

impl<T> Enqueue<T> for MemQueue<T> {
    fn enqueue(&mut self, message: T) {
        if self.message_limit == 0 || self.messages.len() < self.message_limit {
            self.messages.push_back(message)
        } else {
            self.dropped_messages += 1;
        }
    }
}

impl<T> Dequeue<T> for MemQueue<T> {
    fn dequeue(&mut self) -> Option<T> {
        match self.has_messages() {
            true => self.messages.pop_front(),
            false => None,
        }
    }
}

impl<T> QueueMeta for MemQueue<T> {
    fn has_messages(&self) -> bool {
        !self.messages.is_empty()
    }
}

impl<T> Queue<T> for MemQueue<T> {}

pub type QueueHandle<T> = Rc<RefCell<dyn Queue<T>>>;

impl<T> MemQueue<T> {
    pub fn new(message_limit: usize) -> MemQueue<T>
    {
        MemQueue {
            messages: VecDeque::new(),
            dropped_messages: 0,
            message_limit,
        }
    }

    pub fn len(&self) -> usize {
        self.messages.len()
    }
}

#[cfg(test)]
mod queue_tests {
    use crate::*;
    use alloc::string::ToString;

    #[test]
    fn test_queue_enqueue() {
        let limit = 100;
        let mut queue = MemQueue::new(limit);

        for i in 0..limit {
            queue.enqueue(i.to_string());
            assert_eq!(i + 1, queue.len())
        }
        assert_eq!(limit, queue.len())
    }

        #[test]
        fn test_dequeue() {
            let limit = 100;

            let mut queue = MemQueue::new(limit);

            assert_eq!(0, queue.len());
            queue.dequeue();
            assert_eq!(0, queue.len());

            queue.enqueue("a".to_string());
            assert_eq!(1, queue.len());
            queue.dequeue();
            assert_eq!(0, queue.len());

            for i in 0..limit {
                let s = i.to_string();
                queue.enqueue(s.clone());
            }
            assert_eq!(limit, queue.len());

            let l = queue.len();

            // Ensure FIFO order is preserved
            for i in 0..l {
                let m = queue.dequeue().unwrap();
                assert_eq!(i.to_string(), m)
            }
            assert_eq!(0, queue.len());
        }

        #[test]
        fn test_has_messages() {
            let mut queue = MemQueue::new(1);
            queue.enqueue("a".to_string());
            assert!(queue.has_messages());
            queue.dequeue();
            assert!(!queue.has_messages());
        }
}
