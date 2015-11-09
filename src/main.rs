extern crate mio;

use std::io::Write;
use std::io::Read;
use std::rc::{Rc};
use std::cell::{RefCell};
use mio::tcp::*;

mod server;
mod broker;
mod message;


const MQTT_SERVER_TOKEN: mio::Token = mio::Token(0);

fn main() {
    let address = "0.0.0.0:1883".parse().unwrap();
    let listener = TcpListener::bind(&address).unwrap();
    let mut event_loop = mio::EventLoop::new().unwrap();
    event_loop.register(&listener, MQTT_SERVER_TOKEN).unwrap();
    event_loop.run(&mut MioHandler::new(listener)).unwrap();
}


struct MioHandler {
    listener: TcpListener,
    connections: mio::util::Slab<Rc<RefCell<Connection>>>,
    mqtt_streams: mio::util::Slab<server::Stream>,
    server: server::Server<Connection>,
}

struct Connection {
    socket: mio::tcp::TcpStream,
}

impl MioHandler {
    fn new(listener: TcpListener) -> Self {
        let max_conns = 1024 * 32;
        let connections_slab = mio::util::Slab::new_starting_at(mio::Token(1), max_conns);
        let mqtt_stream_slab = mio::util::Slab::new_starting_at(mio::Token(1), max_conns);

        MioHandler {
            listener: listener,
            connections: connections_slab,
            mqtt_streams: mqtt_stream_slab,
            server: server::Server::new(),
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
            MQTT_SERVER_TOKEN => {
                assert!(events.is_readable());

                println!("Server socket is ready to accept a connetion");
                match self.listener.accept() {
                    Ok(Some(socket)) => {
                        println!("New mio client connection");
                        let token = self.connections
                            .insert_with(|_| Rc::new(RefCell::new(Connection::new(socket))))
                            .unwrap();
                        self.mqtt_streams.insert_with(|_| server::Stream::new()).unwrap();
                        let connection = &self.connections[token].clone();
                        event_loop.register_opt(
                            &connection.borrow().socket,
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
                connection_ready(&mut self.server,
                                 &mut self.mqtt_streams[token],
                                 self.connections[token].clone());
            }
        }
    }
}

fn connection_ready(server: &mut server::Server<Connection>,
                    stream: &mut server::Stream,
                    connection: Rc<RefCell<Connection>>) {
    let connection = connection.clone();
    let read_result = connection.borrow_mut().read(stream.buffer());

    println!("Oh read result! {:?}", read_result);

    match read_result {
        Ok(length) => {
            stream.handle_messages(length, server, connection.clone());
        },
        _ => panic!("Error reading bytes from stream"),
    }
}


impl Connection {
    fn new(socket: mio::tcp::TcpStream) -> Self {
        Connection { socket: socket }
    }

    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        self.socket.read(buffer)
    }
}

impl broker::Subscriber for Connection {
    fn new_message(&mut self, bytes: &[u8]) {
        self.socket.write(bytes).unwrap();
    }
}
