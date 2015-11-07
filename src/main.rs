extern crate mio;

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
    thread::spawn(miomain);

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

const MQTT_SERVER: mio::Token = mio::Token(0);

fn miomain() {
    let address = "0.0.0.0:1884".parse().unwrap();
    let listener = mio::tcp::TcpListener::bind(&address).unwrap();
    let mut event_loop = mio::EventLoop::new().unwrap();
    event_loop.register(&listener, MQTT_SERVER);
    println!("Running mio server");
    event_loop.run(&mut MioHandler::new(listener));
}


struct MioHandler {
    listener: mio::tcp::TcpListener,
    connections: mio::util::Slab<Connection>,
}

impl MioHandler {
    fn new(listener: mio::tcp::TcpListener) -> Self {
        let slab = mio::util::Slab::new_starting_at(mio::Token(1), 1024 * 32);

        MioHandler {
            listener: listener,
            connections: slab,
        }
    }
}

struct Connection {
    socket: mio::tcp::TcpStream,
    token: mio::Token,
}

impl Connection {
    fn new(socket: mio::tcp::TcpStream, token: mio::Token) -> Self {
        Connection { socket: socket, token: token }
    }

    fn ready(&mut self, event_loop: &mut mio::EventLoop<MioHandler>, events: mio::EventSet) {
        println!("Connection ready!");
        assert!(events.is_readable());
        let mut buffer : Vec<u8> = vec![0; 1024];
        let res = self.socket.read(&mut buffer[..]);
        match res {
            Ok(length) => {
                println!("Read these bytes: {:?}", &buffer[0..length]);
            }
            _ => {
                println!("Could not read from client socket");
            }
        }
    }
}

impl mio::Handler for MioHandler {
    type Timeout = ();
    type Message = ();

    fn ready(&mut self,
             event_loop: &mut mio::EventLoop<MioHandler>,
             token: mio::Token,
             events: mio::EventSet) {
        match token {
            MQTT_SERVER => {
                assert!(events.is_readable());

                println!("Server socket is ready to accept a connetion");
                match self.listener.accept() {
                    Ok(Some(socket)) => {
                        println!("New mio client connection");
                        let token = self.connections
                            .insert_with(|token| Connection::new(socket, token))
                            .unwrap();
                        event_loop.register_opt(
                            &self.connections[token].socket,
                            token,
                            mio::EventSet::readable(),
                            mio::PollOpt::edge()).unwrap();
                    }
                    Ok(None) => {
                        println!("The server socket wasn't actually ready");
                    }
                    Err(e) => {
                        println!("listener.accept errored: {}", e);
                        event_loop.shutdown();
                    }
                }
            }
            _ => {
                self.connections[token].ready(event_loop, events);
            }
        }
    }
}
