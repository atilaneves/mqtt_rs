use message;
use broker;

use std::rc::{Rc};
use std::cell::{RefCell};

pub struct Server<T: broker::Subscriber> {
    broker: broker::Broker<T>,
}


static CONNACK_OK : [u8; 4] = [32, 2, 0, 0];
static PING_RESP : [u8; 2] = [0xd0, 0];


impl<T: broker::Subscriber> Server<T> {
    pub fn new(use_cache: bool) -> Self {
        Server { broker: broker::Broker::new(use_cache) }
    }

    fn new_message(&mut self, client: Rc<RefCell<T>>, bytes: &[u8]) -> bool {
        let message_type = message::message_type(bytes);
        match message_type {
            message::MqttType::Connect => {
                let client = client.clone();
                client.borrow_mut().new_message(&CONNACK_OK);
                true
            }
            message::MqttType::PingReq => {
                let client = client.clone();
                client.borrow_mut().new_message(&PING_RESP);
                true
            }
            message::MqttType::Subscribe => {
                for topic in message::subscribe_topics(bytes) {
                    self.broker.subscribe(client.clone(), &topic[..]);
                }

                let msg_id = message::subscribe_msg_id(bytes);
                let qos: u8 = 0;
                let client = client.clone();
                client.borrow_mut().new_message(&[0x90u8, 3, 0, msg_id as u8, qos][..]);
                true
            }
            message::MqttType::Publish => {
                self.broker.publish(&message::publish_topic(bytes)[..], bytes);
                true
            }
            message::MqttType::Disconnect => {
                false
            }
            _ => {
                println!("Bad message {:?}", &bytes);
                true
            }
        }
    }

    pub fn unsubscribe_all(&mut self, client: Rc<RefCell<T>>) {
        self.broker.unsubscribe_all(client);
    }
}

pub struct Stream {
    buffer: Vec<u8>,
    bytes_start: usize, //the start of the next byte window
}

impl Stream {
    pub fn new() -> Self {
        Stream { buffer: vec![0; 1024 * 1024], bytes_start: 0 }
    }

    pub fn buffer(&mut self) -> &mut [u8] {
        &mut self.buffer[self.bytes_start .. ]
    }

    pub fn total_buffer_len(&self) -> usize {
        self.buffer.len()
    }

    pub fn handle_messages<T: broker::Subscriber>(&mut self, bytes_read:
                                                  usize, server: &mut Server<T>,
                                                  client: Rc<RefCell<T>>) -> bool {
        let vec : Vec<u8>;
        let mut res = true;
        {
            let mut slice = &self.buffer[0 .. self.bytes_start + bytes_read];
            const HEADER_LEN: usize = 2;
            let mut total_len = HEADER_LEN;
            while slice.len() >= total_len {

                total_len = message::total_length(slice);
                if total_len > slice.len() {
                    self.bytes_start += bytes_read;
                    return true;
                }
                let msg = &slice[0 .. total_len];
                slice = &slice[total_len..];
                self.bytes_start += total_len;
                res = res && server.new_message(client.clone(), msg);

                if slice.len() >= 2 {
                    total_len = message::total_length(slice);
                }
            }
            vec = slice.to_vec();
        }

        //shift everything to the beginning of the buffer
        for i in 0 .. vec.len() {
            self.buffer[i] = vec[i];
        }

        self.bytes_start = vec.len();
        res
    }
}

#[cfg(test)]
struct TestClient {
    msgs: Vec<Vec<u8>>,
    payloads: Vec<Vec<u8>>,
}

#[cfg(test)]
impl TestClient {
    fn new() -> Self {
        TestClient { msgs: vec![], payloads: vec![] }
    }

    fn last_msg(&self) -> &[u8] {
        self.msgs.last().expect("TestClient has no last message")
    }

    fn read(&self, buffer: &mut [u8], bytes: &[u8]) -> usize {
        for i in 0..bytes.len() {
            buffer[i] = bytes[i];
        }
        bytes.len()
    }
}

#[cfg(test)]
impl broker::Subscriber for TestClient {
    fn new_message(&mut self, bytes: &[u8]) {
        self.msgs.push(bytes.to_vec());
        if message::message_type(bytes) == message::MqttType::Publish {
            self.payloads.push(message::publish_payload(bytes).to_vec());
        }
    }
}


#[test]
fn test_connect() {
    let connect_bytes = &[
        0x10u8, 0x2a, // fixed header
        0x00, 0x06, 'M' as u8, 'Q' as u8, 'I' as u8, 's' as u8, 'd' as u8, 'p' as u8,
        0x03, // protocol version
        0xcc, // connection flags 1100111x user, pw, !wr, w(01), w, !c, x
        0x00, 0x0a, // keepalive of 100
        0x00, 0x03, 'c' as u8, 'i' as u8, 'd' as u8, // client ID
        0x00, 0x04, 'w' as u8, 'i' as u8, 'l' as u8, 'l' as u8, // will topic
        0x00, 0x04, 'w' as u8, 'm' as u8, 's' as u8, 'g' as u8, // will msg
        0x00, 0x07, 'g' as u8, 'l' as u8, 'i' as u8, 'f' as u8, 't' as u8, 'e' as u8, 'l' as u8, // username
        0x00, 0x02, 'p' as u8, 'w' as u8, // password
        ][0..];

    let mut server = Server::<TestClient>::new(false);
    let client = Rc::new(RefCell::new(TestClient::new()));

    server.new_message(client.clone(), connect_bytes);
    assert_eq!(client.borrow().msgs.len(), 1);
    assert_eq!(client.borrow().last_msg(), &CONNACK_OK);
}


#[test]
fn test_ping() {
    let ping_bytes =  &[0xc0u8, 0][0..];

    let mut server = Server::<TestClient>::new(false);
    let client = Rc::new(RefCell::new(TestClient::new()));
    let client = client.clone();

    server.new_message(client.clone(), ping_bytes);

    assert_eq!(client.borrow().last_msg(), &PING_RESP);
}


#[test]
fn test_pings_all_at_once() {
    let ping_bytes = &[0xc0u8, 0, 0xc0, 0, 0xc0, 0, 0xc0, 0][0..];

    let mut server = Server::<TestClient>::new(false);
    let mut stream = Stream::new();
    let client = Rc::new(RefCell::new(TestClient::new()));
    let client = client.clone();

    let bytes_read = client.borrow_mut().read(stream.buffer(), ping_bytes);
    stream.handle_messages(bytes_read, &mut server, client.clone());

    let client = client.clone();
    assert_eq!(client.borrow().msgs.len(), 4);
    for msg in &client.borrow().msgs {
        assert_eq!(msg, &PING_RESP);
    }
}

#[test]
fn test_pings_multiple_time_unbroken() {
    let ping_bytes = &[0xc0u8, 0, 0xc0, 0][0..];

    let mut server = Server::<TestClient>::new(false);
    let mut stream = Stream::new();
    let client = Rc::new(RefCell::new(TestClient::new()));
    let client = client.clone();

    let bytes_read = client.borrow_mut().read(stream.buffer(), ping_bytes);
    stream.handle_messages(bytes_read, &mut server, client.clone());
    assert_eq!(client.borrow().msgs.len(), 2);

    let bytes_read = client.borrow_mut().read(stream.buffer(), ping_bytes);
    stream.handle_messages(bytes_read, &mut server, client.clone());
    assert_eq!(client.borrow().msgs.len(), 4);

    for msg in &client.borrow().msgs {
        assert_eq!(msg, &PING_RESP);
    }
}


#[test]
fn test_pings_broken() {
    let ping_fst = &[0xc0u8][0..];
    let ping_snd = &[0u8][0..];

    let mut server = Server::<TestClient>::new(false);
    let mut stream = Stream::new();
    let client = Rc::new(RefCell::new(TestClient::new()));
    let client = client.clone();

    let bytes_read = client.borrow_mut().read(stream.buffer(), ping_fst);
    stream.handle_messages(bytes_read, &mut server, client.clone());
    assert_eq!(client.borrow().msgs.len(), 0);

    let bytes_read = client.borrow_mut().read(stream.buffer(), ping_snd);
    stream.handle_messages(bytes_read, &mut server, client.clone());
    assert_eq!(client.borrow().msgs.len(), 1);

    let bytes_read = client.borrow_mut().read(stream.buffer(), ping_fst);
    stream.handle_messages(bytes_read, &mut server, client.clone());
    assert_eq!(client.borrow().msgs.len(), 1);

    let bytes_read = client.borrow_mut().read(stream.buffer(), ping_snd);
    stream.handle_messages(bytes_read, &mut server, client.clone());
    assert_eq!(client.borrow().msgs.len(), 2);

    for msg in &client.borrow().msgs {
        assert_eq!(msg, &PING_RESP);
    }
}

#[cfg(test)]
fn string_to_bytes(s: &str) -> Vec<u8> {
    let mut vec: Vec<u8> = vec![];
    for c in s.chars() {
        vec.push(c as u8);
    }
    vec
}

#[cfg(test)]
fn subscribe_bytes(topic: &str, msg_id: u16) -> Vec<u8> {
    let mut fixed_header = [0x8cu8, 5 + topic.len() as u8].to_vec();
    let mut msg_part = vec![0, msg_id as u8];
    let mut topic_header = vec![0, topic.len() as u8];
    let mut string_bytes = string_to_bytes(topic);
    let mut bytes = vec![];
    bytes.append(&mut fixed_header);
    bytes.append(&mut msg_part);
    bytes.append(&mut topic_header);
    bytes.append(&mut string_bytes);
    bytes.push(0u8); //qos
    bytes
}

#[test]
fn test_suback_bytes() {
    let subscribe_bytes = &subscribe_bytes("topic", 42)[..];
    let qos: u8 = 0;
    let suback_bytes = &[0x90u8, 3, 0, 42, qos][..];

    let mut server = Server::<TestClient>::new(false);
    let mut stream = Stream::new();
    let client = Rc::new(RefCell::new(TestClient::new()));
    let client = client.clone();

    let bytes_read = client.borrow_mut().read(stream.buffer(), &subscribe_bytes);
    stream.handle_messages(bytes_read, &mut server, client.clone());
    assert_eq!(client.borrow().last_msg(), suback_bytes);
}


#[test]
fn test_subscribe() {
    let mut server = Server::<TestClient>::new(false);
    let mut stream = Stream::new();
    let client = Rc::new(RefCell::new(TestClient::new()));
    let client = client.clone();

    let pub_bytes = vec![
        0x3c, 0x0d, //fixed header
        0x00, 0x05, 'f' as u8, 'i' as u8, 'r' as u8, 's' as u8, 't' as u8,//topic name
        0x00, 0x21, //message ID
        'b' as u8, 'o' as u8, 'r' as u8, 'g' as u8, //payload
        ];
    let bytes_read = client.borrow_mut().read(stream.buffer(), &pub_bytes);
    assert_eq!(stream.handle_messages(bytes_read, &mut server, client.clone()), true);
    assert_eq!(client.borrow().payloads.len(), 0);

    let sub_bytes = vec![
        0x8b, 0x13, //fixed header
        0x00, 0x21, //message ID
        0x00, 0x05, 'f' as u8, 'i' as u8, 'r' as u8, 's' as u8, 't' as u8,
        0x01, //qos
        0x00, 0x06, 's' as u8, 'e' as u8, 'c' as u8, 'o' as u8, 'n' as u8, 'd' as u8,
        0x02, //qos
        ];
    let bytes_read = client.borrow_mut().read(stream.buffer(), &sub_bytes);
    assert_eq!(stream.handle_messages(bytes_read, &mut server, client.clone()), true);

    let pub_bytes = vec![
        0x3c, 0x0d, //fixed header
        0x00, 0x05, 'f' as u8, 'i' as u8, 'r' as u8, 's' as u8, 't' as u8,//topic name
        0x00, 0x21, //message ID
        'b' as u8, 'o' as u8, 'r' as u8, 'g' as u8, //payload
        ];
    let bytes_read = client.borrow_mut().read(stream.buffer(), &pub_bytes);
    assert_eq!(stream.handle_messages(bytes_read, &mut server, client.clone()), true);

    let pub_bytes = vec![
        0x3c, 0x0d, //fixed header
        0x00, 0x06, 's' as u8, 'e' as u8, 'c' as u8, 'o' as u8, 'n' as u8, 'd' as u8,//topic name
        0x00, 0x21, //message ID
        'f' as u8, 'o' as u8, 'o' as u8,//payload
        ];
    let bytes_read = client.borrow_mut().read(stream.buffer(), &pub_bytes);
    assert_eq!(stream.handle_messages(bytes_read, &mut server, client.clone()), true);

    let pub_bytes = vec![
        0x3c, 0x0c, //fixed header
        0x00, 0x05, 't' as u8, 'h' as u8, 'i' as u8, 'r' as u8, 'd' as u8,//topic name
        0x00, 0x21, //message ID
        'f' as u8, 'o' as u8, 'o' as u8,//payload
        //--
        0xe0, 0, //disconnect
        ];
    let bytes_read = client.borrow_mut().read(stream.buffer(), &pub_bytes);
    //false since last msg is disconnect
    assert_eq!(stream.handle_messages(bytes_read, &mut server, client.clone()), false);

    assert_eq!(client.borrow().payloads, vec![b"borg".to_vec(), b"foo".to_vec()]);
}

#[test]
fn test_bug1() {
    let mut server = Server::<TestClient>::new(false);
    let mut stream = Stream::new();
    let client = Rc::new(RefCell::new(TestClient::new()));
    let client = client.clone();

    //printed from when the bug occurred
    let first = vec![
        48, 30, 0, 12, 108, 111, 97, 100, 116, 101, 115, 116, 47, 49, 54, 54
            ];
    let bytes_read = client.borrow_mut().read(stream.buffer(), &first);
    stream.handle_messages(bytes_read, &mut server, client.clone());
    assert_eq!(client.borrow().payloads.len(), 0);
}



#[test]
fn test_publish_in_two_msgs() {
    let mut server = Server::<TestClient>::new(false);
    let mut stream = Stream::new();
    let client = Rc::new(RefCell::new(TestClient::new()));
    let client = client.clone();

    let sub_bytes = vec![
        0x8b, 0x13, //fixed header
        0x00, 0x21, //message ID
        0x00, 0x05, 'f' as u8, 'i' as u8, 'r' as u8, 's' as u8, 't' as u8,
        0x01, //qos
        0x00, 0x06, 's' as u8, 'e' as u8, 'c' as u8, 'o' as u8, 'n' as u8, 'd' as u8,
        0x02, //qos
        ];
    let bytes_read = client.borrow_mut().read(stream.buffer(), &sub_bytes);
    assert_eq!(stream.handle_messages(bytes_read, &mut server, client.clone()), true);

    //1st part of message
    let pub_bytes = vec![
        0x3c, 0x0d, //fixed header
        0x00, 0x05, 'f' as u8, 'i' as u8, 'r' as u8, 's' as u8, 't' as u8,//topic name
        ];
    let bytes_read = client.borrow_mut().read(stream.buffer(), &pub_bytes);
    assert_eq!(stream.handle_messages(bytes_read, &mut server, client.clone()), true);
    assert_eq!(client.borrow().payloads.len(), 0);

    //2nd part of message
    let pub_bytes = vec![
        0x00, 0x21, //message ID
        'b' as u8, 'o' as u8, 'r' as u8, 'g' as u8, //payload
        ];
    let bytes_read = client.borrow_mut().read(stream.buffer(), &pub_bytes);
    assert_eq!(stream.handle_messages(bytes_read, &mut server, client.clone()), true);
    assert_eq!(client.borrow().payloads.len(), 1);
}
