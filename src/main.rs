use std::io;
use std::collections::HashMap;
use std::net::Ipv4Addr;
mod tcp;


#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq)]
struct Quad {
    src: (Ipv4Addr, u16),
    dst: (Ipv4Addr, u16),
}
fn main() -> io::Result<()> {
    let mut connections: HashMap<Quad, tcp::state> = Default::default();
    println!("Hello TCP");
    let new_interface = tun_tap::Iface::new("tun0", tun_tap::Mode::Tun)?;
    let mut buf = [0u8; 1504];
    loop {

        let nbytes = new_interface.recv(&mut buf[..])?;
        let flags = u16::from_be_bytes([buf[0], buf[1]]);
        let proto = u16::from_be_bytes([buf[2], buf[3]]);
        if proto != 0x0800 {
            // eprintln!("proto {:x} not ipv4", proto);
            //not ipv4
            continue;
        }
        match etherparse::Ipv4HeaderSlice::from_slice(&buf[4..nbytes]) {
            Ok(header) => {
                let src = header.source_addr();
                let dst = header.destination_addr();
                let ip_proto = header.protocol();
                let iph_len = header.slice().len();
                if ip_proto != etherparse::IpNumber(0x06) {
                    // eprintln!("proto {:x} not tcp", header.protocol());
                    //not tcp
                    continue;
                }
                match etherparse::TcpHeaderSlice::from_slice(&buf[4+header.slice().len()..nbytes]) {
                    Ok(tcp) => {
                        let src_port = tcp.source_port();
                        let dst_port = tcp.destination_port();
                        let data = 4 + iph_len + tcp.slice().len();
                        connections.entry(Quad {
                            src: (src, src_port),
                            dst: (dst, dst_port),
                        }).or_default().on_packet(header, tcp, &buf[data..nbytes]);
                        // println!("tcp source_port:{} destination_port:{} of bytes {:x?}", src_port, dst_port, tcp.slice().len());
                       
                    }
                    Err(e) => {
                        eprintln!("error parsing tcp header {:?}", e);
                    }
                }
            }
            Err(e) => {
                eprintln!("error parsing ipv4 header {:?}", e);
            }
        }
    // eprintln!("read {} flag{:x} proto{:x} bytes {:x?}", nbytes,flags,proto, &buf[4..nbytes]);
    }
    Ok(())

    
}
