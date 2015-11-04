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

    pub fn new_message<'a, C: Client<'a>>(&mut self, client: &mut C, bytes: &'a [u8]) {
        println!("Server #{}", self.id);
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
}

pub trait Client<'a> {
    fn send(&mut self, bytes: &'a [u8]);
}

struct TestClient<'a> {
    last_msg: &'a [u8],
}

impl<'a> TestClient<'a> {
    fn new() -> Self {
        TestClient { last_msg: &[] }
    }
}

impl<'a> Client<'a> for TestClient<'a> {
    fn send(&mut self, bytes: &'a [u8]) {
        self.last_msg = bytes;
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
    assert_eq!(client.last_msg, &CONNACK_OK);
}


#[test]
fn test_ping() {
    let ping_bytes =  &[0xc0u8, 0][0..];

    let mut server = Server::new();
    let mut client = TestClient::new();

    server.new_message(&mut client, ping_bytes);

    assert_eq!(client.last_msg, &PING_RESP);
}
