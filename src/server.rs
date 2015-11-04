struct Server {
    dummy: i32,
}

static CONNACK_OK : [u8; 4] = [32, 2, 0, 0];

impl Server {
    fn new() -> Self {
        Server { dummy: 5 }
    }

    fn new_client(&mut self, client: &MqttConnection) {
    }

    fn new_message(&mut self, client: &mut MqttConnection, bytes: &[u8]) {
        client.send(&CONNACK_OK)
    }

}


struct MqttConnection<'a> {
    last_msg: &'a [u8],
}

impl<'a> MqttConnection<'a> {
    fn new() -> Self {
        MqttConnection { last_msg: &[] }
    }

    fn send(&mut self, bytes: &'a [u8]) {
        self.last_msg = bytes;
    }
}


#[test]
fn test_connect() {
    let mut server = Server::new();
    let mut client = MqttConnection::new();
    server.new_client(&client);

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

    server.new_message(&mut client, connect_bytes);
    assert_eq!(client.last_msg, &[32, 2, 0, 0]);
}
