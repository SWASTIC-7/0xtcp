use std::io;
use std::collections::HashMap;
use std::net::Ipv4Addr;
mod tcp;
mod parser;
mod sniffer;
mod packet_sender;

#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq)]
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
            // eprintln!("proto {:x} not ipv4", proto);
            //not ipv4
            continue;
        }
        if let Some(packet) = parser::parser(&buf[4..nbytes]) {
            packet.ip_header.sniffer();
        }

    
    }
    Ok(())

    
}
