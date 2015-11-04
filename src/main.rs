use std::io::Write;
use std::io::Read;
use std::net::TcpListener;
use std::thread;

mod server;
mod message;

fn main() {
    let listener = TcpListener::bind(("0.0.0.0", 1883)).unwrap();
    for stream in listener.incoming() {
        thread::spawn(|| {
            {
                let mut stream = stream.unwrap();
                let mut buffer = vec![0u8; 1024];
                let read_result = stream.read(&mut buffer);

                match read_result {
                    Ok(length) => {
                        let message_bytes = &buffer[0..length];
                        let message_type = message::message_type(message_bytes);
                        match message_type {
                            message::MqttType::Connect => {
                                stream.write(&[32, 2, 0, 0]).unwrap();
                            }
                            message::MqttType::PingReq => {
                                stream.write(&[0xd0, 0]).unwrap();
                            }
                            _ => println!("Unknown message type"),
                        }
                    }
                    _ => panic!("Error reading bytes from stream"),
                }
            }
        });
    }
}
