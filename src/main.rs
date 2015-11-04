use std::io::Write;
use std::net::TcpListener;
use std::thread;

mod server;
mod message;

fn main() {
    let listener = TcpListener::bind(("0.0.0.0", 1883)).unwrap();
    for stream in listener.incoming() {
        thread::spawn(|| {
            let mut stream = stream.unwrap();
            stream.write(&[32, 2, 0, 0]).unwrap();
        });
    }
}
