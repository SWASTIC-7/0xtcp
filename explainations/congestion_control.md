# Congestion Control: TCP's Traffic Management System

## Introduction

Imagine a highway system where every driver drives as fast as possible without considering traffic. Cars would pile up, accidents would occur, and eventually the entire system would grind to a halt - a phenomenon called **congestion collapse**. This nearly happened to the Internet in 1986 when sudden congestion caused throughput to drop from 32 Kbps to 40 bps - a 1000× reduction!

**TCP Congestion Control** is the Internet's traffic management system. It ensures that senders don't overwhelm the network, adapting transmission rates based on network conditions. Unlike flow control (which protects the receiver), congestion control protects the **network itself**.

In this deep dive, we'll explore the three foundational congestion control algorithms that saved the Internet: **Tahoe**, **Reno**, and **NewReno**. Each builds upon its predecessor, becoming progressively smarter about handling packet loss.

---

## Table of Contents

1. [The Congestion Problem](#the-congestion-problem)
2. [Core Concepts](#core-concepts)
3. [TCP Tahoe: The Pioneer](#tcp-tahoe-the-pioneer)
4. [TCP Reno: Fast Recovery](#tcp-reno-fast-recovery)
5. [TCP NewReno: Partial ACK Handling](#tcp-newreno-partial-ack-handling)
6. [Algorithm Comparison](#algorithm-comparison)
7. [Implementation Deep Dive](#implementation-deep-dive)
8. [Real-World Examples](#real-world-examples)

---

## The Congestion Problem

### What is Network Congestion?

Congestion occurs when the network has more data to transmit than it can handle:

```
Network Congestion Scenario:
────────────────────────────────────────────────────

Router Buffer:
┌─────────────────────────────────────────┐
│ [▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓] ← FULL! │
└─────────────────────────────────────────┘
          ↑         ↑         ↑
    Incoming    Incoming  Incoming
    100 Mbps    100 Mbps  100 Mbps
    
    Outgoing: Only 100 Mbps available!
    
Result:
✗ Packets dropped (buffer overflow)
✗ Retransmissions increase load
✗ More packets dropped
✗ CONGESTION COLLAPSE!
```

### The 1986 Internet Collapse

```
Historical Context:
────────────────────────────────────────────────────

Before Congestion Control:
October 1986 - Internet throughput: 32 Kbps
                Network suddenly congested
                Throughput dropped to: 40 bps
                
Reduction: 32,000 / 40 = 800× slower!

Cause: No congestion control
- All senders transmitted at maximum rate
- Router buffers overflowed
- Massive packet loss triggered retransmissions
- Retransmissions caused more congestion
- Positive feedback loop → collapse

Van Jacobson's Solution (1988):
Introduced congestion control algorithms:
- Slow Start
- Congestion Avoidance
- Fast Retransmit
- Fast Recovery

Result: Internet saved! 🎉
```

### Flow Control vs. Congestion Control

```
┌────────────────────────────────────────────────┐
│              Flow Control                      │
├────────────────────────────────────────────────┤
│ Problem:    Overwhelming the receiver          │
│ Controlled: Receiver's buffer (rwnd)           │
│ Feedback:   Window advertisements              │
│ Goal:       Protect receiver                   │
└────────────────────────────────────────────────┘

┌────────────────────────────────────────────────┐
│           Congestion Control                   │
├────────────────────────────────────────────────┤
│ Problem:    Overwhelming the network           │
│ Controlled: Congestion window (cwnd)           │
│ Feedback:   Packet loss, ACKs                  │
│ Goal:       Protect network, fair sharing      │
└────────────────────────────────────────────────┘

Combined Effect:
────────────────────────────────────────────────────
Effective Window = min(rwnd, cwnd)

Sender is limited by BOTH:
- Receiver's capacity (flow control)
- Network's capacity (congestion control)
```

---

## Core Concepts

### The Congestion Window (cwnd)

```
┌────────────────────────────────────────────┐
│ Congestion Window (cwnd)                   │
├────────────────────────────────────────────┤
│ Definition: Sender's estimate of network   │
│            capacity                        │
│                                            │
│ Unit:       Bytes (or segments)            │
│                                            │
│ Controls:   How much data can be in-flight│
│            at any time                     │
│                                            │
│ Updates:    Dynamically based on network   │
│            feedback (ACKs, losses)         │
└────────────────────────────────────────────┘
```

### Slow Start Threshold (ssthresh)

```
┌────────────────────────────────────────────┐
│ Slow Start Threshold (ssthresh)            │
├────────────────────────────────────────────┤
│ Definition: Boundary between slow start    │
│            and congestion avoidance        │
│                                            │
│ Initial:    Very large (∞ or 65535)        │
│                                            │
│ Updated:    On congestion events           │
│            ssthresh = max(cwnd/2, 2*MSS)   │
│                                            │
│ Purpose:    Remember previous congestion   │
│            point                           │
└────────────────────────────────────────────┘
```

### The Two Phases

```
Congestion Control Phases:
────────────────────────────────────────────────────

┌──────────────────────────────────────────┐
│         Phase 1: SLOW START              │
├──────────────────────────────────────────┤
│ When:    cwnd < ssthresh                 │
│ Growth:  Exponential (double per RTT)    │
│ Rate:    cwnd += MSS for each ACK        │
│ Goal:    Quickly find network capacity   │
└──────────────────────────────────────────┘
                   ↓
         cwnd reaches ssthresh
                   ↓
┌──────────────────────────────────────────┐
│    Phase 2: CONGESTION AVOIDANCE         │
├──────────────────────────────────────────┤
│ When:    cwnd ≥ ssthresh                 │
│ Growth:  Linear (increase by MSS²/cwnd per ACK)│
│ Rate:    cwnd += MSS²/cwnd per ACK      │
│ Goal:    Cautiously probe for more       │
│         capacity                         │
└──────────────────────────────────────────┘


Visual Growth Pattern:
────────────────────────────────────────────────────

cwnd
  ^
  │                    ╱ Congestion Avoidance
  │                  ╱  (linear growth)
  │                ╱
  │              ╱
  │            ╱
  │          ╱
  │        ╱ Slow Start
  │      ╱  (exponential)
  │    ╱
  │  ╱
  │╱
  └─────────────────────────────────────────────> Time
        ssthresh
```

---

## TCP Tahoe: The Pioneer

### Overview

**TCP Tahoe** (1988) was the first TCP implementation with congestion control, introduced by Van Jacobson. It's named after Lake Tahoe.

### Key Characteristics

```
┌────────────────────────────────────────────┐
│            TCP Tahoe                       │
├────────────────────────────────────────────┤
│ Year:           1988                       │
│ Author:         Van Jacobson               │
│ Innovation:     Slow Start, Congestion     │
│                Avoidance                   │
│ Loss Response:  Always reset to cwnd=1 MSS │
│ Recovery:       Slow Start                 │
└────────────────────────────────────────────┘
```

### Algorithm

```
Tahoe State Machine:
────────────────────────────────────────────────────

┌─────────────────────────────────────────────┐
│ Initial State:                              │
│   cwnd = 1 MSS                              │
│   ssthresh = 65535 (large value)            │
└─────────────────────────────────────────────┘
                    ↓
        ┌───────────────────────┐
        │   SLOW START          │
        │ cwnd < ssthresh       │
        │                       │
        │ On each ACK:          │
        │   cwnd += MSS         │
        │                       │
        │ Growth: Exponential   │
        └───────────────────────┘
                    ↓
         cwnd ≥ ssthresh
                    ↓
        ┌───────────────────────┐
        │ CONGESTION AVOIDANCE  │
        │ cwnd ≥ ssthresh       │
        │                       │
        │ On each ACK:          │
        │   cwnd += MSS²/cwnd   │
        └───────────────────────┘
                    ↓
    ┌───────────────────────────────┐
    │ Loss Detected:                │
    │ - Timeout OR                  │
    │ - 3 Duplicate ACKs            │
    └───────────────────────────────┘
                    ↓
        ┌───────────────────────┐
        │ CONGESTION EVENT      │
        │                       │
        │ Set:                  │
        │   ssthresh = cwnd/2   │
        │   cwnd = 1 MSS        │
        │                       │
        │ Action:               │
        │   Retransmit lost     │
        │   Return to SLOW START│
        └───────────────────────┘
```

### Numeric Example

```
Tahoe Simulation (MSS = 1460 bytes):
────────────────────────────────────────────────────

RTT 0:
  cwnd = 1 MSS = 1,460 bytes
  Send 1 segment
  
RTT 1: ← ACK received
  cwnd = 1 + 1 = 2 MSS = 2,920 bytes
  Send 2 segments
  
RTT 2: ← 2 ACKs received
  cwnd = 2 + 2 = 4 MSS = 5,840 bytes
  Send 4 segments
  
RTT 3: ← 4 ACKs received
  cwnd = 4 + 4 = 8 MSS = 11,680 bytes
  Send 8 segments
  
RTT 4: ← 8 ACKs received
  cwnd = 8 + 8 = 16 MSS = 23,360 bytes
  Send 16 segments
  
RTT 5: ← 16 ACKs received
  cwnd = 16 + 16 = 32 MSS = 46,720 bytes
  Reached ssthresh = 65,535 (still in slow start)
  Send 32 segments
  
  ... slow start continues ...
  
RTT 10:
  cwnd = 512 MSS = 747,520 bytes
  Send 512 segments
  
  ❌ PACKET LOSS! (3 duplicate ACKs)
  
LOSS EVENT:
  ssthresh = cwnd / 2 = 512 / 2 = 256 MSS
  cwnd = 1 MSS = 1,460 bytes
  
RTT 11: (Back to slow start)
  cwnd = 1 MSS
  Send 1 segment
  
RTT 12: ← ACK
  cwnd = 2 MSS
  Send 2 segments
  
  ... slow start continues ...
  
RTT 19:
  cwnd = 256 MSS (reached ssthresh!)
  Switch to congestion avoidance
  
RTT 20: (Congestion avoidance)
  cwnd = 256 + (1460²/374,080) ≈ 256.0055 MSS
  Send 256 segments


Graph of cwnd over time:
────────────────────────────────────────────────────

cwnd
  ^
  │           ╱╲
  │          ╱  ╲
  │         ╱    ╲
  │        ╱      ╲___Reno/NewReno (similar)
  │       ╱            ────────
  │      ╱
  │    ╱ Slow Start
  │  ╱  (exponential)
  │╱
  └─────────────────────────────────────────────> Time (RTT)
        ssthresh
```

### Pseudocode

```rust
// Tahoe Algorithm
fn tahoe_on_ack(&mut self) {
    if self.cwnd < self.ssthresh {
        // Slow Start
        self.cwnd += self.mss;
    } else {
        // Congestion Avoidance
        let increment = (self.mss * self.mss) / self.cwnd;
        self.cwnd += increment.max(1);
    }
}

fn tahoe_on_loss(&mut self) {
    // On timeout or 3 dup ACKs
    self.ssthresh = self.cwnd.max(2 * self.mss) / 2;
    self.cwnd = self.mss;  // Back to 1 MSS
    // Retransmit lost segment
    // Return to Slow Start
}
```

---

## TCP Reno: Fast Recovery

### Overview

**TCP Reno** (1990) improved upon Tahoe by distinguishing between timeout losses (severe) and duplicate ACK losses (mild congestion).

### Key Innovation: Fast Recovery

```
┌────────────────────────────────────────────┐
│            TCP Reno                        │
├────────────────────────────────────────────┤
│ Year:           1990                       │
│ Innovation:     Fast Recovery              │
│ Key Insight:    3 dup ACKs means network   │
│                is still delivering packets │
│                (not complete congestion)   │
│ Loss Response:  Depends on loss type       │
│ - Timeout:      cwnd = 1 MSS (like Tahoe) │
│ - 3 Dup ACKs:   cwnd = ssthresh (fast rec) │
└────────────────────────────────────────────┘
```

### Algorithm

```
Reno State Machine:
────────────────────────────────────────────────────

        ┌───────────────────────┐
        │   SLOW START          │
        │ cwnd < ssthresh       │
        │                       │
        │ On each ACK:          │
        │   cwnd += MSS         │
        └───────────────────────┘
                    ↓
         cwnd ≥ ssthresh
                    ↓
        ┌───────────────────────┐
        │ CONGESTION AVOIDANCE  │
        │ cwnd ≥ ssthresh       │
        │                       │
        │ On each ACK:          │
        │   cwnd += MSS²/cwnd   │
        └───────────────────────┘
                    ↓
              Loss Detected
                    ↓
        ┌─────────────────────────┐
        │   Which Loss Type?      │
        └─────────────────────────┘
              ↙            ↘
        Timeout        3 Dup ACKs
              ↓                ↓
    ┌─────────────────┐  ┌───────────────────┐
    │ Like Tahoe:     │  │ Fast Recovery:    │
    │                 │  │                   │
    │ ssthresh=cwnd/2 │  │ ssthresh=cwnd/2   │
    │ cwnd = 1 MSS    │  │ cwnd=ssthresh+3   │
    │                 │  │                   │
    │ Return to       │  │ Retransmit        │
    │ Slow Start      │  │ Stay here!        │
    └─────────────────┘  └───────────────────┘
                                  ↓
                    ┌─────────────────────────┐
                    │  Fast Recovery Loop:    │
                    │                         │
                    │  On dup ACK:            │
                    │    cwnd += MSS          │
                    │    (inflate window)     │
                    │                         │
                    │  On new ACK:            │
                    │    cwnd = ssthresh      │
                    │    Exit to Cong. Avoid. │
                    └─────────────────────────┘
```

### Numeric Example

```
Reno Simulation with Fast Recovery:
────────────────────────────────────────────────────

Initial:
  cwnd = 1 MSS
  ssthresh = 65535
  
RTT 0:
  cwnd = 1 MSS = 1,460 bytes
  Send 1 segment
  
RTT 1: ← ACK received
  cwnd = 1 + 1 = 2 MSS = 2,920 bytes
  Send 2 segments
  
RTT 2: ← 2 ACKs received
  cwnd = 2 + 2 = 4 MSS = 5,840 bytes
  Send 4 segments
  
RTT 3: ← 4 ACKs received
  cwnd = 4 + 4 = 8 MSS = 11,680 bytes
  Send 8 segments
  
RTT 4: ← 8 ACKs received
  cwnd = 8 + 8 = 16 MSS = 23,360 bytes
  Send 16 segments
  
RTT 5: ← 16 ACKs received
  cwnd = 16 + 16 = 32 MSS = 46,720 bytes
  Send 32 segments
  
RTT 6: ← 32 ACKs received
  cwnd = 32 + 32 = 64 MSS = 93,440 bytes
  Send 64 segments
  
RTT 7: ← 64 ACKs received
  cwnd = 64 + 64 = 128 MSS = 186,880 bytes
  Send 128 segments
  
RTT 8: ← 128 ACKs received
  cwnd = 128 + 128 = 256 MSS = 373,760 bytes
  Send 256 segments
  
RTT 9: ← 256 ACKs received
  cwnd = 256 + 256 = 512 MSS = 747,520 bytes
  Send 512 segments
  
  ❌ PACKET LOSS! (3 duplicate ACKs)
  
Receiver gets: 1-255, 257-512 (triggers dup ACKs)

Client receives:
  ACK 256 (for segment 255)
  ACK 256 (dup #1, triggered by seg 257)
  ACK 256 (dup #2, triggered by seg 258)
  ACK 256 (dup #3, triggered by seg 259) ← Trigger!

FAST RECOVERY TRIGGERED:
  ssthresh = cwnd / 2 = 512 / 2 = 256 MSS
  cwnd = ssthresh + 3 = 256 + 3 = 259 MSS
  Retransmit segment #256
  
RTT 10: (In Fast Recovery)
  Receive more duplicate ACKs (260-512 still arriving)
  
  Each dup ACK:
    cwnd += MSS  (window inflation)
    cwnd = 260, 261, 262, ... 515
    
  Can send new data! (cwnd - in_flight > 0)
  
RTT 11:
  Receive new ACK: ACK 512 (all data acknowledged!)
  
EXIT FAST RECOVERY:
  cwnd = ssthresh = 256 MSS
  Enter Congestion Avoidance
  
RTT 12: (Congestion Avoidance)
  cwnd = 256 + (1460²/374,080) ≈ 256.0055 MSS
  Send 256 segments


Comparison to Tahoe:
────────────────────────────────────────────────────

Reno Timeline:
  RTT 10: Enter Fast Recovery, retransmit 256
  RTT 11: Receive ACK 512 (full)
          Exit Fast Recovery
          cwnd = 256 (much better!)
  RTT 12: Congestion avoidance

Tahoe Timeline:
  RTT 10: Timeout, cwnd = 1 MSS
  RTT 11: Slow start: cwnd = 2 MSS
  RTT 12: Slow start: cwnd = 4 MSS
  ...
  RTT 20: Tahoe finally recovers
```

### Pseudocode

```rust
// Reno Algorithm
enum CongestionState {
    SlowStart,
    CongestionAvoidance,
    FastRecovery,
}

fn reno_on_ack(&mut self, is_new_ack: bool) {
    match self.state {
        CongestionState::SlowStart => {
            if is_new_ack {
                self.cwnd += self.mss;
                if self.cwnd >= self.ssthresh {
                    self.state = CongestionState::CongestionAvoidance;
                }
            }
        }
        CongestionState::CongestionAvoidance => {
            if is_new_ack {
                let increment = (self.mss * self.mss) / self.cwnd;
                self.cwnd += increment.max(1);
            }
        }
        CongestionState::FastRecovery => {
            if is_new_ack {
                // Exit fast recovery
                self.cwnd = self.ssthresh;
                self.state = CongestionState::CongestionAvoidance;
            } else {
                // Duplicate ACK in fast recovery - inflate window
                self.cwnd += self.mss;
            }
        }
    }
}

fn reno_on_3_dup_acks(&mut self) {
    // Fast Retransmit + Fast Recovery
    self.ssthresh = self.cwnd.max(2 * self.mss) / 2;
    self.cwnd = self.ssthresh + 3 * self.mss;
    self.state = CongestionState::FastRecovery;
    // Retransmit lost segment
}

fn reno_on_timeout(&mut self) {
    // Same as Tahoe
    self.ssthresh = self.cwnd.max(2 * self.mss) / 2;
    self.cwnd = self.mss;
    self.state = CongestionState::SlowStart;
}
```

---

## TCP NewReno: Partial ACK Handling

### Overview

**TCP NewReno** (1999, RFC 2582, updated RFC 6582) improved Reno's handling of **multiple packet losses** in a single window.

### The Problem with Reno

```
Reno's Weakness: Multiple Losses
────────────────────────────────────────────────────

Scenario: 2 packets lost in same window

Sent:     [100] [200] [300] [400] [500]
           ✓     ✗     ✗     ✓     ✓

Receiver: [100]             [400] [500]
ACKs:     ACK 200           ACK 200 (dup #1)
                            ACK 200 (dup #2)
                            ACK 200 (dup #3)

Reno Fast Recovery:
1. Detect 3 dup ACKs
2. Retransmit SEG 200
3. Enter Fast Recovery

Problem:
────────────────────────────────────────────────────
Receiver gets: [200]       [400] [500]
Sends: ACK 300 ← Partial ACK!

Reno interprets this as:
  "All data recovered, exit fast recovery"
  
But SEG 300 is STILL LOST!

Result:
  Exit fast recovery too early
  Wait for timeout to retransmit 300
  Poor performance ❌
```

### NewReno's Solution

```
┌────────────────────────────────────────────┐
│          TCP NewReno                       │
├────────────────────────────────────────────┤
│ Year:           1999 (RFC 2582)            │
│ Innovation:     Partial ACK handling       │
│ Key Insight:    Partial ACK = more losses  │
│ Behavior:       Stay in Fast Recovery until│
│                all data from original     │
│                window is ACKed             │
└────────────────────────────────────────────┘
```

### Algorithm

```
NewReno Fast Recovery:
────────────────────────────────────────────────────

Enter Fast Recovery:
  recover = SND.NXT  (highest seq sent)
  ssthresh = cwnd / 2
  cwnd = ssthresh + 3
  Retransmit first lost segment
  
While in Fast Recovery:
  
  On Duplicate ACK:
    cwnd += MSS  (inflate)
    
  On Partial ACK (ACK < recover):
    // More losses!
    Retransmit next unACKed segment
    cwnd = cwnd - (ACK - old_ACK) + MSS
    // Deflate by amount ACKed, inflate by 1
    
  On Full ACK (ACK ≥ recover):
    // All original data ACKed
    cwnd = ssthresh
    Exit Fast Recovery
```

### Numeric Example

```
NewReno with Multiple Losses:
────────────────────────────────────────────────────

Initial:
  cwnd = 512 MSS
  SND.NXT = 51,200 (sent 512 segments of 100 bytes)
Lost:     200-300, 300-400 (2 consecutive losses)

Receiver gets: 1-199, 201-512 (triggers dup ACKs)

Client receives:
  ACK 200 (for segment 199)
  ACK 200 (dup #1, triggered by seg 201)
  ACK 200 (dup #2, triggered by seg 202)
  ACK 200 (dup #3, triggered by seg 203) ← Trigger!

FAST RECOVERY TRIGGERED:
  recover = 51,200 (highest sent)
  ssthresh = cwnd / 2 = 512 / 2 = 256 MSS
  cwnd = ssthresh + 3 = 256 + 3 = 259 MSS
  Retransmit segment #200
  
RTT 10: (In Fast Recovery)
  Receive more duplicate ACKs (204-512 still arriving)
  
  Each dup ACK:
    cwnd += MSS  (window inflation)
    cwnd = 259, 260, 261, ... 512
    
  Can send new data! (cwnd - in_flight > 0)
  
RTT 11:
  Receive new ACK: ACK 512 (all data acknowledged!)
  
EXIT FAST RECOVERY:
  cwnd = ssthresh = 256 MSS
  Enter Congestion Avoidance
  
RTT 12: (Congestion Avoidance)
  cwnd = 256 + (MSS²/cwnd) ≈ 256.0055 MSS
  Send 256 segments


Comparison to Reno:
────────────────────────────────────────────────────

Reno Timeline:
  RTT 10: Enter Fast Recovery, retransmit 200
  RTT 11: Receive ACK 300 (partial)
          Exit Fast Recovery
  RTT 12: Timeout waiting for 300
  RTT 13: Retransmit 300, cwnd = 1
  RTT 14: Back to Slow Start
  
NewReno Timeline:
  RTT 10: Enter Fast Recovery, retransmit 200
  RTT 11: Receive ACK 300 (partial)
          Stay in Fast Recovery
          Retransmit 300
  RTT 12: Receive ACK 51,200 (full)
          Exit Fast Recovery
          cwnd = 256 (much better!)
  RTT 13: Congestion avoidance
```

### Pseudocode

```rust
// NewReno Algorithm
fn newreno_on_3_dup_acks(&mut self) {
    self.recover = self.snd.nxt;  // Mark recovery point
    self.ssthresh = self.cwnd.max(2 * self.mss) / 2;
    self.cwnd = self.ssthresh + 3 * self.mss;
    self.state = CongestionState::FastRecovery;
    // Retransmit lost segment
}

fn newreno_on_ack_in_fast_recovery(&mut self, ack: u32) {
    if ack < self.recover {
        // PARTIAL ACK - more losses
        let newly_acked = ack - self.last_ack;
        
        // Deflate by amount ACKed, inflate by 1 MSS
        self.cwnd = self.cwnd.saturating_sub(newly_acked) + self.mss;
        
        // Retransmit next unACKed segment
        self.retransmit_next_unacked();
        
        // Stay in Fast Recovery
    } else {
        // FULL ACK - recovery complete
        self.cwnd = self.ssthresh;
        self.state = CongestionState::CongestionAvoidance;
    }
}
```

---

## Algorithm Comparison

### Side-by-Side Comparison

```
┌─────────────────────────────────────────────────────────────────┐
│                  Tahoe vs Reno vs NewReno                       │
├─────────────┬──────────────┬──────────────┬─────────────────────┤
│             │    Tahoe     │     Reno     │      NewReno        │
├─────────────┼──────────────┼──────────────┼─────────────────────┤
│ Year        │ 1988         │ 1990         │ 1999                │
├─────────────┼──────────────┼──────────────┼─────────────────────┤
│ On Timeout  │ cwnd = 1 MSS │ cwnd = 1 MSS │ cwnd = 1 MSS        │
│             │ Slow Start   │ Slow Start   │ Slow Start          │
├─────────────┼──────────────┼──────────────┼─────────────────────┤
│ On 3 Dup    │ cwnd = 1 MSS │ cwnd = ss+3  │ cwnd = ss+3         │
│ ACKs        │ Slow Start   │ Fast Recovery│ Fast Recovery       │
├─────────────┼──────────────┼──────────────┼─────────────────────┤
│ Partial ACK │ N/A          │ Exit Fast    │ Stay in Fast        │
│ in Fast Rec │              │ Recovery     │ Recovery, retransmit│
├─────────────┼──────────────┼──────────────┼─────────────────────┤
│ Best For    │ High loss    │ Single packet│ Multiple packet     │
│             │ Simple       │ losses       │ losses in window    │
├─────────────┼──────────────┼──────────────┼─────────────────────┤
│ Performance │ Baseline     │ 15-20% faster│ 25-30% faster       │
│ (vs Tahoe)  │              │ than Tahoe   │ than Tahoe          │
└─────────────┴──────────────┴──────────────┴─────────────────────┘
```

### Performance Graph

```
Throughput Comparison (Single Loss):
────────────────────────────────────────────────────

cwnd
 ^
 │    Tahoe
 │      ╱╲
 │     ╱  ╲
 │    ╱    ╲
 │   ╱      ╲___Reno/NewReno (similar)
 │  ╱            ────────
 │      ╱╲
 │     ╱  ╲
 │    ╱    ╲
 │   ╱      ╲
 │  ╱        ╲
 │╱          ╲
 └─────────────────────────────────────────────> Time
       Loss

Average throughput: Reno ≈ NewReno > Tahoe


Throughput Comparison (Multiple Losses):
────────────────────────────────────────────────────

cwnd
 ^
 │      Tahoe
 │        ╱╲    NewReno best!
 │       ╱  ╲     ╱────
 │      ╱    ╲   ╱
 │     ╱      ╲_╱  Reno (extra timeout)
 │    ╱         ╲╱╲
 │   ╱            ╲___
 │  ╱
 │ ╱
 │╱____________________________> Time
      Loss Loss

Average throughput: NewReno > Reno > Tahoe
```

---

## Implementation Deep Dive

### Pluggable Congestion Control Trait

```rust
// filepath: /home/nazr/Desktop/projects/tcp/src/congestion_control.rs

use std::fmt;

/// Congestion control actions that can be returned
#[derive(Debug, Clone)]
pub enum CongestionAction {
    /// Update congestion window
    UpdateCwnd(u32),
    /// Update slow start threshold
    UpdateSsthresh(u32),
    /// Retransmit a specific segment
    Retransmit(u32),  // sequence number
    /// Exit fast recovery
    ExitFastRecovery,
}

/// Congestion control state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CongestionState {
    SlowStart,
    CongestionAvoidance,
    FastRecovery,
}

/// Generic congestion control trait
pub trait CongestionControl: fmt::Debug {
    /// Called when a new (non-duplicate) ACK is received
    fn on_ack(&mut self, ack: u32, newly_acked: u32) -> Vec<CongestionAction>;
    
    /// Called when a duplicate ACK is received
    fn on_dup_ack(&mut self, ack: u32) -> Vec<CongestionAction>;
    
    /// Called when retransmission timeout occurs
    fn on_timeout(&mut self) -> Vec<CongestionAction>;
    
    /// Called when entering fast retransmit (3 duplicate ACKs)
    fn on_fast_retransmit(&mut self, recover: u32) -> Vec<CongestionAction>;
    
    /// Get current congestion window
    fn cwnd(&self) -> u32;
    
    /// Get current slow start threshold
    fn ssthresh(&self) -> u32;
    
    /// Get current state
    fn state(&self) -> CongestionState;
    
    /// Get algorithm name
    fn name(&self) -> &str;
}

/// Common congestion control parameters
#[derive(Debug, Clone, Copy)]
pub struct CongestionParams {
    pub cwnd: u32,
    pub ssthresh: u32,
    pub mss: u32,
    pub state: CongestionState,
    
    // For tracking
    pub dup_ack_count: u32,
    pub last_ack: u32,
    
    // For NewReno
    pub recover: u32,  // Recovery point (highest seq when entering fast recovery)
}

impl CongestionParams {
    pub fn new(mss: u32, initial_cwnd: u32) -> Self {
        Self {
            cwnd: initial_cwnd,
            ssthresh: u32::MAX,  // Start with no threshold
            mss,
            state: CongestionState::SlowStart,
            dup_ack_count: 0,
            last_ack: 0,
            recover: 0,
        }
    }
}
```

### TCP Tahoe Implementation

```rust
// filepath: /home/nazr/Desktop/projects/tcp/src/congestion_control.rs

// ...existing code...

/// TCP Tahoe congestion control
#[derive(Debug, Clone)]
pub struct TcpTahoe {
    params: CongestionParams,
}

impl TcpTahoe {
    pub fn new(mss: u32) -> Self {
        Self {
            params: CongestionParams::new(mss, mss),
        }
    }
}

impl CongestionControl for TcpTahoe {
    fn on_ack(&mut self, ack: u32, _newly_acked: u32) -> Vec<CongestionAction> {
        let mut actions = Vec::new();
        
        if ack <= self.params.last_ack {
            // Duplicate ACK - handle separately
            return vec![];
        }
        
        // New ACK - reset dup counter
        self.params.dup_ack_count = 0;
        self.params.last_ack = ack;
        
        // Update cwnd based on state
        match self.params.state {
            CongestionState::SlowStart => {
                // Exponential growth: cwnd += MSS
                self.params.cwnd += self.params.mss;
                
                // Check if we've reached ssthresh
                if self.params.cwnd >= self.params.ssthresh {
                    self.params.state = CongestionState::CongestionAvoidance;
                }
            }
            CongestionState::CongestionAvoidance => {
                // Linear growth: cwnd += MSS²/cwnd
                let increment = (self.params.mss * self.params.mss) / self.params.cwnd;
                self.params.cwnd += increment.max(1);
            }
            _ => {}
        }
        
        actions.push(CongestionAction::UpdateCwnd(self.params.cwnd));
        actions
    }
    
    fn on_dup_ack(&mut self, _ack: u32) -> Vec<CongestionAction> {
        self.params.dup_ack_count += 1;
        
        if self.params.dup_ack_count == 3 {
            // Triple duplicate ACK - treat as loss
            return self.on_fast_retransmit(0);
        }
        
        vec![]
    }
    
    fn on_timeout(&mut self) -> Vec<CongestionAction> {
        let mut actions = Vec::new();
        
        // Set ssthresh to max(FlightSize/2, 2*MSS)
        self.params.ssthresh = self.params.cwnd.max(2 * self.params.mss) / 2;
        
        // Reset cwnd to 1 MSS
        self.params.cwnd = self.params.mss;
        
        // Back to slow start
        self.params.state = CongestionState::SlowStart;
        self.params.dup_ack_count = 0;
        
        actions.push(CongestionAction::UpdateSsthresh(self.params.ssthresh));
        actions.push(CongestionAction::UpdateCwnd(self.params.cwnd));
        
        actions
    }
    
    fn on_fast_retransmit(&mut self, _recover: u32) -> Vec<CongestionAction> {
        // Tahoe treats 3 dup ACKs same as timeout
        self.on_timeout()
    }
    
    fn cwnd(&self) -> u32 { self.params.cwnd }
    fn ssthresh(&self) -> u32 { self.params.ssthresh }
    fn state(&self) -> CongestionState { self.params.state }
    fn name(&self) -> &str { "Tahoe" }
}
```

### TCP Reno Implementation

```rust
// filepath: /home/nazr/Desktop/projects/tcp/src/congestion_control.rs

// ...existing code...

/// TCP Reno congestion control
#[derive(Debug, Clone)]
pub struct TcpReno {
    params: CongestionParams,
}

impl TcpReno {
    pub fn new(mss: u32) -> Self {
        Self {
            params: CongestionParams::new(mss, mss),
        }
    }
}

impl CongestionControl for TcpReno {
    fn on_ack(&mut self, ack: u32, _newly_acked: u32) -> Vec<CongestionAction> {
        let mut actions = Vec::new();
        
        if ack <= self.params.last_ack {
            return vec![];
        }
        
        let is_new_ack = ack > self.params.last_ack;
        self.params.last_ack = ack;
        
        match self.params.state {
            CongestionState::SlowStart => {
                if is_new_ack {
                    self.params.cwnd += self.params.mss;
                    self.params.dup_ack_count = 0;
                    
                    if self.params.cwnd >= self.params.ssthresh {
                        self.params.state = CongestionState::CongestionAvoidance;
                    }
                }
            }
            CongestionState::CongestionAvoidance => {
                if is_new_ack {
                    let increment = (self.params.mss * self.params.mss) / self.params.cwnd;
                    self.params.cwnd += increment.max(1);
                    self.params.dup_ack_count = 0;
                }
            }
            CongestionState::FastRecovery => {
                if is_new_ack {
                    // Exit fast recovery
                    self.params.cwnd = self.params.ssthresh;
                    self.params.state = CongestionState::CongestionAvoidance;
                    self.params.dup_ack_count = 0;
                    
                    actions.push(CongestionAction::ExitFastRecovery);
                } else {
                    // Duplicate ACK in fast recovery - inflate window
                    self.params.cwnd += self.params.mss;
                }
            }
        }
        
        actions.push(CongestionAction::UpdateCwnd(self.params.cwnd));
        actions
    }
    
    fn on_dup_ack(&mut self, _ack: u32) -> Vec<CongestionAction> {
        self.params.dup_ack_count += 1;
        
        if self.params.dup_ack_count == 3 {
            return self.on_fast_retransmit(0);
        }
        
        if self.params.state == CongestionState::FastRecovery {
            self.params.cwnd += self.params.mss;
            return vec![CongestionAction::UpdateCwnd(self.params.cwnd)];
        }
        
        vec![]
    }
    
    fn on_timeout(&mut self) -> Vec<CongestionAction> {
        let mut actions = Vec::new();
        
        self.params.ssthresh = self.params.cwnd.max(2 * self.params.mss) / 2;
        self.params.cwnd = self.params.mss;
        self.params.state = CongestionState::SlowStart;
        self.params.dup_ack_count = 0;
        
        actions.push(CongestionAction::UpdateSsthresh(self.params.ssthresh));
        actions.push(CongestionAction::UpdateCwnd(self.params.cwnd));
        
        actions
    }
    
    fn on_fast_retransmit(&mut self, _recover: u32) -> Vec<CongestionAction> {
        let mut actions = Vec::new();
        
        // Enter fast recovery (Reno's innovation!)
        self.params.ssthresh = self.params.cwnd.max(2 * self.params.mss) / 2;
        self.params.cwnd = self.params.ssthresh + 3 * self.params.mss;
        self.params.state = CongestionState::FastRecovery;
        
        actions.push(CongestionAction::UpdateSsthresh(self.params.ssthresh));
        actions.push(CongestionAction::UpdateCwnd(self.params.cwnd));
        actions.push(CongestionAction::Retransmit(self.params.last_ack));
        
        actions
    }
    
    fn cwnd(&self) -> u32 { self.params.cwnd }
    fn ssthresh(&self) -> u32 { self.params.ssthresh }
    fn state(&self) -> CongestionState { self.params.state }
    fn name(&self) -> &str { "Reno" }
}
```

### TCP NewReno Implementation

```rust
// filepath: /home/nazr/Desktop/projects/tcp/src/congestion_control.rs

// ...existing code...

/// TCP NewReno congestion control
#[derive(Debug, Clone)]
pub struct TcpNewReno {
    params: CongestionParams,
}

impl TcpNewReno {
    pub fn new(mss: u32) -> Self {
        Self {
            params: CongestionParams::new(mss, mss),
        }
    }
}

impl CongestionControl for TcpNewReno {
    fn on_ack(&mut self, ack: u32, newly_acked: u32) -> Vec<CongestionAction> {
        let mut actions = Vec::new();
        
        if ack <= self.params.last_ack {
            return vec![];
        }
        
        let is_new_ack = ack > self.params.last_ack;
        let old_ack = self.params.last_ack;
        self.params.last_ack = ack;
        
        match self.params.state {
            CongestionState::SlowStart => {
                if is_new_ack {
                    self.params.cwnd += self.params.mss;
                    self.params.dup_ack_count = 0;
                    
                    if self.params.cwnd >= self.params.ssthresh {
                        self.params.state = CongestionState::CongestionAvoidance;
                    }
                }
            }
            CongestionState::CongestionAvoidance => {
                if is_new_ack {
                    let increment = (self.params.mss * self.params.mss) / self.params.cwnd;
                    self.params.cwnd += increment.max(1);
                    self.params.dup_ack_count = 0;
                }
            }
            CongestionState::FastRecovery => {
                if ack >= self.params.recover {
                    // FULL ACK - exit fast recovery
                    self.params.cwnd = self.params.ssthresh;
                    self.params.state = CongestionState::CongestionAvoidance;
                    self.params.dup_ack_count = 0;
                    
                    actions.push(CongestionAction::ExitFastRecovery);
                } else {
                    // PARTIAL ACK - more losses!
                    let amount_acked = ack - old_ack;
                    
                    // Deflate by amount ACKed, inflate by 1 MSS
                    self.params.cwnd = self.params.cwnd
                        .saturating_sub(amount_acked) + self.params.mss;
                    
                    // Retransmit next unACKed segment
                    actions.push(CongestionAction::Retransmit(ack));
                    
                    // Stay in fast recovery
                }
            }
        }
        
        actions.push(CongestionAction::UpdateCwnd(self.params.cwnd));
        actions
    }
    
    fn on_dup_ack(&mut self, _ack: u32) -> Vec<CongestionAction> {
        self.params.dup_ack_count += 1;
        
        if self.params.dup_ack_count == 3 {
            return self.on_fast_retransmit(0);
        }
        
        if self.params.state == CongestionState::FastRecovery {
            self.params.cwnd += self.params.mss;
            return vec![CongestionAction::UpdateCwnd(self.params.cwnd)];
        }
        
        vec![]
    }
    
    fn on_timeout(&mut self) -> Vec<CongestionAction> {
        let mut actions = Vec::new();
        
        self.params.ssthresh = self.params.cwnd.max(2 * self.params.mss) / 2;
        self.params.cwnd = self.params.mss;
        self.params.state = CongestionState::SlowStart;
        self.params.dup_ack_count = 0;
        
        actions.push(CongestionAction::UpdateSsthresh(self.params.ssthresh));
        actions.push(CongestionAction::UpdateCwnd(self.params.cwnd));
        
        actions
    }
    
    fn on_fast_retransmit(&mut self, recover: u32) -> Vec<CongestionAction> {
        let mut actions = Vec::new();
        
        // Set recovery point (NewReno's innovation!)
        self.params.recover = recover;
        
        self.params.ssthresh = self.params.cwnd.max(2 * self.params.mss) / 2;
        self.params.cwnd = self.params.ssthresh + 3 * self.params.mss;
        self.params.state = CongestionState::FastRecovery;
        
        actions.push(CongestionAction::UpdateSsthresh(self.params.ssthresh));
        actions.push(CongestionAction::UpdateCwnd(self.params.cwnd));
        actions.push(CongestionAction::Retransmit(self.params.last_ack));
        
        actions
    }
    
    fn cwnd(&self) -> u32 { self.params.cwnd }
    fn ssthresh(&self) -> u32 { self.params.ssthresh }
    fn state(&self) -> CongestionState { self.params.state }
    fn name(&self) -> &str { "NewReno" }
}
```

### Integration with TCB

```rust
// filepath: /home/nazr/Desktop/projects/tcp/src/tcb.rs

// ...existing code...

use crate::congestion_control::{CongestionControl, CongestionAction, TcpNewReno};

pub struct Tcb {
    // ...existing code...
    
    /// Congestion control algorithm (pluggable!)
    pub congestion_control: Box<dyn CongestionControl>,
}

impl Tcb {
    pub fn new(quad: Quad) -> Self {
        Self {
            // ...existing code...
            
            // Default to NewReno (can swap to Tahoe/Reno)
            congestion_control: Box::new(TcpNewReno::new(1460)),
        }
    }
    
    /// Process ACK with congestion control
    pub fn process_ack(&mut self, ack: u32, window: u16) -> bool {
        // ...existing ACK validation...
        
        let newly_acked = ack.wrapping_sub(self.snd.una);
        
        // Invoke congestion control
        let actions = if ack > self.snd.una {
            self.congestion_control.on_ack(ack, newly_acked)
        } else {
            self.congestion_control.on_dup_ack(ack)
        };
        
        // Execute actions
        for action in actions {
            match action {
                CongestionAction::UpdateCwnd(cwnd) => {
                    self.window.cwnd = cwnd;
                    println!("cwnd updated: {} ({})", cwnd, self.congestion_control.name());
                }
                CongestionAction::UpdateSsthresh(ssthresh) => {
                    self.window.ssthresh = ssthresh;
                    println!("ssthresh updated: {}", ssthresh);
                }
                CongestionAction::Retransmit(seq) => {
                    println!("Retransmit triggered for SEQ={}", seq);
                    // Trigger retransmission
                }
                CongestionAction::ExitFastRecovery => {
                    println!("Exiting fast recovery");
                }
            }
        }
        
        // ...existing code...
        
        true
    }
    
    /// Handle timeout with congestion control
    fn handle_timeout(&mut self) {
        let actions = self.congestion_control.on_timeout();
        
        for action in actions {
            match action {
                CongestionAction::UpdateCwnd(cwnd) => {
                    self.window.cwnd = cwnd;
                }
                CongestionAction::UpdateSsthresh(ssthresh) => {
                    self.window.ssthresh = ssthresh;
                }
                _ => {}
            }
        }
    }
}
```

---

## Real-World Examples

### Example 1: Web Page Load (Tahoe vs NewReno)

```
Scenario: Loading 500 KB web page
Network: 10 Mbps, 50ms RTT, 1% packet loss

Tahoe Performance:
────────────────────────────────────────────────────
Time    Event                           cwnd
0ms     Start                           1 MSS
50ms    ACK (slow start)                2 MSS
100ms   ACK (slow start)                4 MSS
150ms   ACK (slow start)                8 MSS
...
500ms   Reached ssthresh=65535          128 MSS
550ms   Congestion avoidance            129 MSS
600ms   LOSS! (1% probability)          1 MSS ← Reset!
650ms   Slow start again                2 MSS
...
1200ms  Page loaded

Total time: 1.2 seconds


NewReno Performance:
────────────────────────────────────────────────────
Time    Event                           cwnd
0ms     Start                           1 MSS
50ms    ACK (slow start)                2 MSS
100ms   ACK (slow start)                4 MSS
...
500ms   Reached ssthresh=65535          128 MSS
550ms   Congestion avoidance            129 MSS
600ms   LOSS! 3 dup ACKs                129→67 MSS ← Fast rec!
650ms   Exit fast recovery              64 MSS
700ms   Congestion avoidance            65 MSS
...
900ms   Page loaded

Total time: 0.9 seconds (25% faster!)
```

### Example 2: Video Streaming

```
Scenario: 4K video stream (25 Mbps required)
Network: 50 Mbps capacity, variable congestion

Without Congestion Control:
────────────────────────────────────────────────────
Sender transmits at 25 Mbps constantly
Network sometimes congested (other users)
Packet loss: 10-20%
Video buffering: Frequent
Quality: Poor


With NewReno:
────────────────────────────────────────────────────
cwnd adapts to network conditions:
- Low congestion: cwnd grows, high throughput
- Congestion detected: cwnd reduces, less loss
- Fast recovery: Quick adaptation to transient loss

Result:
- Packet loss: 0.5-1%
- Video buffering: Rare
- Quality: Excellent
- Fair sharing with other users
```

---

## Key Takeaways

### 🎯 Core Principles

1. **Congestion control protects the network** - Not just the connection
2. **Probe for bandwidth** - Increase until loss, then back off
3. **React to signals** - Packet loss indicates congestion
4. **Two phases** - Exponential slow start, then linear growth
5. **Different losses need different responses** - Timeout vs duplicate ACKs

### 🔧 Implementation Checklist

```
✓ Implement pluggable CongestionControl trait
✓ Support Tahoe, Reno, and NewReno
✓ Track cwnd and ssthresh
✓ Implement slow start (exponential growth)
✓ Implement congestion avoidance (linear growth)
✓ Detect 3 duplicate ACKs
✓ Implement fast retransmit
✓ Implement fast recovery (Reno/NewReno)
✓ Handle partial ACKs correctly (NewReno)
✓ Reset on timeout
✓ Measure and update RTT
```

### 📊 When to Use Which Algorithm

| Scenario | Best Algorithm | Why |
|----------|----------------|-----|
| **High loss rate** | Tahoe | Simple, conservative |
| **Single packet losses** | Reno | Fast recovery |
| **Burst losses** | NewReno | Partial ACK handling |
| **Modern networks** | CUBIC/BBR | Beyond this guide |
| **Satellite/high BDP** | CUBIC/BBR | Better for high delay |
| **Educational** | Tahoe | Simplest to understand |

---

## Further Reading

- **RFC 5681** - TCP Congestion Control ⭐ PRIMARY
- **RFC 2001** - TCP Slow Start, Congestion Avoidance, Fast Retransmit, and Fast Recovery Algorithms (obsoleted by 5681)
- **RFC 2582** - The NewReno Modification to TCP's Fast Recovery Algorithm
- **RFC 6582** - The NewReno Modification to TCP's Fast Recovery Algorithm (updates 2582)
- **RFC 6298** - Computing TCP's Retransmission Timer
- **"Congestion Avoidance and Control"** - Van Jacobson, 1988 (the paper that started it all!)

---

## Conclusion

Congestion control is TCP's **self-regulation mechanism** - the Internet's immune system. Without it, the Internet would have collapsed in the 1980s and never recovered.

The evolution from Tahoe → Reno → NewReno shows progressively smarter handling of packet loss:
- **Tahoe**: "Loss? Reset everything!" (simple but slow)
- **Reno**: "Duplicate ACKs? Network still works, fast recovery!" (major improvement)
- **NewReno**: "Partial ACK? More losses, stay vigilant!" (handles burst losses)

Understanding these algorithms deeply is essential for:
- **Debugging network performance** - Why is my transfer slow?
- **Implementing TCP correctly** - How should I react to loss?
- **Appreciating modern protocols** - CUBIC, BBR build on these foundations
- **Fair network sharing** - Congestion control is a cooperative protocol

Every video stream, every download, every cloud backup relies on congestion control to share the Internet fairly and efficiently. It's the invisible hand that keeps the Internet functioning.

**Master congestion control, master network efficiency! 🚀**

---

*Part of the 0xTCP project - Building TCP from scratch in Rust*
*Previous: [Window Scaling](./window_scaling_option.md) | Next: Selective Acknowledgment (SACK)*