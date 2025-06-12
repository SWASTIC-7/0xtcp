use crate::parser::Packet;

impl Packet {
    pub fn create_packet(&self) -> [u8; 1504] {
        let mut packet = [0u8; 1504];
        let mut offset = 0;

        let flags_bytes = 0x0000u16.to_be_bytes();  // [0x00, 0x00]
        let proto_bytes = 0x0800u16.to_be_bytes();  // [0x08, 0x00]
        packet[0..2].copy_from_slice(&flags_bytes);
        packet[2..4].copy_from_slice(&proto_bytes);
        offset += 4;

        packet[offset] = (self.ip_header.version << 4) | (self.ip_header.ihl & 0x0F); offset += 1;
        packet[offset] = self.ip_header.type_of_service; offset += 1;
        packet[offset..offset+2].copy_from_slice(&self.ip_header.total_len.to_be_bytes()); offset += 2;
        packet[offset..offset+2].copy_from_slice(&self.ip_header.identification.to_be_bytes()); offset += 2;

        let flags_fragment = ((self.ip_header.flags as u16) << 13) | (self.ip_header.fragment_offset & 0x1FFF);
        packet[offset..offset+2].copy_from_slice(&flags_fragment.to_be_bytes()); offset += 2;

        packet[offset] = self.ip_header.ttl; offset += 1;
        packet[offset] = self.ip_header.protocol; offset += 1;
        packet[offset..offset+2].copy_from_slice(&self.ip_header.header_checksum.to_be_bytes()); offset += 2;
        packet[offset..offset+4].copy_from_slice(&self.ip_header.source.octets()); offset += 4;
        packet[offset..offset+4].copy_from_slice(&self.ip_header.destination.octets()); offset += 4;

        packet[offset..offset+2].copy_from_slice(&self.tcp_header.source_port.to_be_bytes()); offset += 2;
        packet[offset..offset+2].copy_from_slice(&self.tcp_header.destination_port.to_be_bytes()); offset += 2;
        packet[offset..offset+4].copy_from_slice(&self.tcp_header.sequence_number.to_be_bytes()); offset += 4;
        packet[offset..offset+4].copy_from_slice(&self.tcp_header.acknowledge_number.to_be_bytes()); offset += 4;

        let data_offset_reserved = (self.tcp_header.data_offset << 4) | (self.tcp_header.reserved & 0x0F);
        packet[offset] = data_offset_reserved; offset += 1;
        packet[offset] = self.tcp_header.control_bit; offset += 1;

        packet[offset..offset+2].copy_from_slice(&self.tcp_header.window.to_be_bytes()); offset += 2;
        packet[offset..offset+2].copy_from_slice(&self.tcp_header.checksum.to_be_bytes()); offset += 2;
        packet[offset..offset+2].copy_from_slice(&self.tcp_header.urgent_pointer.to_be_bytes()); offset += 2;

        let data_len = self.data.len().min(1504 - offset); // prevent overflow
        packet[offset..offset+data_len].copy_from_slice(&self.data[..data_len]);

        packet
    }
}
