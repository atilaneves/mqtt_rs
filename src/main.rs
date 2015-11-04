use std::io::Write;
use std::net::TcpListener;
use std::thread;

fn main() {
    let listener = TcpListener::bind(("0.0.0.0", 1883)).unwrap();
    for stream in listener.incoming() {
        thread::spawn(|| {
            let mut stream = stream.unwrap();
            stream.write(b"Hello world\r\n").unwrap();
        });
    }
}
