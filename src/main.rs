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
    let listener = TcpListener::bind(&address).expect(&format!("Could not bind to {}", address));
    let mut event_loop = mio::EventLoop::new().expect("Could not create MIO event loop");
    event_loop.register(&listener, MQTT_SERVER_TOKEN).expect("Could not register listener");
    event_loop.run(&mut MioHandler::new(listener)).expect("Could not run event loop");
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

                match self.listener.accept() {
                    Ok(Some(socket)) => {
                        let token = self.connections
                            .insert_with(|_| Rc::new(RefCell::new(Connection::new(socket))))
                            .expect("Could not insert new connection in slab");
                        self.mqtt_streams.
                            insert_with(|_| server::Stream::new())
                            .expect("Could not insert new stream into slab");
                        let connection = &self.connections[token].clone();
                        event_loop.register_opt(
                            &connection.borrow().socket,
                            token,
                            mio::EventSet::readable(),
                            mio::PollOpt::edge()).expect("Could not register connection with event loop");
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
                let still_connected = connection_ready(&mut self.server,
                                                       &mut self.mqtt_streams[token],
                                                       self.connections[token].clone());
                if !still_connected {
                    event_loop.deregister(&self.connections[token].borrow().socket)
                        .expect("Could not deregister connection with event loop");
                    self.server.unsubscribe_all(self.connections[token].clone());
                    self.connections[token].borrow_mut().socket.flush().expect("Could not flush socket");
                    self.connections.remove(token).expect("Could not remove connection from slab");
                }
            }
        }
    }
}

fn connection_ready(server: &mut server::Server<Connection>,
                    stream: &mut server::Stream,
                    connection: Rc<RefCell<Connection>>) -> bool {
    let connection = connection.clone();
    let read_result = connection.borrow_mut().read(stream.buffer());

    match read_result {
        Ok(length) => {
            if length >= stream.total_buffer_len() {
                panic!(format!("Too many bytes ({}) for puny stream buffer ({})",
                               length, stream.total_buffer_len()));
            }
            stream.handle_messages(length, server, connection.clone())
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
        self.socket.write_all(bytes).expect("Error writing to socket");
    }
}
