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
        packet[offset..offset+2].copy_from_slice(&0u16.to_be_bytes()); // temporary zero for checksum
        let checksum_pos = offset;
        offset += 2;

        packet[offset..offset+4].copy_from_slice(&self.ip_header.source.octets()); offset += 4;
        packet[offset..offset+4].copy_from_slice(&self.ip_header.destination.octets()); offset += 4;

        // Now calculate and insert IP checksum
        let ip_checksum = Self::calculate_checksum(&packet[4..24]);
        packet[checksum_pos..checksum_pos+2].copy_from_slice(&ip_checksum.to_be_bytes());

        packet[offset..offset+2].copy_from_slice(&self.tcp_header.source_port.to_be_bytes()); offset += 2;
        packet[offset..offset+2].copy_from_slice(&self.tcp_header.destination_port.to_be_bytes()); offset += 2;
        packet[offset..offset+4].copy_from_slice(&self.tcp_header.sequence_number.to_be_bytes()); offset += 4;
        packet[offset..offset+4].copy_from_slice(&self.tcp_header.acknowledge_number.to_be_bytes()); offset += 4;

        let data_offset_reserved = (self.tcp_header.data_offset << 4) | (self.tcp_header.reserved & 0x0F);
        packet[offset] = data_offset_reserved; offset += 1;
        packet[offset] = self.tcp_header.control_bit; offset += 1;

        packet[offset..offset+2].copy_from_slice(&self.tcp_header.window.to_be_bytes()); offset += 2;
        packet[offset..offset+2].copy_from_slice(&0u16.to_be_bytes()); // placeholder for TCP checksum
        let tcp_checksum_pos = offset;
        offset += 2;
        packet[offset..offset+2].copy_from_slice(&self.tcp_header.urgent_pointer.to_be_bytes()); offset += 2;

        let data_len = self.data.len().min(1504 - offset); // prevent overflow
        // packet[offset..offset+data_len].copy_from_slice(&self.data[..data_len]);

        // TCP checksum
        let tcp_len = (self.tcp_header.data_offset as usize * 4) + data_len;
        let pseudo_header = Self::create_pseudo_header(
            &self.ip_header.source.octets(),
            &self.ip_header.destination.octets(),
            6, // TCP
            tcp_len as u16
        );

        let mut tcp_segment = Vec::new();
        tcp_segment.extend_from_slice(&packet[checksum_pos + 2..offset + data_len]);
        let mut checksum_input = Vec::new();
        checksum_input.extend_from_slice(&pseudo_header);
        checksum_input.extend_from_slice(&tcp_segment);

        let tcp_checksum = Self::calculate_checksum(&checksum_input);
        packet[tcp_checksum_pos..tcp_checksum_pos+2].copy_from_slice(&tcp_checksum.to_be_bytes());

        packet
    }

    fn calculate_checksum(data: &[u8]) -> u16 {
        let mut sum = 0u32;
        let mut i = 0;

        while i + 1 < data.len() {
            let word = ((data[i] as u16) << 8) | (data[i + 1] as u16);
            sum += word as u32;
            i += 2;
        }

        if i < data.len() {
            sum += ((data[i] as u16) << 8) as u32;
        }

        while (sum >> 16) != 0 {
            sum = (sum & 0xFFFF) + (sum >> 16);
        }

        !(sum as u16)
    }

    fn create_pseudo_header(src: &[u8], dst: &[u8], proto: u8, length: u16) -> Vec<u8> {
        let mut header = Vec::new();
        header.extend_from_slice(src);
        header.extend_from_slice(dst);
        header.push(0);
        header.push(proto);
        header.extend_from_slice(&length.to_be_bytes());
        header
    }
}
