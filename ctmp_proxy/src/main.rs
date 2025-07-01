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

fn calculate_checksum(packet_data: &[u8], packet_size: usize) -> u16 {
    let mut sum: u32 = 0;

    for i in (0..packet_size).step_by(2) {
        let mut word:u16 = (packet_data[i] as u16) << 8;

        if i + 1 < packet_size {
            word |= packet_data[i + 1] as u16;
        }

        // Ignore the checksum field.
        if i == 4 {
            word = 0xCCCC;
        }

        sum += word as u32;

        // Fold the carry bits.
        if sum > 0xFFFF {
            sum = (sum & 0xFFFF) + 1;
        }
    }

    return !sum as u16;
}

fn check_checksum(packet_data: &[u8], packet_size: usize) -> bool {
    let expected_checksum: usize = u16::from_be_bytes([packet_data[4], packet_data[5]]) as usize;
    let actual_checksum: usize = calculate_checksum(packet_data, packet_size) as usize;

    if expected_checksum == actual_checksum {
        return true
    }

    eprintln!("Wrong checksum! Expected: {}, Actual: {}", expected_checksum, actual_checksum);
    return false;
}

fn read_from_source(source: &mut TcpStream, buffer: &mut [u8]) -> usize {
    buffer.fill(0);
    let mut total_bytes = 0;

    loop {
        match source.read(&mut buffer[total_bytes..]) {
            // If the packet is incomplete but there's no more data from the source, return false.
            Ok(0) => return 0,
            Ok(bytes_read) => {
                println!("{} bytes read from source", bytes_read);
                total_bytes += bytes_read;

                // If the packet data is less than the header length, keep reading.
                if total_bytes < 8 {
                    continue;
                }

                // If the magic byte is wrong, stop reading and return false.
                if buffer[0] != 0xCC {
                    eprintln!("Invalid magic byte: {}", buffer[0]);
                    return 0;
                }

                let expected_length: usize = u16::from_be_bytes([buffer[2], buffer[3]]) as usize;

                // If the total bytes read doesn't match the expected length, keep reading.
                if total_bytes - 8 != expected_length {
                    println!("Received {} byte packet from source", total_bytes);
                    continue;
                }

                // If the packet is 'sensitive' and the checksum is wrong, return false.
                if buffer[1] & 0x40 > 0 && !check_checksum(&buffer, total_bytes) {
                    return 0;
                }
                
                return total_bytes;
            },
            Err(e) => {
                return 0;
            }
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
            Ok((stream, socket_address)) => {
                println!("New destination connection: {}", socket_address.port());
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