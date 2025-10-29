# Duplicate ACKs & Fast Retransmit: TCP's Speed Boost

## Introduction

Imagine you're streaming your favorite show, and suddenly... buffering. A packet got lost. Traditional TCP would wait a full second (the retransmission timeout) before resending it. But what if TCP could detect the loss **immediately** and resend the packet in milliseconds instead?

Enter **Fast Retransmit** - one of TCP's most elegant optimizations. By listening to "duplicate ACKs" - the receiver's way of saying "I'm still waiting for that missing packet!" - TCP can recover from loss up to **5-10x faster** than waiting for a timeout.

This isn't just theory - Fast Retransmit is why modern internet feels responsive even with 1-2% packet loss. Let's dive deep into how it works.

---

## Table of Contents

1. [What Are Duplicate ACKs?](#what-are-duplicate-acks)
2. [The Problem with Timeout-Only Recovery](#the-problem-with-timeout-only-recovery)
3. [Fast Retransmit: The Solution](#fast-retransmit-the-solution)
4. [Why Three Duplicate ACKs?](#why-three-duplicate-acks)
5. [Fast Recovery (TCP Reno)](#fast-recovery-tcp-reno)
6. [Implementation Details](#implementation-details)
7. [Real-World Examples](#real-world-examples)
8. [Performance Impact](#performance-impact)

---

## What Are Duplicate ACKs?

### Definition

A **Duplicate ACK** is an acknowledgment that:
1. Has the same ACK number as a previous ACK
2. Advertises the same or larger window
3. Contains no data
4. Is sent in response to an out-of-order segment

### Visual Explanation

```
Normal ACKs (in-order delivery):
──────────────────────────────────────────
Sender                          Receiver
  │                               │
  │─── SEQ=1000, LEN=100 ───────►│
  │                               │ RCV.NXT=1100
  │◄── ACK=1100 ──────────────────│
  │                               │
  │─── SEQ=1100, LEN=100 ───────►│
  │                               │ RCV.NXT=1200
  │◄── ACK=1200 ──────────────────│
  │                               │
  │─── SEQ=1200, LEN=100 ───────►│
  │                               │ RCV.NXT=1300
  │◄── ACK=1300 ──────────────────│

Each ACK is unique - advancing forward


Duplicate ACKs (out-of-order delivery):
──────────────────────────────────────────
Sender                          Receiver
  │                               │
  │─── SEQ=1000, LEN=100 ───────►│
  │                               │ RCV.NXT=1100
  │◄── ACK=1100 ──────────────────│
  │                               │
  │─── SEQ=1100, LEN=100 ───────×│ LOST!
  │                               │
  │─── SEQ=1200, LEN=100 ───────►│ Out of order!
  │                               │ Still RCV.NXT=1100
  │◄── ACK=1100 ──────────────────│ Duplicate #1
  │                               │ (buffered SEQ=1200)
  │─── SEQ=1300, LEN=100 ───────►│ Out of order!
  │                               │ Still RCV.NXT=1100
  │◄── ACK=1100 ──────────────────│ Duplicate #2
  │                               │ (buffered SEQ=1300)
  │─── SEQ=1400, LEN=100 ───────►│ Out of order!
  │                               │ Still RCV.NXT=1100
  │◄── ACK=1100 ──────────────────│ Duplicate #3
  │                               │ (buffered SEQ=1400)

Same ACK=1100 three times = 3 duplicate ACKs!
```

### Why Do Duplicate ACKs Occur?

```
Cause 1: Packet Loss
──────────────────────────────────────────
Segment 2 dropped by network
Segments 3, 4, 5 arrive out of order
Receiver keeps ACKing "still need segment 2"


Cause 2: Packet Reordering
──────────────────────────────────────────
Network takes different paths
Segments arrive: 1, 3, 2, 4
Not really lost - just reordered
(This is why we wait for 3 duplicates!)


Cause 3: Window Update
──────────────────────────────────────────
Receiver's buffer space changes
Sends ACK with same number but new window
Rare and handled specially
```

---

## The Problem with Timeout-Only Recovery

### Timeout-Based Retransmission

Without Fast Retransmit, TCP relies solely on RTO (Retransmission Timeout):

```
Timeline of Loss Recovery (Timeout Only):

T=0ms     Send SEQ=1000 ──────────────►
T=50ms    ◄──── ACK=1100 ────────────────────
          
T=50ms    Send SEQ=1100 ──────────────× LOST!
          Start RTO timer (1000ms)
          
T=100ms   Send SEQ=1200 ──────────────►
T=150ms   ◄──── ACK=1100 ──────────────────── Dup #1
          
T=150ms   Send SEQ=1300 ──────────────►
T=200ms   ◄──── ACK=1100 ──────────────────── Dup #2
          
T=200ms   Send SEQ=1400 ──────────────►
T=250ms   ◄──── ACK=1100 ──────────────────── Dup #3
          
          ... still waiting ...
          
T=1050ms  TIMEOUT! ⏰
          Retransmit SEQ=1100 ─────────►
          
T=1100ms  ◄──── ACK=1500 ────────────────────
          Finally recovered!

Total recovery time: 1050ms
Wasted time: 800ms (waiting when loss was obvious)
```

### The Cost of Waiting

```
Impact on Throughput:
──────────────────────────────────────────
During timeout:
- Sender is idle (not sending new data)
- Receiver is waiting (can't deliver data to app)
- Bandwidth is wasted
- User experiences lag

For a 1 Gbps link with 100ms RTT:
- Bandwidth-Delay Product = 12.5 MB
- Timeout wastes 1 second = 125 MB of potential data!
- That's 10x the pipe capacity wasted
```

---

## Fast Retransmit: The Solution

### The Algorithm (RFC 5681)

Fast Retransmit detects loss **without waiting for timeout**:

```
Fast Retransmit Algorithm:
──────────────────────────────────────────
1. Sender tracks duplicate ACK count per connection

2. When ACK arrives:
   IF ACK number == previous ACK number:
      dup_ack_count++
   ELSE:
      dup_ack_count = 0
      
3. IF dup_ack_count >= 3:
      TRIGGER: Fast Retransmit
      - Retransmit the segment at SND.UNA
      - Enter Fast Recovery (optional)
      - Adjust congestion window

4. Continue sending new data (if window allows)
```

### Visual Timeline

```
Timeline with Fast Retransmit:

T=0ms     Send SEQ=1000 ──────────────►
T=50ms    ◄──── ACK=1100 ────────────────────
          dup_ack_count = 0
          
T=50ms    Send SEQ=1100 ──────────────× LOST!
          
T=100ms   Send SEQ=1200 ──────────────►
T=150ms   ◄──── ACK=1100 ──────────────────── Dup #1
          dup_ack_count = 1
          
T=150ms   Send SEQ=1300 ──────────────►
T=200ms   ◄──── ACK=1100 ──────────────────── Dup #2
          dup_ack_count = 2
          
T=200ms   Send SEQ=1400 ──────────────►
T=250ms   ◄──── ACK=1100 ──────────────────── Dup #3
          dup_ack_count = 3 → FAST RETRANSMIT!
          
T=250ms   Retransmit SEQ=1100 ──────────────►
          (Don't wait for timeout!)
          
T=300ms   ◄──── ACK=1500 ────────────────────
          Recovered!

Total recovery time: 250ms
Improvement: 4x faster than timeout (1050ms → 250ms)
```

---

## Why Three Duplicate ACKs?

### The Threshold Trade-off

Why not trigger on 1 or 2 duplicates? Why specifically 3?

```
With 1 Duplicate ACK:
──────────────────────────────────────────
Problem: Packet reordering triggers false alarms

Network path:
SEQ=1000 ──┐
           ├──► Arrives 1st ✓
SEQ=1100 ──┘
           
SEQ=1200 ──┐
           ├──► Arrives 2nd (reordered!)
SEQ=1300 ──┘

Receiver sends:
ACK=1200 (for 1000)
ACK=1200 (for 1300 - dup!)

1 duplicate → FALSE ALARM
Sender retransmits 1100 unnecessarily!


With 2 Duplicate ACKs:
──────────────────────────────────────────
Still too sensitive to reordering
Studies show ~20% false retransmits


With 3 Duplicate ACKs:
──────────────────────────────────────────
Sweet spot:
✓ Catches real loss quickly
✓ Tolerates typical network reordering
✓ False positives < 1%

Research shows packet reordering rarely exceeds
3 packets in modern networks
```

### Statistics from Real Networks

```
Duplicate ACK Distribution:
──────────────────────────────────────────
Cause                      % of Cases
────────────────────────────────────────────
Loss                       70-80%
Reordering (2 packets)     15-20%
Reordering (3 packets)     5-8%
Reordering (4+ packets)    <2%

Optimal threshold = 3
- Detects 70-80% of losses instantly
- False positives from reordering < 2%
```

---

## Fast Recovery (TCP Reno)

### Beyond Fast Retransmit

Fast Retransmit is half the story. **Fast Recovery** (RFC 5681) keeps sending during recovery:

```
TCP Tahoe (Old - 1988):
──────────────────────────────────────────
Fast Retransmit detected!
1. Retransmit lost segment
2. ssthresh = cwnd / 2
3. cwnd = 1 MSS (slow start)
4. Stop sending until ACK

Problem: Slow restart, wastes bandwidth


TCP Reno (Modern - 1990):
──────────────────────────────────────────
Fast Retransmit detected!
1. Retransmit lost segment
2. ssthresh = cwnd / 2
3. cwnd = ssthresh + 3 MSS (for dupack)
4. KEEP SENDING new data!
5. For each additional dupACK: cwnd += 1 MSS
6. On new ACK: cwnd = ssthresh (exit recovery)

Benefit: Maintains throughput during recovery
```

### Fast Recovery State Machine

```
Normal State:
┌────────────────────────────────────────┐
│ Sending data                           │
│ cwnd growing (slow start or CA)        │
│ dup_ack_count = 0                      │
└────────────────────────────────────────┘
             │
             │ 3rd Duplicate ACK received
             ↓
┌────────────────────────────────────────┐
│ Fast Recovery State                    │
├────────────────────────────────────────┤
│ 1. Retransmit lost segment             │
│ 2. ssthresh = cwnd / 2                 │
│ 3. cwnd = ssthresh + 3                 │
│ 4. For each additional dupACK:         │
│    cwnd += 1 MSS (inflate window)      │
│ 5. Continue sending if window allows   │
└────────────────────────────────────────┘
             │
             │ New ACK received (not duplicate)
             ↓
┌────────────────────────────────────────┐
│ Exiting Fast Recovery                  │
├────────────────────────────────────────┤
│ cwnd = ssthresh (deflate)              │
│ dup_ack_count = 0                      │
│ Resume normal operation                │
└────────────────────────────────────────┘
```

---

## Implementation Details

### 1. Data Structures

```rust
pub struct Tcb {
    // ...existing code...
    
    /// Duplicate ACK tracking
    pub dup_ack_count: u32,
    pub last_ack_received: u32,
    
    /// Fast recovery state
    pub in_fast_recovery: bool,
    pub recovery_point: u32,  // Highest SEQ when entering recovery
}
```

### 2. Processing ACKs

```rust
pub fn process_ack(&mut self, ack: u32, window: u16) -> bool {
    // Check if this is a duplicate ACK
    if ack == self.last_ack_received && ack < self.snd.nxt {
        // Same ACK number - potential duplicate
        
        // Only count as duplicate if:
        // 1. ACK number equals SND.UNA
        // 2. Window hasn't changed (or increased)
        // 3. No data in this segment
        
        if ack == self.snd.una && !self.in_fast_recovery {
            self.dup_ack_count += 1;
            
            println!("Duplicate ACK #{}: ACK={}", 
                self.dup_ack_count, ack);
            
            // Fast Retransmit threshold
            if self.dup_ack_count == 3 {
                self.fast_retransmit();
                return true;
            }
            
            // Fast Recovery: inflate cwnd for each additional dupACK
            if self.in_fast_recovery {
                self.window.cwnd += self.window.mss as u32;
                println!("Fast Recovery: inflate cwnd={}", 
                    self.window.cwnd);
            }
        }
        
        return false; // Duplicate - not new data ACKed
    }
    
    // New ACK - reset duplicate counter
    if ack > self.last_ack_received {
        self.dup_ack_count = 0;
        self.last_ack_received = ack;
        
        // Exit fast recovery if we were in it
        if self.in_fast_recovery {
            if ack >= self.recovery_point {
                self.exit_fast_recovery();
            }
        }
        
        // Normal ACK processing
        self.process_new_ack(ack, window);
        return true;
    }
    
    false
}
```

### 3. Fast Retransmit Trigger

```rust
fn fast_retransmit(&mut self) {
    println!("🚀 FAST RETRANSMIT triggered! (3 dupACKs)");
    
    // 1. Save current state
    self.recovery_point = self.snd.nxt;
    
    // 2. Adjust congestion control (RFC 5681)
    let old_cwnd = self.window.cwnd;
    
    // ssthresh = max(FlightSize / 2, 2*MSS)
    let flight_size = self.snd.nxt.wrapping_sub(self.snd.una);
    self.window.ssthresh = flight_size.max(2 * self.window.mss as u32) / 2;
    
    // cwnd = ssthresh + 3*MSS (account for 3 dupACKs in flight)
    self.window.cwnd = self.window.ssthresh + 3 * self.window.mss as u32;
    
    println!("Congestion window: {} → {}", old_cwnd, self.window.cwnd);
    println!("ssthresh: {}", self.window.ssthresh);
    
    // 3. Retransmit the first unacknowledged segment
    if let Some(segment) = self.retransmission_queue.front() {
        let retransmit_action = RetransmitAction::Retransmit {
            seq: segment.seq,
            flags: segment.flags,
            data: segment.data.clone(),
            attempt: segment.retransmit_count + 1,
        };
        
        // Don't increment retransmit_count for fast retransmit
        // (it's not a timeout-based retransmission)
        
        println!("Fast retransmitting SEQ={}", segment.seq);
        
        // Schedule for transmission
        self.fast_retransmit_action = Some(retransmit_action);
    }
    
    // 4. Enter fast recovery state
    self.in_fast_recovery = true;
}
```

### 4. Exit Fast Recovery

```rust
fn exit_fast_recovery(&mut self) {
    println!("✓ Exiting Fast Recovery");
    
    // Deflate congestion window back to ssthresh
    self.window.cwnd = self.window.ssthresh;
    
    println!("cwnd deflated to: {}", self.window.cwnd);
    
    // Clear recovery state
    self.in_fast_recovery = false;
    self.dup_ack_count = 0;
}
```

### 5. Integration with Main Loop

```rust
// In main.rs - processing received ACKs
if let Some(packet) = parser::parser(&buf[4..nbytes]) {
    if packet.ip_header.protocol == 6 {
        let state = tcp::State::check_state(packet.tcp_header.control_bit);
        
        if state == "ACK" {
            if let Some(tcb) = connections.get_mut(&quad) {
                let is_new_ack = tcb.process_ack(
                    packet.tcp_header.acknowledge_number,
                    packet.tcp_header.window,
                );
                
                // Check if fast retransmit triggered
                if let Some(action) = tcb.fast_retransmit_action.take() {
                    match action {
                        RetransmitAction::Retransmit { seq, flags, data, .. } => {
                            println!("Executing fast retransmit for SEQ={}", seq);
                            
                            let retrans_packet = tcp::State::create_retransmit_packet(
                                &quad, seq, flags, data, tcb
                            );
                            
                            new_interface.send(&retrans_packet)?;
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}
```

---

## Real-World Examples

### Example 1: Single Packet Loss

```
Scenario: Transferring file, 1 packet lost

T=0ms    Send [SEQ=1000, LEN=1460] ─────────►
T=10ms   ◄──── ACK=1460 ────────────────────
         cwnd=10 MSS (14600 bytes)
         
T=10ms   Send [SEQ=1460] ──────────────────× LOST!
         Send [SEQ=2920] ─────────────────────►
         Send [SEQ=4380] ─────────────────────►
         ...up to cwnd...
         
T=60ms   ◄──── ACK=1460 ──────────────────── Dup #1
T=60ms   ◄──── ACK=1460 ──────────────────── Dup #2
T=60ms   ◄──── ACK=1460 ──────────────────── Dup #3
         
         🚀 FAST RETRANSMIT!
         ssthresh = 14600 / 2 = 7300
         cwnd = 7300 + 4380 = 11680
         
T=60ms   Retransmit [SEQ=1460] ──────────────►
         
T=70ms   ◄──── ACK=11680 ────────────────────
         (All data up to 11680 ACKed!)
         
         Exit Fast Recovery
         cwnd = ssthresh = 7300
         
Recovery time: 70ms (vs 1000ms timeout!)
Throughput maintained: Yes
```

### Example 2: Multiple Losses

```
Scenario: Burst loss - 3 consecutive packets lost

T=0ms    Send [1000][1460][2920][3380]... ──►
         
T=50ms   Packet 1460 lost × 
         Packet 2920 lost ×
         Packet 3380 lost ×
         
T=100ms  ◄──── ACK=1460 ──────────────────── Dup #1
         ◄──── ACK=1460 ──────────────────── Dup #2
         ◄──── ACK=1460 ──────────────────── Dup #3
         
         Fast Retransmit SEQ=1460
         
T=100ms  Retransmit [SEQ=1460] ──────────────►
         
T=150ms  ◄──── ACK=2920 ──────────────────── (got 1460)
         Still missing 2920!
         ◄──── ACK=2920 ──────────────────── Dup #1
         ◄──── ACK=2920 ──────────────────── Dup #2
         ◄──── ACK=2920 ──────────────────── Dup #3
         
         Fast Retransmit SEQ=2920
         
T=150ms  Retransmit [SEQ=2920] ──────────────►
         
T=200ms  ◄──── ACK=3380 ──────────────────── (got 2920)
         Still missing 3380!
         
         (Process continues for 3380...)

Note: Multiple losses are harder - Fast Retransmit
      only helps with the first loss per window.
      Solution: SACK (Selective ACK) - RFC 2018
```

### Example 3: Packet Reordering (False Alarm Avoided)

```
Scenario: Packets reordered but not lost

T=0ms    Send [1000][1460][2920] ────────────►
         
         Network reorders: 1460 and 2920 swap
         
T=50ms   Packet 1000 arrives ✓
         Packet 2920 arrives (out of order!)
         
         ◄──── ACK=1460 ──────────────────── Dup #1
         (buffered 2920)
         
T=51ms   Packet 1460 arrives (late, not lost!)
         
         ◄──── ACK=4380 ──────────────────── 
         (delivered 1460 + 2920!)
         
Result: Only 1 duplicate ACK
        Fast Retransmit NOT triggered
        No spurious retransmission ✓
        
This is why threshold is 3, not 1!
```

---

## Performance Impact

### Throughput Comparison

```
Test Scenario:
- 100 Mbps link
- 100ms RTT
- 1% packet loss
- 1 MB file transfer

Without Fast Retransmit (Timeout only):
──────────────────────────────────────────
Average loss recovery: 1000ms per loss
Expected losses: ~10 per 1MB
Total recovery time: 10 seconds
Transfer time: ~15 seconds
Effective throughput: 0.67 Mbps (0.67% utilization!)


With Fast Retransmit:
──────────────────────────────────────────
Average loss recovery: 150ms per loss
Expected losses: ~10 per 1MB
Total recovery time: 1.5 seconds
Transfer time: ~3 seconds
Effective throughput: 2.67 Mbps (2.67% utilization)

Improvement: 4x faster! 🚀


With Fast Retransmit + SACK:
──────────────────────────────────────────
Selective retransmission of lost segments
Transfer time: ~1.5 seconds
Effective throughput: 5.33 Mbps (5.33% utilization)

Improvement: 8x faster than timeout-only!
```

### Latency Impact

```
HTTP Request Latency (1% loss):
──────────────────────────────────────────
Metric                  Without FR    With FR
────────────────────────────────────────────
P50 latency             100ms         100ms
P95 latency             1200ms        250ms
P99 latency             2000ms        500ms
Timeout events          15%           < 1%

Video Streaming (1% loss):
──────────────────────────────────────────
Without FR: Frequent buffering, stuttering
With FR:    Smooth playback, rare rebuffering
```

---

## Key Takeaways

### 🎯 Core Principles

1. **Duplicate ACKs signal loss** - Same ACK number repeated = missing segment
2. **Threshold of 3 is optimal** - Balances speed vs false positives
3. **Fast Retransmit is 5-10x faster** - Than waiting for RTO timeout
4. **Fast Recovery maintains throughput** - Don't slow down during recovery
5. **Track state per connection** - Each TCB needs its own dup_ack_count

### 🔧 Implementation Checklist

```
✓ Track last_ack_received per connection
✓ Maintain dup_ack_count counter
✓ Trigger fast retransmit at 3 duplicates
✓ Implement fast recovery (TCP Reno)
✓ Adjust ssthresh = cwnd / 2
✓ Set cwnd = ssthresh + 3*MSS
✓ Inflate cwnd for additional dupACKs
✓ Exit recovery on new ACK
✓ Don't confuse window updates with dupACKs
✓ Integrate with congestion control
```

### 📊 Performance Gains

| Scenario | Without FR | With FR | Improvement |
|----------|-----------|---------|-------------|
| **1% loss, 100ms RTT** | 1000ms recovery | 150ms recovery | 6.7x faster |
| **Video streaming** | Frequent buffering | Smooth playback | 10x better UX |
| **File transfer** | 0.67 Mbps | 2.67 Mbps | 4x throughput |
| **Web browsing** | P99: 2000ms | P99: 500ms | 4x lower tail latency |

---

## Further Reading

- **RFC 5681** - TCP Congestion Control (includes Fast Retransmit/Recovery) ⭐
- **RFC 2581** - TCP Congestion Control (obsoleted by 5681)
- **RFC 2018** - TCP Selective Acknowledgment Options (SACK)
- **RFC 6675** - A Conservative Loss Recovery Algorithm Based on SACK
- **TCP/IP Illustrated, Volume 1** - W. Richard Stevens (Chapter 21)

---

## Conclusion

Fast Retransmit is one of TCP's most impactful optimizations. By detecting loss through duplicate ACKs instead of waiting for timeouts, it reduces recovery time by **5-10x** and dramatically improves user experience.

The elegance lies in its simplicity:
- **3 duplicate ACKs** = Clear signal of loss
- **Immediate retransmission** = Fast recovery
- **Continued transmission** (Fast Recovery) = Maintained throughput

Every video stream, every web page, every file download benefits from this algorithm running silently in the background. It's the difference between a stuttering connection and a smooth internet experience.

Understanding Fast Retransmit deeply is essential for:
- **Diagnosing loss recovery** - Is it using fast retransmit or timing out?
- **Tuning TCP performance** - Optimize for specific loss patterns
- **Implementing TCP correctly** - Get the state machine right

**Fast Retransmit: Turning loss recovery from seconds into milliseconds! ⚡**

---

*Part of the 0xTCP project - Building TCP from scratch in Rust*
*Previous: [Retransmission Timer](./Retransmission_time.md) | Next: Out-of-Order Segments*