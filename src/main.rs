use std::io;
use std::net::Ipv4Addr;
mod packet_sender;
mod parser;
mod sniffer;
mod tcp;

#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq)]
#[allow(dead_code)]
struct Quad {
    src: (Ipv4Addr, u16),
    dst: (Ipv4Addr, u16),
}

fn main() -> io::Result<()> {
    // let mut connections: HashMap<Quad, tcp::state> = Default::default();
    println!("Hello TCP");
    let new_interface = tun_tap::Iface::new("tun0", tun_tap::Mode::Tun)?;
    let mut buf = [0u8; 1504];
    loop {
        let nbytes = new_interface.recv(&mut buf[..])?;
        let _flags = u16::from_be_bytes([buf[0], buf[1]]);
        let proto = u16::from_be_bytes([buf[2], buf[3]]);

        if proto != 0x0800 {
            continue;
        }

        if let Some(packet) = parser::parser(&buf[4..nbytes]) {
            packet.ip_header.sniffer();
            if packet.ip_header.protocol == 6 {
                let state = tcp::State::check_state(packet.tcp_header.control_bit);
                println!(
                    "Received {}: SEQ={}, ACK={}",
                    state, packet.tcp_header.sequence_number, packet.tcp_header.acknowledge_number
                );

                let to_send_packet = tcp::State::tcp_connection(&state, &packet);

                // Only send if we have a valid packet (not all zeros)
                if to_send_packet[4] != 0 {
                    match new_interface.send(&to_send_packet) {
                        Ok(bytes) => println!("Sent {} bytes", bytes),
                        Err(e) => eprintln!("Error sending packet: {}", e),
                    }
                }
            }
        }
    }
}
