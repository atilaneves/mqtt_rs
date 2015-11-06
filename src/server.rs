use message;


pub struct Server {
    id: i32,
}


static CONNACK_OK : [u8; 4] = [32, 2, 0, 0];
static PING_RESP : [u8; 2] = [0xd0, 0];


impl Server {
    pub fn new() -> Self {
        Server { id: 4 }
    }

    fn new_message<C: Client>(&mut self, client: &mut C, bytes: &[u8]) {
        let message_type = message::message_type(bytes);
        match message_type {
            message::MqttType::Connect => {
                client.send(&CONNACK_OK);
            },
            message::MqttType::PingReq => {
                client.send(&PING_RESP);
            },
            message::MqttType::Subscribe => {
                let msg_id = message::subscribe_msg_id(bytes);
                let qos: u8 = 0;
                client.send(&[0x90u8, 3, 0, msg_id as u8, qos][..]);
            },
        }
    }
}

pub struct Stream {
    buffer: Vec<u8>,
    bytes_start: usize, //the start of the next byte window
}

impl Stream {
    pub fn new() -> Self {
        Stream { buffer: vec![0; 1024], bytes_start: 0 }
    }

    pub fn buffer(&mut self) -> &mut [u8] {
        &mut self.buffer[self.bytes_start .. ]
    }

    pub fn handle_messages<C: Client>(&mut self, bytes_read: usize, server: &mut Server, client: &mut C) {
        let vec : Vec<u8>;
        {
            let mut slice = &self.buffer[0 .. self.bytes_start + bytes_read];
            const HEADER_LEN: usize = 2;
            let mut total_len = HEADER_LEN;
            while slice.len() >= total_len {
                total_len = message::remaining_length(slice) + HEADER_LEN;
                let msg = &slice[0 .. total_len];
                slice = &slice[total_len..];
                self.bytes_start += total_len;
                server.new_message(client, msg);
            }
            vec = slice.to_vec();
        }

        //shift everything to the beginning of the buffer
        for i in 0 .. vec.len() {
            self.buffer[i] = vec[i];
        }

        self.bytes_start = vec.len();
    }
}


pub trait Client {
    fn send(&mut self, bytes: &[u8]);
}

#[cfg(test)]
struct TestClient {
    msgs: Vec<Vec<u8>>,
}

#[cfg(test)]
impl TestClient {
    fn new() -> Self {
        TestClient { msgs: vec!() }
    }

    fn last_msg(&self) -> &[u8] {
        self.msgs.last().unwrap()
    }

    fn read(&self, buffer: &mut [u8], bytes: &[u8]) -> usize {
        for i in 0..bytes.len() {
            buffer[i] = bytes[i];
        }
        bytes.len()
    }
}

#[cfg(test)]
impl Client for TestClient {
    fn send(&mut self, bytes: &[u8]) {
        self.msgs.push(bytes.to_vec());
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

    let mut server = Server::new();
    let mut client = TestClient::new();

    server.new_message(&mut client, connect_bytes);
    assert_eq!(client.last_msg(), &CONNACK_OK);
}


#[test]
fn test_ping() {
    let ping_bytes =  &[0xc0u8, 0][0..];

    let mut server = Server::new();
    let mut client = TestClient::new();

    server.new_message(&mut client, ping_bytes);

    assert_eq!(client.last_msg(), &PING_RESP);
}


#[test]
fn test_pings_all_at_once() {
    let ping_bytes = &[0xc0u8, 0, 0xc0, 0, 0xc0, 0, 0xc0, 0][0..];

    let mut server = Server::new();
    let mut client = TestClient::new();
    let mut stream = Stream::new();

    let bytes_read = client.read(stream.buffer(), ping_bytes);
    stream.handle_messages(bytes_read, &mut server, &mut client);

    assert_eq!(client.msgs.len(), 4);
    for msg in client.msgs {
        assert_eq!(msg, &PING_RESP);
    }
}

#[test]
fn test_pings_multiple_time_unbroken() {
    let ping_bytes = &[0xc0u8, 0, 0xc0, 0][0..];

    let mut server = Server::new();
    let mut client = TestClient::new();
    let mut stream = Stream::new();

    let bytes_read = client.read(stream.buffer(), ping_bytes);
    stream.handle_messages(bytes_read, &mut server, &mut client);
    assert_eq!(client.msgs.len(), 2);

    let bytes_read = client.read(stream.buffer(), ping_bytes);
    stream.handle_messages(bytes_read, &mut server, &mut client);
    assert_eq!(client.msgs.len(), 4);

    for msg in client.msgs {
        assert_eq!(msg, &PING_RESP);
    }
}


#[test]
fn test_pings_broken() {
    let ping_fst = &[0xc0u8][0..];
    let ping_snd = &[0u8][0..];

    let mut server = Server::new();
    let mut client = TestClient::new();
    let mut stream = Stream::new();

    let bytes_read = client.read(stream.buffer(), ping_fst);
    stream.handle_messages(bytes_read, &mut server, &mut client);
    assert_eq!(client.msgs.len(), 0);

    let bytes_read = client.read(stream.buffer(), ping_snd);
    stream.handle_messages(bytes_read, &mut server, &mut client);
    assert_eq!(client.msgs.len(), 1);

    let bytes_read = client.read(stream.buffer(), ping_fst);
    stream.handle_messages(bytes_read, &mut server, &mut client);
    assert_eq!(client.msgs.len(), 1);

    let bytes_read = client.read(stream.buffer(), ping_snd);
    stream.handle_messages(bytes_read, &mut server, &mut client);
    assert_eq!(client.msgs.len(), 2);

    for msg in client.msgs {
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
fn test_subscribe() {
    let subscribe_bytes = &subscribe_bytes("topic", 42)[..];
    let qos: u8 = 0;
    let suback_bytes = &[0x90u8, 3, 0, 42, qos][..];

    let mut server = Server::new();
    let mut client = TestClient::new();
    let mut stream = Stream::new();
    let bytes_read = client.read(stream.buffer(), &subscribe_bytes);
    stream.handle_messages(bytes_read, &mut server, &mut client);
    assert_eq!(client.last_msg(), suback_bytes);
}
