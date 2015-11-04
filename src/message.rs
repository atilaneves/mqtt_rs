use std::mem;

#[derive(PartialEq, Debug)]
enum MqttType {
    Reserved = 0,
    Connect,
    ConnAck,
    Publish,
    PubAck,
    PubRec,
    PubRel,
    PubComp,
    Subscribe,
    SubAck,
    Unsubscribe,
    UnsubAck,
    PingReq,
    PingResp,
    Disconnect,
}

fn message_type(bytes: &[u8]) -> MqttType {
    unsafe { mem::transmute((bytes[0] & 0xf0) >> 4) }
}

#[test]
fn connect_type() {
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
    assert_eq!(message_type(connect_bytes), MqttType::Connect);
}

#[test]
fn ping_type() {
    let ping_bytes = &[0xc0u8, 0][0..];
    assert_eq!(message_type(ping_bytes), MqttType::PingReq);
}
