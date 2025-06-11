use crate::parser::IPHeader;


impl IPHeader {
    pub fn sniffer(&self){
        
        let proto = match self.protocol {
            1 => String::from("ICMP"),
            6 => String::from("TCP"),
            _ => String::from("other than TCP and ICMP packet"),
        };

        println!("{} packet received from {} ttl={}", proto, self.source, self.ttl)
    }
}
