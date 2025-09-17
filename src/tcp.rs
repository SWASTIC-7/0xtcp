

//                               +---------+ ---------\      active OPEN  
//                               |  CLOSED |            \    -----------  
//                               +---------+<---------\   \   create TCB  
//                                 |     ^              \   \  snd SYN    
//                    passive OPEN |     |   CLOSE        \   \           
//                    ------------ |     | ----------       \   \         
//                     create TCB  |     | delete TCB         \   \       
//                                 V     |                      \   \     
//                               +---------+            CLOSE    |    \   
//                               |  LISTEN |          ---------- |     |  
//                               +---------+          delete TCB |     |  
//                    rcv SYN      |     |     SEND              |     |  
//                   -----------   |     |    -------            |     V  
//  +---------+      snd SYN,ACK  /       \   snd SYN          +---------+
//  |         |<-----------------           ------------------>|         |
//  |   SYN   |                    rcv SYN                     |   SYN   |
//  |   RCVD  |<-----------------------------------------------|   SENT  |
//  |         |                    snd ACK                     |         |
//  |         |------------------           -------------------|         |
//  +---------+   rcv ACK of SYN  \       /  rcv SYN,ACK       +---------+
//    |           --------------   |     |   -----------                  
//    |                  x         |     |     snd ACK                    
//    |                            V     V                                
//    |  CLOSE                   +---------+                              
//    | -------                  |  ESTAB  |                              
//    | snd FIN                  +---------+                              
//    |                   CLOSE    |     |    rcv FIN                     
//    V                  -------   |     |    -------                     
//  +---------+          snd FIN  /       \   snd ACK          +---------+
//  |  FIN    |<-----------------           ------------------>|  CLOSE  |
//  | WAIT-1  |------------------                              |   WAIT  |
//  +---------+          rcv FIN  \                            +---------+
//    | rcv ACK of FIN   -------   |                            CLOSE  |  
//    | --------------   snd ACK   |                           ------- |  
//    V        x                   V                           snd FIN V  
//  +---------+                  +---------+                   +---------+
//  |FINWAIT-2|                  | CLOSING |                   | LAST-ACK|
//  +---------+                  +---------+                   +---------+
//    |                rcv ACK of FIN |                 rcv ACK of FIN |  
//    |  rcv FIN       -------------- |    Timeout=2MSL -------------- |  
//    |  -------              x       V    ------------        x       V  
//     \ snd ACK                 +---------+delete TCB         +---------+
//      ------------------------>|TIME WAIT|------------------>| CLOSED  |
//                               +---------+                   +---------+

//                       TCP Connection State Diagram

use std::os::unix::process::parent_id;

use etherparse::err::packet;

use crate::parser::IPHeader;
use crate::parser::TCPHeader;
use crate::parser::Packet;


#[allow(dead_code)]
pub enum State {
    Closed,
    Listen,
    SynRcvd,
    SynSent,
    Estab
}

pub struct Sequence {
    snd_una: u32,
    snd_nxt: u32,
    seg_ack: u32,
    seg_seq: u32,
    seg_len: u32,


}

impl State {
    pub fn check_state(flags : u8)-> String {
        let syn = (flags & 0x02) != 0;  // SYN flag
        let ack = (flags & 0x10) != 0;  // ACK flag
        let fin = (flags & 0x01) != 0;  // FIN flag
        let rst = (flags & 0x04) != 0;  // RST flag

        match (syn, ack, fin, rst) {
            (true, false, false, false) => "SYN",
            (true, true, false, false) => "SYN-ACK",
            (false, true, false, false) => "ACK",
            (false, false, true, false) => "FIN",
            (false, true, true, false) => "FIN-ACK",
            (false, false, false, true) => "RST",
            _ => "UNKNOWN"
        }.to_string()
        
    }
    pub fn tcp_connection(state: &String, packet: &Packet)-> [u8; 1504]{
        
        let raw_packet = if state == "SYN" {

            // sending a syn_ack
            let packet = Packet {
                ip_header: IPHeader {
                    version: 4,
                    ihl: 5,
                    type_of_service: 0,
                    total_len: packet.ip_header.total_len, 
                    identification: packet.ip_header.identification,
                    flags: packet.ip_header.flags,
                    fragment_offset: packet.ip_header.fragment_offset,
                    ttl: packet.ip_header.ttl,
                    protocol: 6, 
                    header_checksum: 
                    source: packet.ip_header.destination ,
                    destination: packet.ip_header.source,
                },
                tcp_header: TCPHeader {
                    source_port: packet.tcp_header.destination_port,
                    destination_port: packet.tcp_header.source_port,
                    sequence_number: 0,
                    acknowledge_number: packet.tcp_header.sequence_number+1,
                    data_offset: 5,
                    reserved: 0,
                    control_bit: 0x12,  //syn_ack
                    window: 64240,
                    checksum: 0, 
                    urgent_pointer: 0,
                },
                data: [0u8; 500],
            };
            
            
            
            packet.create_packet()
            
        } else {
            [0u8; 1504]
        };
        raw_packet
    }
}

