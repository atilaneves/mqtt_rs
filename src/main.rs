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
    server: server::Server<Connection>,
}

struct Connection {
    socket: mio::tcp::TcpStream,
    stream: server::Stream,
}

impl MioHandler {
    fn new(listener: TcpListener) -> Self {
        let slab = mio::util::Slab::new_starting_at(mio::Token(1), 1024 * 32);

        MioHandler {
            listener: listener,
            connections: slab,
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
                //self.connections[token].clone().borrow_mut().ready(&mut self.server);
                connection_ready(&mut self.server, self.connections[token].clone());
            }
        }
    }
}

fn connection_ready(server: &mut server::Server<Connection>, connection: Rc<RefCell<Connection>>) {
    let connection = connection.clone();
    let read_result = connection.borrow_mut().read();

    match read_result {
        Ok(length) => {
            connection.borrow_mut().stream.handle_messages(length, server, connection.clone());
        },
        _ => panic!("Error reading bytes from stream"),
    }
}


impl Connection {
    fn new(socket: mio::tcp::TcpStream) -> Self {
        Connection { socket: socket, stream: server::Stream::new() }
    }

    fn read(&mut self) -> std::io::Result<usize> {
        self.socket.read(self.stream.buffer())
    }

    // fn ready(&mut self, server: &mut server::Server<Connection>) {
    //     let read_result = self.socket.read(self.stream.buffer());

    //     match read_result {
    //         Ok(length) => {
    //             self.stream.handle_messages(length, server, self);
    //         },
    //         _ => panic!("Error reading bytes from stream"),
    //     }
    // }
}

impl broker::Subscriber for Connection {
    fn new_message(&mut self, bytes: &[u8]) {
        self.socket.write(bytes);
    }
}
