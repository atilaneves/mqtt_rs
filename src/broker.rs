pub struct Broker<T: Subscriber> {
    sub: T,
}

impl<T: Subscriber> Broker<T> {
    pub fn new() -> Self {
        Broker{ sub: T::new()}
    }

    pub fn publish(&self, topic: &str, payload: &[u8]) {
    }
}

pub trait Subscriber {
    fn new() -> Self;
    fn new_msg(&mut self, bytes: &[u8]);
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

    fn new_msg(&mut self, bytes: &[u8]) {
        self.msgs.push(bytes.to_vec());
    }
}

#[test]
fn no_subscriptions() {
    let broker = Broker::<TestSubscriber>::new();
    broker.publish("topics/foo", &[0, 1, 3]);
    assert_eq!(broker.sub.msgs.len(), 0);

}
