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

    pub fn new_message<C: Client>(&mut self, client: &mut C, bytes: &[u8]) {
        let message_type = message::message_type(bytes);
        match message_type {
            message::MqttType::Connect => {
                client.send(&CONNACK_OK);
            },
            message::MqttType::PingReq => {
                client.send(&PING_RESP);
            },
            _ => panic!("Unknown message type")
        }
    }

    pub fn new_bytes<C: Client>(&mut self, client: &mut C, bytes: &[u8]) {
        let mut slice = bytes;

        while slice.len() > 0 {
            let remaining_len = message::remaining_length(bytes);
            let total_len = remaining_len + 2;

            if slice.len() >= total_len {
                let msg = &slice[0 .. total_len];
                slice = &slice[total_len ..];
                self.new_message(client, msg);
            } else {
                slice = &[];
            }
        }
    }
}

struct Stream {
    //the reason there's bytes_start and bytes_read is because bytes_read always grows,
    //but bytes_start only moves if full messages have been processed
    buffer: Vec<u8>,
    bytes_start: usize, //the start of the next byte window
    bytes_read: usize,  //bytes read so far
}

impl Stream {
    pub fn new() -> Self {
        Stream { buffer: vec![0; 1024], bytes_start: 0, bytes_read: 0 }
    }

    pub fn buffer(&mut self) -> &mut [u8] {
        &mut self.buffer[self.bytes_read .. ]
    }

    pub fn handle_messages<C: Client>(&mut self, bytes_read: usize, server: &mut Server, client: &mut C) {
        let mut slice = &self.buffer[self.bytes_start .. self.bytes_read + bytes_read];
        const HEADER_LEN: usize = 2;
        let mut total_len = HEADER_LEN;
        while slice.len() >= total_len {
            total_len = message::remaining_length(slice) + HEADER_LEN;
            let msg = &slice[0 .. total_len];
            slice = &slice[total_len..];
            self.bytes_start += total_len;
            server.new_message(client, msg);
        }

        // //shift everything to the beginning of the buffer
        // for i in 0..slice.len() {
        //     self.buffer[i] = slice[i];
        // }

        self.bytes_read += bytes_read;
    }
}


pub trait Client {
    fn send(&mut self, bytes: &[u8]);
}

struct TestClient {
    msgs: Vec<Vec<u8>>,
}

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
