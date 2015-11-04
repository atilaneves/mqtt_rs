use std::rc::{Rc};
use std::cell::{RefCell};


pub struct Broker<T: Subscriber> {
    subscriptions: Subscriptions<T>,
}

impl<T: Subscriber> Broker<T> {
    pub fn new() -> Self {
        Broker{ subscriptions: Subscriptions::<T>::new()}
    }

    pub fn publish(&mut self, topic: &str, payload: &[u8]) {
        self.subscriptions.publish(topic, payload);
    }

    pub fn subscribe(&mut self, subscriber: Rc<RefCell<T>>, topics: &[&str]) {
        for sub in &self.subscriptions.subscribers {
            let sub1 = sub.clone();
            let sub2 = subscriber.clone();
            if sub1.borrow().id() == sub2.borrow().id() {
                sub2.borrow_mut().append_topics(topics);
                return;
            }
        }
        self.subscriptions.subscribers.push(subscriber);
        let lesub = self.subscriptions.subscribers[self.subscriptions.subscribers.len() - 1].clone();
        lesub.borrow_mut().append_topics(topics);
    }
}

pub struct Subscriptions<T: Subscriber> {
    subscribers: Vec<Rc<RefCell<T>>>,
}

impl<T: Subscriber> Subscriptions<T> {
    fn new() -> Self {
        return Subscriptions { subscribers: vec![], }
    }

    fn publish(&mut self, topic: &str, payload: &[u8]) {
        for s in &mut self.subscribers {
            let sub_clone = s.clone();
            let mut s = sub_clone.borrow_mut();

            for t in s.topics() {
                if t == topic {
                    s.new_message(payload);
                }
            }
        }
    }
}

pub trait Subscriber {
    fn new_message(&mut self, bytes: &[u8]);
    fn id(&self) -> i64;
    fn append_topics(&mut self, topics: &[&str]);
    fn topics(&self) -> Vec<String>;
}

#[cfg(test)]
struct TestSubscriber {
    msgs: Vec<Vec<u8>>,
    id: i64,
    topics: Vec<String>,
}
#[cfg(test)]
impl TestSubscriber {
    fn new(id: i64) -> Self {
        return TestSubscriber{msgs: vec![], id: id, topics: vec![]}
    }
}

#[cfg(test)]
impl Subscriber for TestSubscriber {

    fn new_message(&mut self, bytes: &[u8]) {
        self.msgs.push(bytes.to_vec());
    }

    fn id(&self) -> i64 {
        return self.id;
    }

    fn append_topics(&mut self, topics: &[&str]) {
        let mut new_topics = topics.to_vec().into_iter().map(|t| t.to_string()).collect();
        self.topics.append(&mut new_topics);
        println!("after append, len is {}", self.topics.len());
    }

    fn topics(&self) -> Vec<String> {
        println!("self topics len: {}", self.topics.len());
        self.topics.clone()
    }
}

#[test]
fn no_subscriptions() {
    let mut broker = Broker::<TestSubscriber>::new();
    let sub_ref = Rc::new(RefCell::new(TestSubscriber::new(42)));
    let subscriber = sub_ref.clone();
    broker.publish("topics/foo", &[0, 1, 2]);
    assert_eq!(subscriber.borrow().msgs.len(), 0);

    broker.subscribe(subscriber.clone(), &["topics/foo"]);
    broker.publish("topics/foo", &[0, 1, 9]);
    broker.publish("topics/bar", &[2, 4, 6]);
    assert_eq!(subscriber.borrow().msgs.len(), 1);
    assert_eq!(subscriber.borrow().msgs[0], &[0, 1, 9]);

    broker.subscribe(subscriber.clone(), &["topics/bar"]);
    broker.publish("topics/foo", &[1, 3, 5, 7]);
    broker.publish("topics/bar", &[2, 4]);
    assert_eq!(subscriber.borrow().msgs.len(), 3);
    assert_eq!(subscriber.borrow().msgs[0], &[0, 1, 9]);
    assert_eq!(subscriber.borrow().msgs[1], &[1, 3, 5, 7]);
    assert_eq!(subscriber.borrow().msgs[2], &[2, 4]);
}
