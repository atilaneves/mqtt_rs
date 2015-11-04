use std::mem;

#[derive(PartialEq, Debug)]
pub enum MqttType {
    Reserved = 0,
    Connect = 1,
    // ConnAck = 2,
    // Publish = 3,
    // PubAck = 4,
    // PubRec = 5,
    // PubRel = 6,
    // PubComp = 7,
    Subscribe = 8,
    // SubAck = 9,
    // Unsubscribe = 0xa,
    // UnsubAck = 0xb,
    PingReq = 0xc,
    // PingResp = 0xd,
    // Disconnect = 0xe,
}

pub fn message_type(bytes: &[u8]) -> MqttType {
    unsafe { mem::transmute((bytes[0] & 0xf0) >> 4) }
}

fn connect_bytes() -> Vec<u8> {
    vec!(
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
        )
}

#[test]
fn connect_type() {
    let connect_bytes = connect_bytes();
    assert_eq!(message_type(&connect_bytes), MqttType::Connect);
}

#[test]
fn ping_type() {
    let ping_bytes = &[0xc0u8, 0][0..];
    assert_eq!(message_type(ping_bytes), MqttType::PingReq);
}


pub fn remaining_length(bytes: &[u8]) -> usize {
    if bytes.len() < 2 {
        return 0;
    }

    let bytes = &bytes[1..]; //skip half of the fixed header

    //algorithm straight from the MQTT spec
    let mut multiplier: usize = 1;
    let mut value: usize = 0;
    let mut digit: usize = bytes[0] as usize;

    loop { // do-while would be cleaner...
        value += (digit & 127) * multiplier;
        multiplier *= 128;

        if (digit & 128) == 0 {
            break;
        }

        if bytes.len() > 1 {
            digit = bytes[1..][0] as usize;
        }
    }

    value
}

#[test]
fn connect_len() {
    assert_eq!(remaining_length(&connect_bytes()), 42);
}

#[test]
fn ping_len() {
    let ping_bytes = &[0xc0u8, 0][0..];
    assert_eq!(remaining_length(&ping_bytes), 0);
}

#[test]
fn msg_lens() {
    assert_eq!(remaining_length(&[]), 0);
    assert_eq!(remaining_length(&[0x15, 5]), 5);
    assert_eq!(remaining_length(&[0x27, 7]), 7);
    assert_eq!(remaining_length(&[0x12, 0xc1, 0x02]), 321);
    assert_eq!(remaining_length(&[0x12, 0x83, 0x02]), 259);
    //assert_eq!(remaining_length(&[0x12, 0x85, 0x80, 0x01]), 2097157);
}


pub fn subscribe_msg_id(bytes: &[u8]) -> u16 {
    bytes[3] as u16
}

#[test]
fn subscribe_msg_id_happy() {
    assert_eq!(subscribe_msg_id(&[0x8cu8, 3, 0, 33]), 33);
    assert_eq!(subscribe_msg_id(&[0x8cu8, 3, 0, 21]), 21);
}
