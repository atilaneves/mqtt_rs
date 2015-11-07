use std::rc::{Rc};
use std::cell::{RefCell};
use std::collections::HashMap;


pub struct Broker<T: Subscriber> {
    subscriptions: Subscriptions<T>,
}

impl<T: Subscriber> Broker<T> {
    pub fn new() -> Self {
        Broker{ subscriptions: Subscriptions::<T>::new() }
    }

    pub fn publish(&mut self, topic: &str, payload: &[u8]) {
        self.subscriptions.publish(topic, payload);
    }

    pub fn subscribe(&mut self, subscriber: Rc<RefCell<T>>, topics: &[&str]) {
        self.subscriptions.subscribe(subscriber, &topics)
    }
}

pub struct Subscriptions<T: Subscriber> {
    subscribers: Vec<Rc<RefCell<T>>>,
}

impl<T: Subscriber> Subscriptions<T> {
    fn new() -> Self {
        return Subscriptions { subscribers: vec![], }
    }

    fn subscribe(&mut self, subscriber: Rc<RefCell<T>>, topics: &[&str]) {
        let subscriber = self.add_or_find(subscriber);
        subscriber.borrow_mut().append_topics(topics);
    }

    fn add_or_find(&mut self, subscriber: Rc<RefCell<T>>) -> Rc<RefCell<T>> {
        for sub in &self.subscribers {
            let sub1 = sub.clone();
            let sub2 = subscriber.clone();
            //this is horrible
            //borrow doesn't return T, it returns Ref<T>
            //since Ref<T> itself doesn't satisfy the template contraint, we need to deref
            //it to get the to the T inside. And since is_same takes borrows, we reapply
            //the ampersand
            if is_same(&*sub1.borrow(), &*sub2.borrow()) {
                return sub2;
            }
        }

        self.subscribers.push(subscriber);
        self.subscribers[self.subscribers.len() - 1].clone()
    }

    fn publish(&mut self, topic: &str, payload: &[u8]) {
        for subscriber in &mut self.subscribers {
            let subscriber = subscriber.clone();
            let mut subscriber = subscriber.borrow_mut();

            for _ in subscriber.topics().iter().filter(|&t| topic_matches(&topic, &t)) {
                subscriber.new_message(payload);
            }
        }
    }
}

//is same identity
fn is_same<T>(lhs: &T, rhs: &T) -> bool {
    return lhs as *const T as i64 == rhs as *const T as i64
}

fn topic_matches(pub_topic: &str, sub_topic: &str) -> bool {
    pub_topic == sub_topic
}

pub trait Subscriber {
    fn new_message(&mut self, bytes: &[u8]);
    fn append_topics(&mut self, topics: &[&str]);
    fn topics(&self) -> Vec<String>;
}

pub trait SubscriberT {
    fn new_message(&mut self, bytes: &[u8]);
}

struct Subscription<T: SubscriberT> {
    subscriber: Rc<RefCell<T>>,
    //topic: String,
}

impl<T: SubscriberT> Subscription<T> {
    fn new(subscriber: Rc<RefCell<T>>/*, topic: &str*/) -> Self {
        Subscription { subscriber: subscriber.clone()}//, topic: topic.to_string() }
    }
}

struct Node<T: SubscriberT> {
    children: HashMap<String, Node<T>>,
    leaves: Vec<Subscription<T>>,
}

impl<T: SubscriberT> Node<T> {
    fn new() -> Self {
        Node { children: HashMap::new(), leaves: vec![] }
    }

    fn add_subscription(&mut self, subscription: Subscription<T>) {
        self.leaves.push(subscription);
    }
}

pub struct SubscriptionsT<T: SubscriberT> {
    tree: Node<T>,
}


impl<T: SubscriberT> SubscriptionsT<T> {
    pub fn new() -> Self {
        SubscriptionsT { tree: Node::new() }
    }

    pub fn subscribe(&mut self, subscriber: Rc<RefCell<T>>, topics: &[&str]) {
        for topic in topics {
            let sub_parts : Vec<&str> = topic.split("/").collect();
            Self::ensure_node_exists(&sub_parts, &mut self.tree);
            Self::add_subscription_to_node(&mut self.tree, subscriber.clone(), &sub_parts);
        }
    }

    fn ensure_node_exists(sub_parts: &[&str], node: &mut Node<T>) {
        if sub_parts.len() == 0 {
            return;
        }

        let part = &sub_parts[0][..];
        if !node.children.contains_key(part) {
            node.children.insert(sub_parts[0].to_string(), Node::new());
        }

        Self::ensure_node_exists(&sub_parts[1..], node.children.get_mut(part).unwrap());
    }

    fn add_subscription_to_node(tree: &mut Node<T>, subscriber: Rc<RefCell<T>>, sub_parts: &[&str]) {
        if sub_parts.len() < 1 {
            panic!("oops");
        }

        let part = &sub_parts[0][..];
        let node = tree.children.get_mut(part).unwrap();
        let sub_parts = &sub_parts[1..];

        if sub_parts.len() == 0 {
            node.add_subscription(Subscription::new(subscriber.clone()));
        } else {
            Self::add_subscription_to_node(node, subscriber, sub_parts);
        }
    }

    pub fn publish(&self, topic: &str, payload: &[u8]) {
        let pub_parts : Vec<&str> = topic.split("/").collect();
        Self::publish_impl(&self.tree, &pub_parts, &payload);
    }

    fn publish_impl(tree: &Node<T>, pub_parts: &[&str], payload: &[u8]) {
        if pub_parts.len() < 1 {
            panic!("oops");
        }

        let part = &pub_parts[0][..];
        let node = tree.children.get(part);
        let pub_parts = &pub_parts[1..];

        match node {
            None => {
                return; //didn't find a subscriber
            },
            Some(node) => {
                if pub_parts.len() == 0 {
                    for subscription in &node.leaves {
                        let subscriber = subscription.subscriber.clone();
                        subscriber.borrow_mut().new_message(payload);
                    }
                } else {
                    Self::publish_impl(node, pub_parts, payload);
                }
            }
        }
    }
}

#[cfg(test)]
struct TestSubscriberT {
    msgs: Vec<Vec<u8>>,
}

#[cfg(test)]
impl TestSubscriberT {
    fn new() -> Self {
        return TestSubscriberT{msgs: vec![]}
    }
}

#[cfg(test)]
impl SubscriberT for TestSubscriberT {
    fn new_message(&mut self, bytes: &[u8]) {
        println!("new message with bytes {:?}", bytes);
        self.msgs.push(bytes.to_vec());
    }
}

#[test]
fn test_subt() {
    let mut broker = SubscriptionsT::<TestSubscriberT>::new();
    let sub_ref = Rc::new(RefCell::new(TestSubscriberT::new()));
    let subscriber = sub_ref.clone();
    broker.publish("topics/foo", &[0, 1, 2]);
    assert_eq!(subscriber.borrow().msgs.len(), 0);

    broker.subscribe(subscriber.clone(), &["topics/foo"]);
    broker.publish("topics/foo", &[0, 1, 9]); //should get this
    broker.publish("topics/bar", &[2, 4, 6]); //shouldn't get this
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

#[cfg(test)]
struct TestSubscriber {
    msgs: Vec<Vec<u8>>,
    topics: Vec<String>,
}
#[cfg(test)]
impl TestSubscriber {
    fn new() -> Self {
        return TestSubscriber{msgs: vec![], topics: vec![]}
    }
}

#[cfg(test)]
impl Subscriber for TestSubscriber {

    fn new_message(&mut self, bytes: &[u8]) {
        self.msgs.push(bytes.to_vec());
    }

    fn append_topics(&mut self, topics: &[&str]) {
        let mut new_topics = topics.to_vec().into_iter().map(|t| t.to_string()).collect();
        self.topics.append(&mut new_topics);
    }

    fn topics(&self) -> Vec<String> {
        self.topics.clone()
    }
}


#[test]
fn test_subscribe() {
    let mut broker = Broker::<TestSubscriber>::new();
    let sub_ref = Rc::new(RefCell::new(TestSubscriber::new()));
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
