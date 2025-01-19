pub struct state {

}


impl Default for state {
    fn default() -> Self {
        state {}
    }
}

impl state {
    pub fn on_packet(&mut self, ip_header: etherparse::Ipv4HeaderSlice, tcp_header: etherparse::TcpHeaderSlice, data: &[u8]) {
        println!("tcp source_port:{} destination_port:{}", tcp_header.source_port(), tcp_header.destination_port());
        println!("data {:x?}", data);
    }
}