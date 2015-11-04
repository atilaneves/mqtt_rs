use std::io::Write;
use std::io::Read;
use std::net::TcpListener;
use std::thread;
use std::sync::{Arc, Mutex};

mod server;
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

    for stream in listener.incoming() {
        let server = server.clone();
        let mut buffer = vec![0u8; 1024];

        thread::spawn(move || {
            let mut stream = stream.unwrap();
            loop {
                let read_result = stream.read(&mut buffer);
                let mut client = TcpClient::new(&mut stream);

                match read_result {
                    Ok(length) => {
                        let message_bytes = &buffer[0..length];
                        let mut server = server.lock().unwrap();
                        server.new_message(&mut client, message_bytes);
                    },
                    _ => panic!("Error reading bytes from stream"),
                }
            }
        });
    }
}
