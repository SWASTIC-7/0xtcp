# TCP Project

## Description
[Brief description of what your TCP project does]

## Features
- Sequence & Acknowledgment Number Tracking
- Data Transmission + ACK Handling
- Retransmission Timer
- Duplicate ACK & Fast Retransmit
- Out-of-Order Segments & Reassembly Queue
- Flow Control
- Window Scaling Option (RFC 7323)
- Congestion Control -- Reno, NewReno, Tahoe
- Selective Acknowledgment (SACK) — RFC 2018
- Duplicate SACK (D-SACK) — RFC 2883
- Timestamp Option — RFC 7323
- TCP Fast Open (TFO) — RFC 7413
- Delayed ACKs
- Nagle’s Algorithm
- TCP Keep-Alive
- Asynchronous Runtime Integration

## Requirements
- Rust


## Usage
```bash
 ./run.sh
```

## Examples
 Start listening using ```./run.sh```, you will get the tun interface. send packets to the tun interface using netcat


