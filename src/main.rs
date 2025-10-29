use std::collections::HashMap;
use std::io;
use std::os::unix::io::AsRawFd;
use std::time::{Duration, Instant};
mod packet_sender;
mod parser;
mod sniffer;
mod tcp;
mod tcb;

use tcb::{Quad, Tcb, RetransmitAction};

fn main() -> io::Result<()> {
    println!("Hello TCP");
    
    let mut connections: HashMap<Quad, Tcb> = HashMap::new();
    
    let new_interface = tun_tap::Iface::new("tun0", tun_tap::Mode::Tun)?;
    let mut buf = [0u8; 1504];
    
    // Set the interface to non-blocking mode
    let fd = new_interface.as_raw_fd();
    unsafe {
        let flags = libc::fcntl(fd, libc::F_GETFL, 0);
        libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
    }
    
    let mut last_check = Instant::now();
    
    loop {
        // Calculate timeout for next retransmission check
        let timeout = connections
            .values()
            .filter_map(|tcb| tcb.time_until_retransmit())
            .min()
            .unwrap_or(Duration::from_millis(100)); // Default 100ms if no timers
        
        // Use select to wait for data with timeout
        let mut read_fds = unsafe {
            let mut fds: libc::fd_set = std::mem::zeroed();
            libc::FD_ZERO(&mut fds);
            libc::FD_SET(fd, &mut fds);
            fds
        };
        
        let mut timeout_val = libc::timeval {
            tv_sec: timeout.as_secs() as libc::time_t,
            tv_usec: timeout.subsec_micros() as libc::suseconds_t,
        };
        
        let result = unsafe {
            libc::select(
                fd + 1,
                &mut read_fds,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut timeout_val,
            )
        };
        
        if result > 0 {
            // Data available to read
            match new_interface.recv(&mut buf[..]) {
                Ok(nbytes) => {
                    let _flags = u16::from_be_bytes([buf[0], buf[1]]);
                    let proto = u16::from_be_bytes([buf[2], buf[3]]);

                    if proto != 0x0800 {
                        continue;
                    }

                    if let Some(packet) = parser::parser(&buf[4..nbytes]) {
                        packet.ip_header.sniffer();
                        
                        if packet.ip_header.protocol == 6 {
                            let quad = Quad {
                                src: (packet.ip_header.source, packet.tcp_header.source_port),
                                dst: (packet.ip_header.destination, packet.tcp_header.destination_port),
                            };
                            
                            let state = tcp::State::check_state(packet.tcp_header.control_bit);
                            println!(
                                "Received {}: SEQ={}, ACK={} from {}:{}",
                                state,
                                packet.tcp_header.sequence_number,
                                packet.tcp_header.acknowledge_number,
                                packet.ip_header.source,
                                packet.tcp_header.source_port
                            );

                            let to_send_packet = tcp::State::tcp_connection(&state, &packet, &mut connections, quad);

                            if to_send_packet[4] != 0 {
                                match new_interface.send(&to_send_packet) {
                                    Ok(bytes) => println!("Sent {} bytes", bytes),
                                    Err(e) => eprintln!("Error sending packet: {}", e),
                                }
                            }
                        }
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                    // This shouldn't happen after select, but handle it anyway
                }
                Err(e) => {
                    eprintln!("Error receiving packet: {}", e);
                }
            }
        } else if result == 0 {
            // Timeout occurred - check for retransmissions
            let now = Instant::now();
            if now.duration_since(last_check) >= Duration::from_millis(10) {
                last_check = now;
                
                let actions = tcp::State::check_retransmissions(&mut connections);
                
                for (quad, action) in actions {
                    match action {
                        RetransmitAction::Retransmit { seq, flags, data, attempt } => {
                            println!("⟳ Retransmitting SEQ={} (attempt #{})", seq, attempt);
                            
                            if let Some(tcb) = connections.get(&quad) {
                                let packet = tcp::State::create_retransmit_packet(
                                    &quad, seq, flags, data, tcb
                                );
                                
                                if packet[4] != 0 {
                                    match new_interface.send(&packet) {
                                        Ok(bytes) => println!("Retransmitted {} bytes", bytes),
                                        Err(e) => eprintln!("Error retransmitting: {}", e),
                                    }
                                }
                            }
                        }
                        RetransmitAction::GiveUp { seq, reason } => {
                            println!("✗ Giving up on SEQ={}: {}", seq, reason);
                            // Could remove connection here
                        }
                    }
                }
            }
        } else {
            // Error occurred
            eprintln!("Select error: {}", io::Error::last_os_error());
        }
    }
}
