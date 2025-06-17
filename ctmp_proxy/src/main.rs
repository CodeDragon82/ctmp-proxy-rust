use std::net::TcpListener;
use std::process;

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

fn main() {
    let source_socket: TcpListener = create_listener(SOURCE_PORT);
    let destination_socket: TcpListener = create_listener(DESTINATION_PORT);

    loop {
        match source_socket.accept() {
            Ok((stream, socket_address)) => println!("New source connection: {}", socket_address.port()),
            Err(e) => {}
        }

        match destination_socket.accept() {
            Ok((stream, socket_addres)) => println!("New destination connection: {}", socket_addres.port()),
            Err(e) => {}
        }
    }
}