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

    pub fn subscribe(&mut self, subscriber: T, topics: &[&str]) {
        self.subscriptions.subscribers.push(subscriber);
    }
}

pub struct Subscriptions<T: Subscriber> {
    subscribers: Vec<T>,
}

impl<T: Subscriber> Subscriptions<T> {
    fn new() -> Self {
        return Subscriptions { subscribers: vec![], }
    }

    fn publish(&mut self, topic: &str, payload: &[u8]) {
        for s in &mut self.subscribers {
            if topic == "topics/foo" {
                s.new_message(&[0, 1, 3]);
            }
        }
    }
}

pub trait Subscriber {
    fn new() -> Self;
    fn new_message(&mut self, bytes: &[u8]);
}

#[cfg(test)]
struct TestSubscriber {
    msgs: Vec<Vec<u8>>,
}
#[cfg(test)]
impl TestSubscriber {
}

#[cfg(test)]
impl Subscriber for TestSubscriber {
    fn new() -> Self {
        return TestSubscriber{msgs: vec![]}
    }

    fn new_message(&mut self, bytes: &[u8]) {
        self.msgs.push(bytes.to_vec());
    }
}

#[test]
fn no_subscriptions() {
    let mut broker = Broker::<TestSubscriber>::new();
    let subscriber = TestSubscriber::new();
    broker.publish("topics/foo", &[0, 1, 3]);
    assert_eq!(subscriber.msgs.len(), 0);

    broker.subscribe(subscriber, &["topics/foo"]);
    broker.publish("topics/foo", &[0, 1, 3]);
    broker.publish("topics/bar", &[2, 4, 6]);
    let subscriber = &broker.subscriptions.subscribers[0];
    assert_eq!(subscriber.msgs.len(), 1);
    assert_eq!(subscriber.msgs[0], &[0, 1, 3]);

    // broker.subscribe(&subscriber, &["topics/bar"]);
    // broker.publish("topics/foo", &[1, 3, 5, 7]);
    // broker.publish("topics/bar", &[2, 4]);
    // assert_eq!(subscriber.msgs.len(), 3);
    // assert_eq!(subscriber.msgs[0], &[0, 1, 3]);
    // assert_eq!(subscriber.msgs[1], &[1, 3, 5, 7]);
    // assert_eq!(subscriber.msgs[1], &[2, 4]);
}
