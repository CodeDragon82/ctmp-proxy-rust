use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::{process, usize};

const LOCALHOST: &str = "127.0.0.1";
const SOURCE_PORT: &str = "33333";
const DESTINATION_PORT: &str = "44444";

fn create_listener(port: &str) -> TcpListener {
    let socket_address = format!("{}:{}", LOCALHOST, port);
    
    match TcpListener::bind(&socket_address) {
        Ok(listener) => {
            println!("Opened listener: {}", &socket_address);

            // Turn on non-blocking mode; accept() should NOT block the thread.
            match listener.set_nonblocking(true) {
                Ok(_) => {
                    println!("Socket non-blocking set.");
                    return listener;
                }
                Err(e) => {
                    eprintln!("Failed to set non-blocking mode: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to bind to {}: {}", &socket_address, e);
            process::exit(1);
        }
    };
}

fn read_from_source(source: &mut TcpStream, buffer: &mut [u8]) -> usize {
    match source.read(buffer) {
        Ok(0) => return 0,
        Ok(byte_count) => {
            println!("{} bytes read from source", byte_count);
            return byte_count;
        },
        Err(e) => {
            eprintln!("Failed to read from source: {}", e);
            return 0;
        }
    }
}

fn broadcast_to_destinations(destination_clients: &mut Vec<TcpStream>, buffer: &[u8], buffer_size: usize) {
    for destination_client in &mut destination_clients.iter_mut() {
        match destination_client.write_all(&buffer[..buffer_size]) {
            Ok(_) => println!("Sending {} bytes to {}", buffer_size, destination_client.local_addr().unwrap().port()),
            Err(_) => eprintln!("Failed to send data to {}", destination_client.local_addr().unwrap().port()),
        }
    }
}

fn main() {
    let source_socket: TcpListener = create_listener(SOURCE_PORT);
    let destination_socket: TcpListener = create_listener(DESTINATION_PORT);

    let mut source_client: Option<TcpStream> = None;
    let mut destination_clients: Vec<TcpStream> =  Vec::new();

    let mut buffer: [u8; 70000] = [0; 70000];

    loop {
        match source_socket.accept() {
            Ok((stream, socket_address)) => {
                println!("New source connection: {}", socket_address.port());

                // Don't block the thread when reading data (i.e., don't wait).
                stream.set_nonblocking(true);

                source_client = Some(stream);
            },
            Err(e) => {}
        }

        match destination_socket.accept() {
            Ok((stream, socket_addres)) => {
                println!("New destination connection: {}", socket_addres.port());
                destination_clients.push(stream);
            },
            Err(e) => {}
        }

        // Attempt to read data from source client if connected.
        match source_client.as_mut() {
            Some(source) => {
                let byte_count= read_from_source(source, &mut buffer);
                if byte_count > 0 {
                    broadcast_to_destinations(&mut destination_clients, &buffer, byte_count);
                }
            },
            None => {}
        }
    }
}