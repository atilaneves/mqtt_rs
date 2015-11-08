use std::mem;

const HEADER_LEN: usize = 2;

#[derive(PartialEq, Debug)]
pub enum MqttType {
    //Reserved = 0,
    Connect = 1,
    // ConnAck = 2,
    Publish = 3,
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

#[test]
fn subscribe_type() {
    let subscribe_header = &[0x80, 0][0..];
    assert_eq!(message_type(subscribe_header), MqttType::Subscribe);
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

pub fn publish_topic(bytes: &[u8]) -> String {
    let topic_len = ((bytes[2] as u16) << 8) + bytes[3] as u16;
    let topic_len = topic_len as usize;
    String::from_utf8(bytes[4 .. 4 + topic_len].to_vec()).unwrap()
}

#[test]
fn test_get_topic_with_msg_id() {
    let pub_bytes = vec![
        0x3c, 0x0d, //fixed header
        0x00, 0x05, 'f' as u8, 'i' as u8, 'r' as u8, 's' as u8, 't' as u8,//topic name
        0x00, 0x21, //message ID
        'b' as u8, 'o' as u8, 'r' as u8, 'g' as u8, //payload
        ];
    assert_eq!(publish_topic(&pub_bytes[..]), "first");
}


pub fn publish_payload(bytes: &[u8]) -> &[u8] {
    let topic_len = publish_topic(bytes).len();
    let mut start = HEADER_LEN + topic_len + 2;
    if (bytes[0] & 0x06) != 0 {
        start += 2;
    }

    &bytes[start ..]
}

#[test]
fn test_get_payload_with_msg_id() {
    let pub_bytes = vec![
        0x3c, 0x0d, //fixed header
        0x00, 0x05, 'f' as u8, 'i' as u8, 'r' as u8, 's' as u8, 't' as u8,//topic name
        0x00, 0x21, //message ID
        1, 2, 3, 4, //payload
        ];
    assert_eq!(publish_payload(&pub_bytes[..]).to_vec(), vec![1, 2, 3, 4]);
}


#[test]
fn test_get_payload_no_msg_id() {
    let pub_bytes = vec![
        0x30, 0x0a, //fixed header
        0x00, 0x05, 'f' as u8, 'i' as u8, 'r' as u8, 's' as u8, 't' as u8,//topic name
        9, 8, 7, //payload
        ];
    assert_eq!(publish_payload(&pub_bytes[..]).to_vec(), vec![9, 8, 7]);
}


pub fn subscribe_topics(bytes: &[u8]) -> Vec<String> {
    let start = HEADER_LEN + 2; // final 2 for msg_id
    let mut res = vec![];
    let mut slice = &bytes[start .. ];
    while slice.len() > 0 {
        let topic_len = (((slice[0] as u16) << 8) + slice[1] as u16) as usize;
        let topic_slice = &slice[2 .. 2 + topic_len];
        res.push(String::from_utf8(topic_slice.to_vec()).unwrap());
        //first 2 for topic len, last 1 for qos
        slice = &slice[2 + topic_len + 1 .. ];
    }
    res
}

#[test]
fn test_subscribe_topics1() {
    let sub_bytes = vec![
        0x8b, 0x13, //fixed header
        0x00, 0x21, //message ID
        0x00, 0x05, 'f' as u8, 'i' as u8, 'r' as u8, 's' as u8, 't' as u8,
        0x01, //qos
        0x00, 0x06, 's' as u8, 'e' as u8, 'c' as u8, 'o' as u8, 'n' as u8, 'd' as u8,
        0x02, //qos
        ];
    assert_eq!(subscribe_topics(&sub_bytes[..]), vec!["first".to_string(), "second".to_string()]);
}

#[test]
fn test_subscribe_topics2() {
    let sub_bytes = vec![
        0x8b, 0x12, //fixed header
        0x00, 0x21, //message ID
        0x00, 0x04, 'f' as u8, 'i' as u8, 'r' as u8, 's' as u8,
        0x01, //qos
        0x00, 0x06, 's' as u8, 'e' as u8, 'c' as u8, 'o' as u8, 'n' as u8, 'd' as u8,
        0x02, //qos
        ];
    assert_eq!(subscribe_topics(&sub_bytes[..]), vec!["firs".to_string(), "second".to_string()]);
}
