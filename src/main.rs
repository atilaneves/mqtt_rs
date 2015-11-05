use std::io::Write;
use std::io::Read;
use std::net::TcpListener;
use std::thread;
use std::sync::{Arc, Mutex};

mod server;
mod broker;
mod message;

struct TcpClient<'a> {
    stream: &'a mut std::net::TcpStream,
}

impl<'a> TcpClient<'a> {
    fn new(stream: &'a mut std::net::TcpStream) -> Self {
        TcpClient { stream : stream }
    }
}

impl<'a> server::Client for TcpClient<'a> {
    fn send(&mut self, bytes: &[u8]) {
        let _ = self.stream.write(bytes);
    }
}

fn main() {
    let server = Arc::new(Mutex::new(server::Server::new()));
    let listener = TcpListener::bind(("0.0.0.0", 1883)).unwrap();

    for byte_stream in listener.incoming() {
        let server = server.clone();
        let mut mqtt_stream = server::Stream::new();

        thread::spawn(move || {
            let mut byte_stream = byte_stream.unwrap();
            loop {
                let read_result = byte_stream.read(mqtt_stream.buffer());
                //the client must be created here due to scoping issues with byte_stream
                let mut client = TcpClient::new(&mut byte_stream);

                match read_result {
                    Ok(length) => {
                        let mut server = server.lock().unwrap();
                        mqtt_stream.handle_messages(length, &mut server, &mut client);
                    },
                    _ => panic!("Error reading bytes from stream"),
                }
            }
        });
    }
}
