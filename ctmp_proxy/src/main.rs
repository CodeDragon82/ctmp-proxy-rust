use std::io::{Read, Write, Error, ErrorKind};
use std::net::{TcpListener, TcpStream};
use std::{process, usize};

const LOCALHOST: &str = "127.0.0.1";
const SOURCE_PORT: &str = "33333";
const DESTINATION_PORT: &str = "44444";

fn create_listener(port: &str) -> TcpListener {
    try_create_listener(port).unwrap_or_else(|e| {
        eprintln!("Failed to create socket on port {}: {}", port, e);
        process::exit(1);
    })
}

fn try_create_listener(port: &str) -> Result<TcpListener, Error> {
    let socket_address = format!("{}:{}", LOCALHOST, port);
    let listener = TcpListener::bind(&socket_address)?;

    println!("Opened listener: {}", socket_address);

    listener.set_nonblocking(true)?;
    println!("Socket non-blocking set.");

    Ok(listener)
}

/// Calculates the packet's checksum based on the 'Internet Checksum' standard
/// defined in RFC 1071. Checksum is calculated with `0xCCCC` replacing checksum 
/// field.
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

/// Calculates the checksum of the packet and compares it to the expected
/// checksum defined within the packet.
fn check_checksum(packet_data: &[u8], packet_size: usize) -> bool {
    let expected_checksum: usize = u16::from_be_bytes([packet_data[4], packet_data[5]]) as usize;
    let actual_checksum: usize = calculate_checksum(packet_data, packet_size) as usize;

    if expected_checksum == actual_checksum {
        return true
    }

    eprintln!("Wrong checksum! Expected: {}, Actual: {}", expected_checksum, actual_checksum);
    return false;
}


/// Reads from the source client until a full valid packet is in the `buffer`.
/// 
/// Returns the number of bytes read.
/// 
/// Returns error if it fails to read a valid packet:
///  - Packet is incomplete, but there's no most data to read.
///  - Packet magic byte is incorrect.
///  - Packet checksum field doesn't match the calculated checksum.
fn read_from_source(source: &mut TcpStream, buffer: &mut [u8]) -> Result<usize, Error> {
    buffer.fill(0);
    let mut total_bytes = 0;

    loop {
        let bytes_read: usize = source.read(&mut buffer[total_bytes..])?;

        // If the packet is incomplete but there's no more data from the source, return error.
        if bytes_read == 0 {
            return Err(Error::new(ErrorKind::Other, "No more data to read and packet is incomplete.".to_owned()));
        }

        println!("{} bytes read from source", bytes_read);
        total_bytes += bytes_read;

        // If the packet data is less than the header length, keep reading.
        if total_bytes < 8 {
            continue;
        }

        // If the magic byte is wrong, stop reading and return error.
        if buffer[0] != 0xCC {
            return Err(Error::new(ErrorKind::Other, format!("Invalid magic byte: {}", buffer[0]).to_owned()));
        }

        let expected_length: usize = u16::from_be_bytes([buffer[2], buffer[3]]) as usize;

        // If the total bytes read doesn't match the expected length, keep reading.
        if total_bytes - 8 != expected_length {
            println!("Received {} byte packet from source", total_bytes);
            continue;
        }

        // If the packet is 'sensitive' and the checksum is wrong, return error.
        if buffer[1] & 0x40 > 0 && !check_checksum(&buffer, total_bytes) {
            return Err(Error::new(ErrorKind::Other, "Checksum is wrong!".to_owned()));
        }
            
        return Ok(total_bytes);
    }
}

/// Send the packet from the `buffer` to every `destination_client`.
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
        if let Some(source) = source_client.as_mut() {
            match read_from_source(source, &mut buffer) {
                Ok(byte_count) => broadcast_to_destinations(&mut destination_clients, &buffer, byte_count),
                Err(e) => eprintln!("{}", e),
            }
        }
    }
}