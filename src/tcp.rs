use std::collections::HashMap;
use crate::parser::IPHeader;
use crate::parser::Packet;
use crate::parser::TCPHeader;
use crate::tcb::{Quad, Tcb, TcpState, RetransmitAction};

#[allow(dead_code)]
pub enum State {
    Closed,
    Listen,
    SynRcvd,
    SynSent,
    Estab,
}

#[allow(dead_code)]
pub struct Sequence {
    snd_una: u32,
    snd_nxt: u32,
    seg_ack: u32,
    seg_seq: u32,
    seg_len: u32,
}

impl State {
    pub fn check_state(flags: u8) -> String {
        let syn = (flags & 0x02) != 0; // SYN flag
        let ack = (flags & 0x10) != 0; // ACK flag
        let fin = (flags & 0x01) != 0; // FIN flag
        let rst = (flags & 0x04) != 0; // RST flag

        match (syn, ack, fin, rst) {
            (true, false, false, false) => "SYN",
            (true, true, false, false) => "SYN-ACK",
            (false, true, false, false) => "ACK",
            (false, false, true, false) => "FIN",
            (false, true, true, false) => "FIN-ACK",
            (false, false, false, true) => "RST",
            _ => "UNKNOWN",
        }
        .to_string()
    }
    pub fn tcp_connection(
        state: &String,
        packet: &Packet,
        connections: &mut HashMap<Quad, Tcb>,
        quad: Quad,
    ) -> [u8; 1504] {
        let raw_packet = if state == "SYN" {
            let tcb = connections.entry(quad).or_insert_with(|| {
                let mut tcb = Tcb::new(quad);
                tcb.passive_open();
                tcb
            });

            // Generate ISN (in production, use secure random)
            let isn: u32 = 1000;
            
            // Process the SYN
            tcb.process_syn(
                packet.tcp_header.sequence_number,
                packet.tcp_header.window,
                isn,
            );

            let ack_num = packet.tcp_header.sequence_number.wrapping_add(1);
            println!("Sending SYN-ACK: SEQ={}, ACK={} (State: {:?})", isn, ack_num, tcb.state);

            let response_packet = Packet {
                ip_header: IPHeader {
                    version: 4,
                    ihl: 5,
                    type_of_service: 0,
                    total_len: 40,
                    identification: packet.ip_header.identification,
                    flags: 0x02,
                    fragment_offset: 0,
                    ttl: 64,
                    protocol: 6,
                    header_checksum: 0,
                    source: packet.ip_header.destination,
                    destination: packet.ip_header.source,
                },
                tcp_header: TCPHeader {
                    source_port: packet.tcp_header.destination_port,
                    destination_port: packet.tcp_header.source_port,
                    sequence_number: isn,
                    acknowledge_number: ack_num,
                    data_offset: 5,
                    reserved: 0,
                    control_bit: 0x12,
                    window: tcb.rcv.wnd,
                    checksum: 0,
                    urgent_pointer: 0,
                },
                data: [0u8; 500],
            };

            // Update send next and queue for retransmission
            tcb.snd.nxt = isn.wrapping_add(1);
            tcb.queue_for_retransmission(isn, 0x12, vec![]); // SYN-ACK needs retransmission

            response_packet.create_packet()
        } else if state == "ACK" {
            if let Some(tcb) = connections.get_mut(&quad) {
                // Process the ACK
                if tcb.process_ack(
                    packet.tcp_header.acknowledge_number,
                    packet.tcp_header.window,
                ) {
                    println!("Connection established! State: {:?}", tcb.state);
                    
                    if tcb.state == TcpState::Established {
                        println!("TCP handshake complete for {}:{} -> {}:{}",
                            quad.src.0, quad.src.1, quad.dst.0, quad.dst.1);
                    }
                }
            }
            [0u8; 1504]
        } else {
            [0u8; 1504]
        };
        
        raw_packet
    }
    
    /// Check for retransmissions across all connections
    pub fn check_retransmissions(connections: &mut HashMap<Quad, Tcb>) -> Vec<(Quad, RetransmitAction)> {
        let mut actions = Vec::new();
        
        for (quad, tcb) in connections.iter_mut() {
            let tcb_actions = tcb.check_retransmission_timeout();
            for action in tcb_actions {
                actions.push((*quad, action));
            }
        }
        
        actions
    }
    
    /// Create retransmission packet
    pub fn create_retransmit_packet(
        quad: &Quad,
        seq: u32,
        flags: u8,
        data: Vec<u8>,
        tcb: &Tcb,
    ) -> [u8; 1504] {
        let ack_num = tcb.rcv.nxt;
        
        let packet = Packet {
            ip_header: IPHeader {
                version: 4,
                ihl: 5,
                type_of_service: 0,
                total_len: 40 + data.len() as u16,
                identification: 0,
                flags: 0x02,
                fragment_offset: 0,
                ttl: 64,
                protocol: 6,
                header_checksum: 0,
                source: quad.dst.0,
                destination: quad.src.0,
            },
            tcp_header: TCPHeader {
                source_port: quad.dst.1,
                destination_port: quad.src.1,
                sequence_number: seq,
                acknowledge_number: ack_num,
                data_offset: 5,
                reserved: 0,
                control_bit: flags,
                window: tcb.rcv.wnd,
                checksum: 0,
                urgent_pointer: 0,
            },
            data: {
                let mut arr = [0u8; 500];
                let len = data.len().min(500);
                arr[..len].copy_from_slice(&data[..len]);
                arr
            },
        };
        
        packet.create_packet()
    }
}
