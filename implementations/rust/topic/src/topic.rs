use alloc::boxed::Box;
use alloc::collections::VecDeque;
use alloc::rc::Rc;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::cell::RefCell;

use hashbrown::HashMap;

use ockam_queue::{MemQueue, QueueHandle};

/// An in-memory [`Queue`] tracking worker. Queues will be created on demand by `get_queue`.
pub struct TopicQueueContainer<T> {
    address: String,
    queue_map: HashMap<String, QueueHandle<T>>,
    default_queue_limit: usize,
}

/// Wrapper type for QueueWorker trait object.
pub type QueueContainerHandle<T> = Rc<RefCell<TopicQueueContainer<T>>>;

impl<T: 'static> TopicQueueContainer<T> {
    pub fn new<S>(address: S, default_queue_limit: usize) -> TopicQueueContainer<T>
        where
            S: ToString,
    {
        TopicQueueContainer {
            address: address.to_string(),
            queue_map: HashMap::new(),
            default_queue_limit,
        }
    }

    pub fn create<S>(address: S, default_queue_limit: usize) -> QueueContainerHandle<T>
        where
            S: ToString,
    {
        Rc::new(RefCell::new(TopicQueueContainer::new(
            address,
            default_queue_limit,
        )))
    }

    pub fn create_unbound<S>(address: S) -> QueueContainerHandle<T>
        where
            S: ToString,
    {
        TopicQueueContainer::create(address, 0)
    }

    fn address(&self) -> String {
        self.address.clone()
    }

    fn get_queue(&mut self, queue_address: &str) -> Option<QueueHandle<T>> {
        if queue_address.is_empty() {
            return None;
        }

        if self.queue_map.contains_key(queue_address) {
            self.queue_map.get(queue_address).cloned()
        } else {
            let name_string = queue_address.to_string();
            let new_queue = MemQueue::new( self.default_queue_limit);
            self.queue_map.insert(name_string.clone(), Rc::new(RefCell::new(new_queue)));
            self.queue_map.get(queue_address).cloned()
        }
    }

    fn remove_queue(&mut self, queue_name: &str) {
        self.queue_map.remove(queue_name);
    }
}

/// An addressable Topic trait.
pub trait Topic {
    fn topic_address(&self) -> &str;
}

/// In-memory implementation of an addressable [`Topic`]
pub struct MemTopic {
    pub topic_address: String,
}

impl MemTopic {
    pub fn create<T>(topic_name: T) -> TopicHandle
    where
        T: ToString,
    {
        Rc::new(RefCell::new(MemTopic {
            topic_address: topic_name.to_string(),
        }))
    }
}

impl Topic for MemTopic {
    fn topic_address(&self) -> &str {
        &self.topic_address
    }
}

/// Wrapper type for a [`Topic`] trait object.
pub type TopicHandle = Rc<RefCell<dyn Topic>>;

/// An association between a Subscriber and a [`Topic`], backed by a [`Queue`].
pub trait Subscription {
    /// Address of the [`Topic`] of this subscription.
    fn topic(&self) -> &str;

    /// Handler to the [`Queue`] implementation.
  //  fn queue(&self) -> QueueHandle<T>;

    /// The address of this subscription.
    fn subscriber_address(&self) -> &str;
}

/// Wrapper type for a [`Subscription`] trait object.
type SubscriptionHandle = Rc<RefCell<dyn Subscription>>;

/// In-memory implementation of a [`Subscription`].
#[derive(Clone)]
pub struct MemSubscription {
    pub topic: String,
  //  pub queue: Rc<RefCell<dyn Queue<QueueMessage>>>,
    pub subscriber_address: String,
}

impl MemSubscription {
    /// Create a new subscription on `topic` with message storage on `queue`, at the given address.
    fn create<S>(
        topic: S,
      //  queue: QueueHandle,
        subscriber_address: S,
    ) -> Rc<RefCell<MemSubscription>>
    where
        S: ToString,
    {
        Rc::new(RefCell::new(MemSubscription {
            topic: topic.to_string(),
    //        queue: queue.clone(),
            subscriber_address: subscriber_address.to_string(),
        }))
    }
}

impl Subscription for MemSubscription {
    fn topic(&self) -> &str {
        &self.topic
    }

/*    fn queue(&self) -> QueueHandle {
        self.queue.clone()
    }*/

    fn subscriber_address(&self) -> &str {
        &self.subscriber_address
    }
}

/// A Worker that manages publish/subscribe [`Subscription`]s to a [`Topic`].
pub trait TopicWorker<T> {
    /// Publishes `message` to the [`Topic`] at `topic`.
    fn publish(&mut self, topic: &str, message: T);

    /// Start a new [`Subscription`] to `topic`. On success, the [`Subscription`]'s Address is
    /// returned.
    fn subscribe(&mut self, topic: &str) -> Option<String>;

    /// Fetch all available messages for `subscriber`.
    fn consume_messages(&mut self, subscriber: &str) -> Box<Vec<T>>;

    /// Remove the [`Subscription`] at address `subscriber`.
    fn unsubscribe(&mut self, subscriber: &str);
}


/// In-memory [`TopicWorker`] for [`Subscription`] state tracking. Subscription addresses are
/// created by an internal counter which increments for every new subscription.
pub struct MemTopicWorker<T> {
    queue_worker: QueueContainerHandle<T>,
    subscriptions: HashMap<String, SubscriptionHandle>,
    subscription_id_counter: usize,
}

impl<T> MemTopicWorker<T> {
    pub fn new(queue_worker: QueueContainerHandle<T>) -> MemTopicWorker<T> {
        MemTopicWorker {
            subscriptions: HashMap::new(),
            subscription_id_counter: 0,
            queue_worker,
        }
    }

    pub fn create(queue_worker: QueueContainerHandle<T>) -> Rc<RefCell<MemTopicWorker<T>>> {
        Rc::new(RefCell::new(MemTopicWorker::new(queue_worker)))
    }
}

impl TopicWorker<String> for MemTopicWorker<String> {
    /// Find all [`Subscription`]s to `topic` and enqueue `message` their [`Queue`]s. If performance
    /// begins to suffer from doing a full scan of subscriptions to match topic, we could rearrange
    /// the internal storage to map topics to subscribers, in addition to the current implementation
    /// which is by subscriber address.
    fn publish(&mut self, topic: &str, message: String) {
      /*  for subscriber in self.subscriptions.values() {
            let sub = subscriber.borrow_mut();
            if sub.topic() == topic {
                sub.queue().borrow_mut().enqueue(message.clone());
            }
        }*/
    }

    /// Creates a new [`Subscription`] to `topic` with a Subscription Worker address of the form
    /// `{int}_{topic}` This implementation will provide unique Subscription Worker addresses
    /// during a given runtime. No state is stored, so addresses will be reused for each new
    /// [`MemTopicWorker`]
    fn subscribe(&mut self, topic: &str) -> Option<String> {
        None
  /*      let subscriber_address = format!("{}_{}", self.subscription_id_counter, topic);
        match self
            .queue_worker
            .borrow_mut()
            .get_queue(subscriber_address.as_str())
        {
            Some(queue) => {
             let sub = MemSubscription::create(topic, queue, &subscriber_address);

                self.subscriptions
                    .insert(subscriber_address.clone(), sub.clone());
                self.subscription_id_counter += 1;
                Some(subscriber_address.clone())
            }
            _ => None,
        }*/
    }

    fn consume_messages(&mut self, subscriber: &str) -> Box<Vec<String>> {
        let mut messages = Box::new(Vec::new());
        match self.subscriptions.get(subscriber) {
/*            Some(sub) => {
                let q = sub.borrow().queue();
                let mut queue = q.borrow_mut();
                while queue.has_messages() {
                    match queue.dequeue() {
                        Some(message) => messages.push(message),
                        _ => (),
                    };
                }
            }*/
            _ => (),
        };
        messages
    }

    fn unsubscribe(&mut self, subscriber: &str) {
        self.subscriptions.remove(subscriber);
    }
}

#[cfg(test)]
mod topic_tests {
    use alloc::rc::Rc;
    use alloc::string::ToString;
    use core::cell::RefCell;

    use ockam_queue::MemQueue;

    use crate::topic::{MemSubscription, MemTopic, MemTopicWorker, Subscription};

    #[test]
    fn test_topic_address() {
        let topic = MemTopic::create("a");
        assert_eq!("a", topic.borrow().topic_address());
    }

    #[test]
    fn test_mem_subscription() {
        let sub = MemSubscription {
            topic: "a".to_string(),
       //     queue: Rc::new(RefCell::new(MemQueue::new( 1))),
            subscriber_address: "".to_string(),
        };

        let sub_b = sub.clone();
        assert_eq!("a", sub_b.topic);
    }
    /*
        #[test]
        fn test_subscription_topic() {
            let topic = "a";
            let subscriber_address = 0.to_string() + "_" + topic;
            let queue = MemQueue::create(subscriber_address.clone(), 1);
            let sub_ref = MemSubscription::create(topic, queue, &subscriber_address);
            let sub = sub_ref.borrow();
            assert_eq!(topic, sub.topic());
            assert_eq!(subscriber_address, sub.subscriber_address());*/
    }

    /*#[test]
   fn test_subscription_queue() {
       let topic = "a";
       let subscriber_address = 0.to_string() + "_" + topic;
       let queue = MemQueue::create(subscriber_address.clone(), 1);
      let sub_ref = MemSubscription::create(topic, queue.clone(), &subscriber_address);
       let sub = sub_ref.borrow();

       sub.queue.borrow_mut().enqueue("a".to_msg().unwrap());

       assert!(queue.borrow().has_messages())
    }
*/
/*    #[test]
    fn test_subscription_subscriber_address() {
        let topic = "a";
        let subscriber_address = 0.to_string() + "_" + topic;
        let queue = MemQueue::create(subscriber_address.clone(), 1);
      /*  let sub_ref = MemSubscription::create(topic, queue, &subscriber_address);
        let sub = sub_ref.borrow();

        assert_eq!(subscriber_address, sub.subscriber_address())*/
    }*/

/*    #[test]
    fn test_topic_worker_publish() {
        let queue_worker = MemQueueWorker::create_unbound("q1");
        let topic_worker_ref = MemTopicWorker::create(queue_worker.clone());
        let mut topic_worker = topic_worker_ref.borrow_mut();
        let sub = topic_worker.subscribe("a").unwrap();

        let validation = "ockam";

        topic_worker.publish("a", validation.to_msg().unwrap());

        let queue_opt = queue_worker.borrow_mut().get_queue(&sub);
        assert!(queue_opt.is_some());
        let queue = queue_opt.unwrap();
        assert_eq!(sub, queue.borrow().address());

        let message = queue.borrow_mut().dequeue();
        assert!(message.is_some());
        let m = message.unwrap();
        assert_eq!(validation.as_bytes().to_vec(), m.body)
    }*/

/*    #[test]
    fn test_topic_worker_subscribe() {
        let queue_worker = MemQueueWorker::create_unbound("q1");
        let topic_worker_ref = MemTopicWorker::create(queue_worker.clone());
        let mut topic_worker = topic_worker_ref.borrow_mut();
        let sub1 = topic_worker.subscribe("a").unwrap();
        let sub2 = topic_worker.subscribe("a").unwrap();

        assert_ne!(sub1, sub2);

        let validation = "ockam";

        topic_worker.publish("a", validation.to_msg().unwrap());

        let messages1 = topic_worker.consume_messages(&sub1);
        assert_eq!(1, messages1.len());

        let messages2 = topic_worker.consume_messages(&sub2);
        assert_eq!(1, messages2.len());

        topic_worker.unsubscribe(&sub1);
        topic_worker.unsubscribe(&sub2);
    }*/
/*
    #[test]
    fn test_topic_worker_unsubscribe() {
        let queue_worker = MemQueueWorker::create_unbound("q1");
        let topic_worker_ref = MemTopicWorker::create(queue_worker.clone());
        let mut topic_worker = topic_worker_ref.borrow_mut();
        let topic = "a";

        let sub1 = topic_worker.subscribe(topic).unwrap();
        topic_worker.unsubscribe(&sub1);

        topic_worker.publish(topic, "no subscribers".to_msg().unwrap());

        let empty_queue = queue_worker.borrow_mut().get_queue(topic).unwrap();
        assert!(!empty_queue.borrow().has_messages())
    }
*/
   /* #[test]
    fn test_topic_worker_consume_messages() {
        let queue_worker = MemQueueWorker::create_unbound("q1");
        let topic_worker_ref = MemTopicWorker::create(queue_worker.clone());
        let mut topic_worker = topic_worker_ref.borrow_mut();
        let topic = "a";

        let sub = topic_worker.subscribe(topic).unwrap();

        let limit = 100;
        for i in 0..limit {
            topic_worker.publish(topic, (&i.to_string()).to_msg().unwrap());
        }

        let messages = topic_worker.consume_messages(&sub);

        assert_eq!(limit, messages.len());
        for i in 0..limit {
            let validation = i.to_string().as_bytes().to_vec();
            assert_eq!(validation, messages.get(i).unwrap().body);
        }
    }
*/
/*    #[test]
    fn topic_tdd() {
        use crate::topic::*;

        let queue_worker = MemQueueWorker::create_unbound("q1");
        let topic_worker = MemTopicWorker::create(queue_worker);

        let mut tw = topic_worker.borrow_mut();
        let sub = tw.subscribe("test").unwrap();

        tw.publish("test", "ockam".to_msg().unwrap());

        tw.publish("test", "ockam!".to_msg().unwrap());

        let messages = tw.consume_messages(&sub);
        assert_eq!(2, messages.len());

        tw.unsubscribe(&sub);
    }
}*/
