extern crate mio;

use std::io::Write;
use std::io::Read;
use mio::tcp::*;

mod server;
mod broker;
mod message;


struct TcpClient<'a> {
    stream: &'a mut mio::tcp::TcpStream,
}

impl<'a> TcpClient<'a> {
    fn new(stream: &'a mut mio::tcp::TcpStream) -> Self {
        TcpClient { stream : stream }
    }
}

impl<'a> server::Client for TcpClient<'a> {
    fn send(&mut self, bytes: &[u8]) {
        let _ = self.stream.write(bytes);
    }
}

impl<'a> broker::Subscriber for TcpClient<'a> {
    fn new_message(&mut self, bytes: &[u8]) {
        self.stream.write(bytes);
    }
}


const MQTT_SERVER_TOKEN: mio::Token = mio::Token(0);

fn main() {
    let address = "0.0.0.0:1883".parse().unwrap();
    let listener = TcpListener::bind(&address).unwrap();
    let mut event_loop = mio::EventLoop::new().unwrap();
    event_loop.register(&listener, MQTT_SERVER_TOKEN).unwrap();
    event_loop.run(&mut MioHandler::new(listener)).unwrap();
}


struct MioHandler<'a> {
    listener: TcpListener,
    connections: mio::util::Slab<Connection>,
    server: server::Server<TcpClient<'a>>,
}

struct Connection {
    socket: mio::tcp::TcpStream,
    stream: server::Stream,
}

impl<'a> MioHandler<'a> {
    fn new(listener: TcpListener) -> Self {
        let slab = mio::util::Slab::new_starting_at(mio::Token(1), 1024 * 32);

        MioHandler {
            listener: listener,
            connections: slab,
            server: server::Server::new(),
        }
    }
}

impl<'a> mio::Handler for MioHandler<'a> {
    type Timeout = ();
    type Message = ();

    fn ready(&mut self,
             event_loop: &mut mio::EventLoop<MioHandler>,
             token: mio::Token,
             events: mio::EventSet) {
        match token {
            MQTT_SERVER_TOKEN => {
                assert!(events.is_readable());

                println!("Server socket is ready to accept a connetion");
                match self.listener.accept() {
                    Ok(Some(socket)) => {
                        println!("New mio client connection");
                        let token = self.connections
                            .insert_with(|_| Connection::new(socket))
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
                assert!(events.is_readable());
                self.connections[token].ready(&mut self.server);
            }
        }
    }
}

impl Connection {
    fn new(socket: mio::tcp::TcpStream) -> Self {
        Connection { socket: socket, stream: server::Stream::new() }
    }

    fn ready(&mut self, server: &mut server::Server<TcpClient>) {
        let read_result = self.socket.read(self.stream.buffer());
        let mut client = TcpClient::new(&mut self.socket);

        match read_result {
            Ok(length) => {
                self.stream.handle_messages(length, server, &mut client);
            },
            _ => panic!("Error reading bytes from stream"),
        }
    }
}
