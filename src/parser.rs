// 0                   1                   2                   3
//     0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
//    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//    |Version|  IHL  |Type of Service|          Total Length         |
//    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//    |         Identification        |Flags|      Fragment Offset    |
//    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//    |  Time to Live |    Protocol   |         Header Checksum       |
//    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//    |                       Source Address                          |
//    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//    |                    Destination Address                        |
//    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//    |                    Options                    |    Padding    |
//    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+

//                     Example Internet Datagram Header(ipv4)

//                     from rfc 971 -- Internet Protocol


//     0                   1                   2                   3   
//     0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 
//    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//    |          Source Port          |       Destination Port        |
//    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//    |                        Sequence Number                        |
//    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//    |                    Acknowledgment Number                      |
//    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//    |  Data |           |U|A|P|R|S|F|                               |
//    | Offset| Reserved  |R|C|S|S|Y|I|            Window             |
//    |       |           |G|K|H|T|N|N|                               |
//    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//    |           Checksum            |         Urgent Pointer        |
//    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//    |                    Options                    |    Padding    |
//    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//    |                             data                              |
//    +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+

//                             TCP Header Format

//                    from rfc 793 -- tcp

use std::net::Ipv4Addr;
#[allow(dead_code)]
pub struct IPHeader {
    pub version:  u8,   //4 bits
    pub ihl: u8,        //4 bits
    pub type_of_service: u8,
    pub total_len: u16,
    pub identification: u16,
    pub flags: u8,      //3 bits
    pub fragment_offset: u16,    //13 bits
    pub ttl: u8,
    pub protocol: u8,
    pub header_checksum: u16,
    pub source: Ipv4Addr,
    pub destination: Ipv4Addr,

}
#[allow(dead_code)]
pub  struct TCPHeader {
    pub source_port: u16,
    pub destination_port: u16,
    pub sequence_number: u32,
    pub acknowledge_number: u32,
    pub data_offset: u8,   //4 bits
    pub reserved: u8,     // 4 bits
    pub control_bit: u8,
    pub window: u16,
    pub checksum: u16,
    pub urgent_pointer: u16,
}
#[allow(dead_code)]
pub struct Packet {
    pub ip_header: IPHeader,
    pub tcp_header: TCPHeader,
    pub data: Vec<u8>,
}

pub fn parser(buffer: &[u8]) -> Option<Packet> {
        if buffer.len() < 20 {
        return None;
    }

    // Parse IP Header
    let ip_header = IPHeader {
        version: (buffer[0] >> 4) & 0x0F,  // First 4 bits
        ihl: buffer[0] & 0x0F,             // Last 4 bits
        type_of_service: buffer[1],
        total_len: u16::from_be_bytes([buffer[2], buffer[3]]),
        identification: u16::from_be_bytes([buffer[4], buffer[5]]),
        flags: (buffer[6] >> 5) & 0x07,    // 3 bits
        fragment_offset: u16::from_be_bytes([buffer[6] & 0x1F, buffer[7]]), // 13 bits
        ttl: buffer[8],
        protocol: buffer[9],
        header_checksum: u16::from_be_bytes([buffer[10], buffer[11]]),
        source: Ipv4Addr::new(buffer[12], buffer[13], buffer[14], buffer[15]),
        destination: Ipv4Addr::new(buffer[16], buffer[17], buffer[18], buffer[19]),
    };

    // Calculate IP header length (IHL is in 32-bit words)
    let ip_header_len = (ip_header.ihl as usize) * 4;
    
    // Checking if buffer is long enough for TCP header
    if buffer.len() < ip_header_len + 20 {  
        return None;
    }

    
    let tcp_start = ip_header_len;
    let tcp_header = TCPHeader {
        source_port: u16::from_be_bytes([buffer[tcp_start], buffer[tcp_start + 1]]),
        destination_port: u16::from_be_bytes([buffer[tcp_start + 2], buffer[tcp_start + 3]]),
        sequence_number: u32::from_be_bytes([
            buffer[tcp_start + 4],
            buffer[tcp_start + 5],
            buffer[tcp_start + 6],
            buffer[tcp_start + 7],
        ]),
        acknowledge_number: u32::from_be_bytes([
            buffer[tcp_start + 8],
            buffer[tcp_start + 9],
            buffer[tcp_start + 10],
            buffer[tcp_start + 11],
        ]),
        data_offset: (buffer[tcp_start + 12] >> 4) & 0x0F,  // First 4 bits
        reserved: buffer[tcp_start + 12] & 0x0F,           // Last 4 bits
        control_bit: buffer[tcp_start + 13],
        window: u16::from_be_bytes([buffer[tcp_start + 14], buffer[tcp_start + 15]]),
        checksum: u16::from_be_bytes([buffer[tcp_start + 16], buffer[tcp_start + 17]]),
        urgent_pointer: u16::from_be_bytes([buffer[tcp_start + 18], buffer[tcp_start + 19]]),
    };

    // Calculate TCP header length (data offset is in 32-bit words)
    let tcp_header_len = (tcp_header.data_offset as usize) * 4;
    
    
    let data_start = ip_header_len + tcp_header_len;
    let data = if data_start < buffer.len() {
        buffer[data_start..].to_vec()
    } else {
        Vec::new()
    };

    Some(Packet {
        ip_header,
        tcp_header,
        data,
    })

}