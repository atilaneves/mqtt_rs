use std::rc::{Rc};
use std::cell::{RefCell};
use std::collections::HashMap;

//is same identity
fn is_same<T>(lhs: &T, rhs: &T) -> bool {
    return lhs as *const T as i64 == rhs as *const T as i64
}

pub trait Subscriber {
    fn new_message(&mut self, bytes: &[u8]);
}

pub struct Broker<T: Subscriber> {
    tree: Node<T>,
}

struct Node<T: Subscriber> {
    children: HashMap<String, Node<T>>,
    leaves: Vec<Subscription<T>>,
}

struct Subscription<T: Subscriber> {
    subscriber: Rc<RefCell<T>>,
}

impl<T: Subscriber> Subscription<T> {
    fn new(subscriber: Rc<RefCell<T>>) -> Self {
        Subscription { subscriber: subscriber.clone()}
    }
}

impl<T: Subscriber> Node<T> {
    fn new() -> Self {
        Node { children: HashMap::new(), leaves: vec![] }
    }

    fn add_subscription(&mut self, subscription: Subscription<T>) {
        self.leaves.push(subscription);
    }
}

impl<T: Subscriber> Broker<T> {
    pub fn new() -> Self {
        Broker { tree: Node::new() }
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
struct TestSubscriber {
    msgs: Vec<Vec<u8>>,
}

#[cfg(test)]
impl TestSubscriber {
    fn new() -> Self {
        return TestSubscriber{msgs: vec![]}
    }
}

#[cfg(test)]
impl Subscriber for TestSubscriber {
    fn new_message(&mut self, bytes: &[u8]) {
        println!("new message with bytes {:?}", bytes);
        self.msgs.push(bytes.to_vec());
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
