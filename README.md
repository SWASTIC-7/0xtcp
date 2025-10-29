# TCP Implementation in Rust

OxTCP is an experimental transport-layer engine [RFC-793] written from scratch in Rust. It focuses on robustness and RFC compliance, supporting selective acknowledgments, D-SACK, timestamp options, reassembly queuing, fast recovery, and async integrations for real-world performance testing.

## Features
- [x] Sequence & Acknowledgment Number Tracking
- [x] Data Transmission + ACK Handling
- [x] Retransmission Timer
- [ ] Duplicate ACK & Fast Retransmit
- [ ] Out-of-Order Segments & Reassembly Queue
- [ ] Flow Control
- [ ] Window Scaling Option (RFC 7323)
- [ ] Congestion Control -- Reno, NewReno, Tahoe
- [ ] Selective Acknowledgment (SACK) — RFC 2018
- [ ] Duplicate SACK (D-SACK) — RFC 2883
- [ ] Timestamp Option — RFC 7323
- [ ] TCP Fast Open (TFO) — RFC 7413
- [ ] Delayed ACKs
- [ ] Nagle’s Algorithm
- [ ] TCP Keep-Alive
- [ ] Asynchronous Runtime Integration


## How It Works

1. **Packet Reception**: Listens on `tun0` interface for incoming packets
2. **Parsing**: Parses IPv4 and TCP headers from raw packet data
3. **State Detection**: Identifies TCP control flags (SYN, ACK, FIN, RST)
4. **Response Generation**: Creates appropriate TCP responses (currently SYN-ACK for SYN packets)
5. **Packet Transmission**: Frames and sends response packets back through the TUN interface

### TCP Three-Way Handshake Flow

```
Client              TUN Interface              This Program
  |                      |                           |
  |-------- SYN -------->|                           |
  |                      |-------- SYN ------------>|
  |                      |                           |
  |                      |<------- SYN-ACK ---------|
  |<----- SYN-ACK -------|                           |
  |                      |                           |
  |-------- ACK -------->|                           |
  |                      |-------- ACK ------------>|
  |                      |                           |
```

## Project Structure

```
tcp/
├── src/
│   ├── main.rs           # Main loop and packet reception
│   ├── parser.rs         # IPv4 and TCP header parsing
│   ├── tcp.rs            # TCP state machine and connection handling
│   ├── packet_sender.rs  # Packet framing and checksum calculation
│   ├── sniffer.rs        # Packet logging and sniffing
│   └── tcb.rs            # Transmission Control Block (placeholder)
├── run.sh                # Build and run script with proper setup
└── README.md
```

## Requirements

- Rust (latest stable version)
- Linux operating system
- Root privileges (for TUN interface and network capabilities)
- Dependencies:
  - `tun-tap` crate
  - `etherparse` crate

## Installation

1. Clone the repository:
```bash
git clone <repository-url>
cd tcp
```

2. Build the project:
```bash
cargo build --release
```

## Usage

### Using the provided script (recommended):

```bash
chmod +x run.sh
./run.sh
```

The script will:
- Build the project in release mode
- Set network admin capabilities
- Assign IP address `192.168.0.1/24` to `tun0`
- Bring up the TUN interface
- Run the TCP implementation

### Manual setup:

```bash
cargo build --release
sudo setcap cap_net_admin=eip ./target/release/tcp
./target/release/tcp &
sudo ip addr add 192.168.0.1/24 dev tun0
sudo ip link set up dev tun0
```

### Testing the connection:

In another terminal, you can test the TCP connection:

```bash
# Send a SYN packet
nc 192.168.0.2 80
```

You should see logs showing:
- Incoming SYN packet
- Sequence and acknowledgment numbers
- Outgoing SYN-ACK packet
