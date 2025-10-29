# Data Transmission & ACK Handling: The Art of Reliable Delivery

## Introduction

After the three-way handshake establishes a connection, the real work begins: **transferring data**. But here's the challenge - the Internet doesn't guarantee delivery, order, or even that packets arrive once. How does TCP transform this chaos into a reliable byte stream?

The answer lies in an elegant dance between **data transmission** and **acknowledgment handling**. Think of it like a conversation where every sentence requires a nod of understanding before moving to the next topic. In this deep dive, we'll explore how TCP ensures every byte arrives exactly once, in perfect order.

---

## Table of Contents

1. [The Data Transfer Problem](#the-data-transfer-problem)
2. [Sending Data: The Transmission Pipeline](#sending-data-the-transmission-pipeline)
3. [Receiving Data: The ACK Response](#receiving-data-the-ack-response)
4. [The Sliding Window Protocol](#the-sliding-window-protocol)
5. [ACK Strategies & Optimization](#ack-strategies--optimization)
6. [Error Scenarios & Recovery](#error-scenarios--recovery)
7. [Performance Implications](#performance-implications)
8. [Implementation Deep Dive](#implementation-deep-dive)

---

## The Data Transfer Problem

### The Challenge

Once a TCP connection is established, we need to transfer data reliably despite:

- **Packet Loss**: Segments disappear into the void
- **Packet Duplication**: Same segment arrives multiple times
- **Packet Reordering**: Segments arrive out of sequence
- **Network Delays**: Variable latency between sender and receiver
- **Limited Bandwidth**: Can't send infinite data at once

### The Solution: Positive Acknowledgment with Retransmission

TCP uses a simple but powerful principle:

```
For every byte sent:
1. Assign it a sequence number
2. Wait for acknowledgment
3. If no ACK arrives in time → Retransmit
4. If ACK arrives → Send next data
```

This creates a **reliable channel** on top of unreliable IP.

---

## Sending Data: The Transmission Pipeline

### The Send Process

When an application writes data to a TCP socket, here's what happens:

```
Application Layer:
    write(socket, "Hello, World!", 13)
             ↓
TCP Send Buffer:
    [H][e][l][l][o][,][ ][W][o][r][l][d][!]
     ↓  ↓  ↓  ↓  ↓  ↓  ↓  ↓  ↓  ↓  ↓  ↓  ↓
    Sequence Numbers Assigned: 1000-1012
             ↓
TCP Segmentation:
    ┌────────────────────────────────┐
    │ TCP Header                     │
    │ SEQ: 1000                      │
    │ LEN: 13                        │
    │ Flags: PSH, ACK                │
    ├────────────────────────────────┤
    │ Data: "Hello, World!"          │
    └────────────────────────────────┘
             ↓
Retransmission Queue:
    Store copy in case we need to resend
             ↓
Network Layer:
    Wrap in IP packet and send!
```

### Key Variables (RFC 793)

```
Send Sequence Space:

    1         2          3          4
------------|----------|----------|----------
         SND.UNA    SND.NXT    SND.UNA+SND.WND

1 = Acknowledged (can discard from buffer)
2 = Sent but not acknowledged (keep for retransmission)
3 = Allowed to send (within window)
4 = Future (blocked by window)

SND.UNA: Send Unacknowledged
         → Oldest byte not yet ACKed
         
SND.NXT: Send Next
         → Next byte we'll send
         
SND.WND: Send Window
         → How many bytes receiver can accept
```

### Transmission Example

```
Initial State:
SND.UNA = 1000
SND.NXT = 1000
SND.WND = 8000  (8KB receive window)

Action: Send 1000 bytes
┌────────────────────────────────────────┐
│ SEQ: 1000, LEN: 1000                   │
│ Data: [... 1000 bytes ...]             │
└────────────────────────────────────────┘

Updated State:
SND.UNA = 1000  (still waiting for ACK)
SND.NXT = 2000  (ready to send next)
SND.WND = 8000

Send Buffer State:
┌─────────────┬─────────────┬─────────────┐
│   Sent &    │   Sent but  │  Ready to   │
│   ACKed     │   not ACKed │    Send     │
├─────────────┼─────────────┼─────────────┤
│   (empty)   │ 1000-1999   │ 2000-8999   │
│             │  ⏳ waiting  │  📦 queued  │
└─────────────┴─────────────┴─────────────┘
           SND.UNA      SND.NXT    SND.UNA+SND.WND
```

---

## Receiving Data: The ACK Response

### The Receive Process

When a TCP segment arrives:

```
Network Layer:
    Packet arrives with SEQ=1000, LEN=1000
             ↓
TCP Processing:
    1. Check if segment is acceptable
    2. Store data in receive buffer
    3. Update RCV.NXT
    4. Decide when to send ACK
             ↓
Send ACK:
    ┌────────────────────────────────┐
    │ TCP Header                     │
    │ ACK: 2000  ← "Send 2000 next"  │
    │ Flags: ACK                     │
    │ Window: 8000 ← "I have 8KB"    │
    └────────────────────────────────┘
             ↓
Deliver to Application:
    read(socket, buffer, size)
    → Returns data to application
```

### Receive Sequence Space

```
Receive Sequence Space:

    1          2          3
------------|----------|----------
         RCV.NXT    RCV.NXT+RCV.WND

1 = Already received (can deliver to app)
2 = Acceptable (within receive window)
3 = Future (would overflow buffer)

RCV.NXT: Receive Next
         → Next byte we expect
         
RCV.WND: Receive Window
         → Buffer space available
```

### ACK Generation Rules

```
Event                          → ACK Action
─────────────────────────────────────────────────────
In-order segment arrives       → Delayed ACK
                                 (wait up to 200ms
                                  for another segment)

In-order segment arrives       → Immediate ACK
with PUSH flag                   (don't delay)

Out-of-order segment arrives   → Immediate ACK
                                 (duplicate ACK
                                  to trigger fast
                                  retransmit)

Gap in sequence detected       → Immediate ACK
                                 (tell sender what
                                  we're missing)

Duplicate segment arrives      → Immediate ACK
                                 (sender may think
                                  ACK was lost)
```

---

## The Sliding Window Protocol

### Concept

Both sender and receiver maintain a **window** of acceptable sequence numbers. This window "slides" forward as data is acknowledged.

```
Sender's Sliding Window:

Time T0: (SND.WND = 8000, SND.UNA = 1000)
┌────────────────────────────────────────────────────┐
│      │◄──────── 8000 bytes ──────────►│            │
├──────┼────────────────────────────────┼────────────┤
│ ACKed│        Can Send                │   Future   │
│      │                                 │   Blocked  │
└──────┴─────────────────────────────────┴────────────┘
     1000                              9000

Send 4000 bytes (SEQ 1000-4999):
┌────────────────────────────────────────────────────┐
│      │◄─ Sent ─►│◄──── 4000 can send ────►│        │
├──────┼──────────┼─────────────────────────┼────────┤
│ ACKed│  Waiting │    Available            │ Future │
│      │  for ACK │                         │        │
└──────┴──────────┴─────────────────────────┴────────┘
     1000       5000                      9000

ACK 3000 received:
┌────────────────────────────────────────────────────┐
│        │◄ Sent  ►│◄──── 6000 can send ────────────►│
├────────┼─────────┼─────────────────────────────────┤
│  ACKed │ Waiting │       Available                 │
│ (grown)│ for ACK │                                 │
└────────┴─────────┴─────────────────────────────────┘
       3000      5000                             11000
        ↑
    Window slid forward by 2000 bytes!
```

### Window Flow Control

The receiver controls the sender's rate by advertising its window size:

```
Scenario 1: Plenty of Buffer Space
────────────────────────────────────
Receiver: RCV.WND = 65535 (64KB)
Sender can send:   64KB at once
Result: High throughput

Scenario 2: Low Buffer Space
────────────────────────────────────
Receiver: RCV.WND = 1024 (1KB)
Sender can send:   1KB at once
Result: Slower, but prevents overflow

Scenario 3: Zero Window
────────────────────────────────────
Receiver: RCV.WND = 0
Sender must stop sending data
Receiver sends: Window Update when space available
```

---

## ACK Strategies & Optimization

### 1. Delayed ACKs (RFC 1122)

Don't ACK every segment immediately - wait a bit:

```
Time →

Segment 1 arrives ──────┐
                        │ Wait up to 200ms
Segment 2 arrives ──────┼──────┐
                        │      │ Send single ACK
Send ACK ───────────────┴──────┘ acknowledging both

Benefits:
✓ Reduces ACK traffic by 50%
✓ Allows piggybacking ACKs on data
✓ Less processing overhead

Drawbacks:
✗ Increases latency by up to 200ms
✗ Can delay retransmissions
```

### 2. Cumulative ACKs

ACKs acknowledge all data up to a sequence number:

```
Sent:     [1000] [1100] [1200] [1300]
            ↓      ↓      ↓      ↓
Received: [1000] [1100] [1200] [1300]

ACKs sent:
ACK 1100 → "Got 1000-1099"
ACK 1200 → "Got 1000-1199" (also acknowledges 1000-1099)
ACK 1300 → "Got 1000-1299" (acknowledges everything)

If ACK 1200 is lost:
ACK 1300 arrives → Sender knows 1000-1299 received
                   (no need for all previous ACKs)
```

### 3. Selective ACKs (SACK - RFC 2018)

Report specific ranges received:

```
Sent:     [1000] [1100] [1200] [1300] [1400]
            ↓      ✗      ↓      ↓      ↓
Received: [1000]        [1200] [1300] [1400]

Standard ACK:
ACK 1100, no SACK → "Only have up to 1099"
Sender must retransmit 1100-1400 (wasteful!)

With SACK:
ACK 1100, SACK: 1200-1499 → "Have 1000-1099 and 1200-1499"
Sender retransmits only 1100-1199 (efficient!)

SACK Block Format:
┌─────────────────────────────────────┐
│ TCP Option: SACK                    │
├─────────────────────────────────────┤
│ Left Edge:  1200 ────┐              │
│ Right Edge: 1499     │ Block 1      │
├──────────────────────┘              │
│ Left Edge:  2000 ────┐              │
│ Right Edge: 2499     │ Block 2      │
└──────────────────────┘              │
  (up to 3 SACK blocks)
```

### 4. Piggybacking

Combine ACKs with data segments:

```
Without Piggybacking:
Client                          Server
   │                               │
   │─── Data: SEQ=1000 ─────────►│
   │                               │
   │◄── ACK=1100 ─────────────────│
   │                               │
   │◄── Data: SEQ=5000 ───────────│
   │                               │
   │─── ACK=5100 ─────────────────►│
   
   Total: 4 packets

With Piggybacking:
Client                          Server
   │                               │
   │─── Data: SEQ=1000 ─────────►│
   │                               │
   │◄── Data: SEQ=5000, ACK=1100 ─│
   │                               │
   │─── Data: SEQ=1100, ACK=5100 ─►│
   
   Total: 3 packets (25% reduction!)
```

---

## Error Scenarios & Recovery

### Scenario 1: Packet Loss

```
Sender                          Receiver
  │                               │
  │─── SEQ=1000, LEN=100 ───────►│ ✓ Received
  │                               │
  │─── SEQ=1100, LEN=100 ───────×│ LOST!
  │                               │
  │─── SEQ=1200, LEN=100 ───────►│ ✓ Out of order
  │                               │
  │◄── ACK=1100 ──────────────────│ "Need 1100"
  │                               │
  │─── SEQ=1300, LEN=100 ───────►│ ✓ Out of order
  │                               │
  │◄── ACK=1100 ──────────────────│ "Still need 1100!"
  │                               │    (duplicate ACK)
  │◄── ACK=1100 ──────────────────│ "Still need 1100!"
  │                               │    (duplicate ACK)
  
  After 3 duplicate ACKs → Fast Retransmit!
  
  │─── SEQ=1100, LEN=100 ───────►│ ✓ Received
  │                               │
  │◄── ACK=1400 ──────────────────│ "Got everything
  │                               │  through 1399!"
```

### Scenario 2: Delayed ACK

```
Sender                          Receiver
  │                               │
  │─── SEQ=1000, LEN=100 ───────►│ ✓ Received
  │                               │ Start 200ms timer
  │                               │
  │                               │ ⏰ Timer expires
  │◄── ACK=1100 ──────────────────│
  │                               │
  │─── SEQ=1100, LEN=100 ───────►│ ✓ Received
  │                               │ Start 200ms timer
  │                               │
  │─── SEQ=1200, LEN=100 ───────►│ ✓ Received
  │                               │ 2nd segment arrived!
  │◄── ACK=1300 ──────────────────│ Send ACK immediately
  │                               │ (acknowledges both)
```

### Scenario 3: Duplicate Data

```
Sender                          Receiver
  │                               │
  │─── SEQ=1000, LEN=100 ───────►│ ✓ Received
  │                               │ RCV.NXT = 1100
  │◄── ACK=1100 ──────────────────│
  │                               │
  │ ACK lost or delayed           │
  │ Retransmit timeout!           │
  │                               │
  │─── SEQ=1000, LEN=100 ───────►│ Duplicate!
  │                               │ SEQ < RCV.NXT
  │                               │ Discard data
  │◄── ACK=1100 ──────────────────│ Re-acknowledge
  │                               │
  ✓ Sender: "OK, they have it"
```

### Scenario 4: Reordering

```
Network reorders packets:

Sender                          Receiver
  │                               │
  │─── SEQ=1000, LEN=100 ───┐    │
  │                          │    │
  │─── SEQ=1100, LEN=100 ───┼───►│ Arrives 2nd!
  │                          │    │ Out of order
  │                          └───►│ Arrives 1st
  │                               │ RCV.NXT = 1100
  │◄── ACK=1100 ──────────────────│
  │                               │
  │                               │ Deliver 1000-1099
  │                               │ Check reassembly queue
  │                               │ Found 1100-1199!
  │                               │ RCV.NXT = 1200
  │◄── ACK=1200 ──────────────────│ 
  │                               │ Deliver both segments
```

---

## Performance Implications

### Bandwidth-Delay Product (BDP)

Maximum throughput is limited by window size and RTT:

```
Throughput ≤ Window Size / RTT

Example:
Window Size = 64 KB (65536 bytes)
RTT = 100 ms (0.1 seconds)

Max Throughput = 65536 / 0.1 = 655,360 bytes/sec
               = 5.24 Mbps

For 1 Gbps link:
Required Window = 1 Gbps × 0.1 sec / 8 bits/byte
                = 12.5 MB

Standard TCP window (64KB) is too small!
Solution: Window Scaling (RFC 7323)
```

### ACK Overhead

```
Scenario: Transferring 1 MB

Without Delayed ACKs:
1 MB data = 683 segments (1460 bytes each)
ACKs sent: 683
Total packets: 1366 (data + ACKs)
ACK overhead: 50%

With Delayed ACKs:
ACKs sent: ~342 (every other segment)
Total packets: 1025
ACK overhead: 25%
Savings: 341 packets (25%)

With SACK:
Lost segment retransmission:
Standard: Retransmit all unACKed data
SACK: Retransmit only missing segments
Savings: 30-50% reduction in retransmissions
```

### Silly Window Syndrome

```
Bad Pattern:
┌────────────────────────────────────────┐
│ App writes: 1 byte                     │
│ TCP sends:  1 byte + 40 byte header    │
│             (2.4% efficiency!)         │
└────────────────────────────────────────┘

Solutions:

1. Nagle's Algorithm (Sender):
   Don't send if:
   - Unacknowledged data exists AND
   - New data < MSS
   
   Wait until:
   - All data ACKed OR
   - Enough data to fill MSS

2. Receiver Delay (Receiver):
   Don't advertise small windows
   Wait until window ≥ min(MSS, 50% of buffer)
```

---

## Implementation Deep Dive

### Our TCP Stack Implementation

#### 1. Sending Data

```rust
pub fn send_data(&mut self, data: &[u8]) -> Result<(), Error> {
    // Check if we can send (within window)
    let available = self.available_window();
    if available == 0 {
        return Err(Error::WindowFull);
    }
    
    // Limit to available window and MSS
    let send_len = data.len()
        .min(available as usize)
        .min(self.window.mss as usize);
    
    let seq = self.snd.nxt;
    let data_to_send = &data[..send_len];
    
    // Create segment
    let segment = create_tcp_segment(
        seq,
        self.rcv.nxt,  // Piggyback ACK
        data_to_send,
        PSH | ACK,
        self.rcv.wnd
    );
    
    // Queue for retransmission
    self.queue_for_retransmission(
        seq,
        PSH | ACK,
        data_to_send.to_vec()
    );
    
    // Update SND.NXT
    self.snd.nxt = seq.wrapping_add(send_len as u32);
    
    // Send the segment
    send_to_network(segment)?;
    
    Ok(())
}
```

#### 2. Receiving and ACKing Data

```rust
pub fn receive_data(&mut self, seg: &Segment) -> Option<Vec<u8>> {
    // Check if segment is acceptable
    if !self.is_segment_acceptable(seg.seq, seg.data.len() as u32) {
        // Out of window - send ACK with current RCV.NXT
        self.send_ack();
        return None;
    }
    
    // In-order segment?
    if seg.seq == self.rcv.nxt {
        // Update RCV.NXT
        self.rcv.nxt = self.rcv.nxt
            .wrapping_add(seg.data.len() as u32);
        
        // Check reassembly queue for more in-order data
        let mut delivered = seg.data.clone();
        while let Some(buffered) = self.get_next_buffered_segment() {
            delivered.extend_from_slice(&buffered);
        }
        
        // Send ACK (possibly delayed)
        self.maybe_send_ack();
        
        Some(delivered)
    } else {
        // Out-of-order - buffer it
        self.buffer_segment(seg.seq, &seg.data);
        
        // Send immediate ACK (duplicate ACK)
        self.send_ack();
        
        None
    }
}
```

#### 3. Processing ACK

```rust
pub fn process_ack(&mut self, ack: u32, window: u16) -> bool {
    // Validate ACK
    if !self.is_ack_acceptable(ack) {
        return self.handle_duplicate_ack(ack);
    }
    
    // Calculate newly acknowledged bytes
    let newly_acked = ack.wrapping_sub(self.snd.una);
    
    // Update send window
    self.snd.wnd = window;
    self.snd.una = ack;
    
    // Remove acknowledged data from retransmission queue
    self.retransmission_queue.retain(|seg| {
        let seg_end = seg.seq.wrapping_add(seg.data.len() as u32);
        seg_end > ack
    });
    
    // Stop retransmission timer if queue empty
    if self.retransmission_queue.is_empty() {
        self.timers.retransmit_timer = None;
    } else {
        // Reset timer for remaining segments
        self.reset_retransmit_timer();
    }
    
    // Update congestion window (slow start / congestion avoidance)
    self.update_cwnd(newly_acked);
    
    // Measure RTT (Karn's Algorithm)
    if let Some(seg) = self.retransmission_queue.front() {
        if seg.retransmit_count == 0 {
            // Only measure RTT for non-retransmitted segments
            if let Some(sent_time) = seg.timestamp {
                let rtt = sent_time.elapsed().as_millis() as u32;
                self.update_rtt(rtt);
            }
        }
    }
    
    true
}
```

#### 4. Congestion Window Update

```rust
fn update_cwnd(&mut self, newly_acked: u32) {
    if self.window.cwnd < self.window.ssthresh {
        // Slow Start: cwnd += MSS for each ACK
        self.window.cwnd += self.window.mss as u32;
        println!("Slow Start: cwnd={}", self.window.cwnd);
    } else {
        // Congestion Avoidance: cwnd += MSS²/cwnd per ACK
        let increment = (self.window.mss as u32 * self.window.mss as u32) 
                        / self.window.cwnd;
        self.window.cwnd += increment.max(1);
        println!("Congestion Avoidance: cwnd={}", self.window.cwnd);
    }
}
```

---

## Real-World Example: HTTP File Transfer

Let's trace a complete file transfer:

```
Transfer: 10 KB file over HTTP
MSS: 1460 bytes
RTT: 50 ms
Window: 8 KB initially

┌─────────────────────────────────────────────────────────┐
│ Phase 1: Connection Establishment (150ms)               │
└─────────────────────────────────────────────────────────┘
T=0ms    Client ─── SYN ──────────────────────► Server
T=50ms   Client ◄── SYN-ACK ──────────────────── Server
T=100ms  Client ─── ACK ──────────────────────► Server
         Connection ESTABLISHED!

┌─────────────────────────────────────────────────────────┐
│ Phase 2: HTTP Request (100ms)                           │
└─────────────────────────────────────────────────────────┘
T=100ms  Client ─── GET /file.dat (200 bytes) ─► Server
                    SEQ=1001, LEN=200
T=150ms  Server ◄── ACK=1201 ──────────────────── Client

┌─────────────────────────────────────────────────────────┐
│ Phase 3: File Transfer (10,240 bytes)                   │
└─────────────────────────────────────────────────────────┘
T=150ms  Server ─── SEQ=5001, LEN=1460 ────────► Client
         Server ─── SEQ=6461, LEN=1460 ────────► Client
         Server ─── SEQ=7921, LEN=1460 ────────► Client
         Server ─── SEQ=9381, LEN=1460 ────────► Client
         Server ─── SEQ=10841, LEN=1460 ───────► Client
         (window full - must wait for ACK)

T=200ms  Server ◄── ACK=10301 ─────────────────── Client
         (acknowledges first 4 segments)
         
         Window slides! Can send more...
         
         Server ─── SEQ=12301, LEN=1460 ───────► Client
         Server ─── SEQ=13761, LEN=1460 ───────► Client
         (window full again)

T=250ms  Server ◄── ACK=15221 ─────────────────── Client
         (acknowledges all sent data)
         
         Only 800 bytes left...
         
         Server ─── SEQ=15221, LEN=800 ────────► Client

T=300ms  Server ◄── ACK=16021 ─────────────────── Client
         ✓ Transfer complete!

┌─────────────────────────────────────────────────────────┐
│ Phase 4: Connection Termination (150ms)                 │
└─────────────────────────────────────────────────────────┘
T=300ms  Server ─── FIN ──────────────────────► Client
T=350ms  Server ◄── ACK ──────────────────────── Client
T=350ms  Server ◄── FIN ──────────────────────── Client
T=400ms  Server ─── ACK ──────────────────────► Client

Total Time: 450ms
Throughput: 10KB / 0.45s = 22.2 KB/s = 177 Kbps

Packet Breakdown:
- Data segments: 8 (10KB / 1.46KB average)
- ACK segments: 4
- Handshake/Teardown: 6
- Total: 18 packets
```

---

## Key Takeaways

### 🎯 Core Principles

1. **Every byte must be acknowledged** - cumulative ACKs provide reliability
2. **Sliding windows enable flow control** - receiver controls sender rate
3. **Delayed ACKs reduce overhead** - but increase latency slightly
4. **Retransmission ensures delivery** - combined with timeouts and duplicate ACKs
5. **Piggybacking improves efficiency** - combine ACKs with data when possible

### 🔧 Optimization Strategies

```
For High Throughput:
✓ Increase window size (Window Scaling)
✓ Enable SACK
✓ Tune congestion control algorithm
✓ Reduce ACK delay (for low-latency apps)

For Low Latency:
✓ Disable Nagle's algorithm (TCP_NODELAY)
✓ Reduce delayed ACK timeout
✓ Use smaller MSS for faster start

For Efficiency:
✓ Enable delayed ACKs
✓ Use SACK to avoid unnecessary retransmissions
✓ Proper congestion window management
```

### 📊 Performance Metrics

| Metric | Formula | Impact |
|--------|---------|--------|
| **Throughput** | Window / RTT | Bigger window = faster |
| **Goodput** | (Data - Retransmits) / Time | Higher = better efficiency |
| **ACK Rate** | ACKs / Segments | Lower = less overhead |
| **Retransmit Rate** | Retransmits / Segments | Lower = better network |

---

## Further Reading

- **RFC 793** - Transmission Control Protocol
- **RFC 1122** - Requirements for Internet Hosts (Delayed ACKs)
- **RFC 2018** - TCP Selective Acknowledgment Options
- **RFC 5681** - TCP Congestion Control
- **RFC 7323** - TCP Extensions for High Performance
- **RFC 896** - Congestion Control in IP/TCP (Nagle's Algorithm)

---

## Conclusion

Data transmission and ACK handling are the beating heart of TCP's reliability. The elegant interplay between sequence numbers, acknowledgments, and sliding windows transforms the chaotic Internet into a reliable, ordered byte stream.

Understanding these mechanisms isn't just academic - it's essential for:
- **Debugging network issues** - Why is my transfer slow?
- **Optimizing performance** - How can I make this faster?
- **Implementing TCP correctly** - What happens if I receive out-of-order data?

Every HTTP request, every video stream, every file download relies on this dance of data and acknowledgments happening billions of times per second across the Internet.

**Master the ACK, master the Internet! 🚀**

---

*Part of the 0xTCP project - Building TCP from scratch in Rust*
*[Read more about Sequence Numbers](./sequence_and_ack_number_tracking.md)*
