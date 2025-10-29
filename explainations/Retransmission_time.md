# Retransmission Timer: TCP's Safety Net

## Introduction

Imagine sending a package through the mail and never receiving confirmation of delivery. How long should you wait before sending another copy? Wait too short, and you waste money on duplicates. Wait too long, and the recipient is left hanging. This is precisely the challenge TCP faces with the **Retransmission Timer**.

The retransmission timer is TCP's **insurance policy** against packet loss. It's the mechanism that transforms an unreliable network into a reliable communication channel. But setting this timer is a delicate balancing act - get it wrong, and you either flood the network with unnecessary retransmissions or leave connections hanging in limbo.

In this deep dive, we'll explore how TCP's retransmission timer works, how it adapts to network conditions, and why it's one of the most critical components of reliable data transfer.

---

## Table of Contents

1. [The Problem: Packet Loss Detection](#the-problem-packet-loss-detection)
2. [What is the Retransmission Timer?](#what-is-the-retransmission-timer)
3. [Retransmission Timeout (RTO)](#retransmission-timeout-rto)
4. [Measuring Round-Trip Time (RTT)](#measuring-round-trip-time-rtt)
5. [RTO Calculation (RFC 6298)](#rto-calculation-rfc-6298)
6. [Karn's Algorithm](#karns-algorithm)
7. [Exponential Backoff](#exponential-backoff)
8. [Retransmission Strategies](#retransmission-strategies)
9. [Implementation Details](#implementation-details)
10. [Real-World Examples](#real-world-examples)

---

## The Problem: Packet Loss Detection

### Why Do We Need Retransmission?

The Internet is fundamentally **unreliable**:

```
Packet Loss Causes:
┌─────────────────────────────────────────────┐
│ 1. Network Congestion                       │
│    Router buffers overflow → packets dropped│
│                                             │
│ 2. Transmission Errors                      │
│    Bit flips, cable issues, interference    │
│                                             │
│ 3. Routing Changes                          │
│    Packet takes wrong path and gets lost    │
│                                             │
│ 4. Hardware Failures                        │
│    Router crashes, link failures            │
└─────────────────────────────────────────────┘
```

### How Does TCP Detect Loss?

TCP has **two mechanisms** to detect packet loss:

```
Method 1: Duplicate ACKs (Fast Detection)
─────────────────────────────────────────────
Sender                          Receiver
  │                               │
  │─── SEQ=1000 ───────────────►│ ✓
  │─── SEQ=1100 ───────────────×│ LOST!
  │─── SEQ=1200 ───────────────►│ ✓ Out of order
  │                               │
  │◄── ACK=1100 ─────────────────│ "Need 1100"
  │◄── ACK=1100 ─────────────────│ Duplicate!
  │◄── ACK=1100 ─────────────────│ Duplicate!
  │                               │
  3 Duplicate ACKs → Fast Retransmit (immediate)


Method 2: Timeout (Slow Detection)
─────────────────────────────────────────────
Sender                          Receiver
  │                               │
  │─── SEQ=1000 ───────────────►│ ✓
  │                               │
  │◄── ACK=1100 ─────────────────│
  │                               │
  │─── SEQ=1100 ───────────────×│ LOST!
  │                               │
  │ ⏰ Wait RTO (1 second)        │
  │                               │
  │─── SEQ=1100 ───────────────►│ Retransmit!
  │                               │
  │◄── ACK=1200 ─────────────────│ ✓
```

Fast retransmit is preferred, but **timeouts are essential** for catching all loss scenarios.

---

## What is the Retransmission Timer?

### Definition

The **Retransmission Timer** is a countdown that starts when TCP sends a segment. If an acknowledgment doesn't arrive before the timer expires, TCP assumes the segment was lost and retransmits it.

### Key Properties

```
┌─────────────────────────────────────────────┐
│ Timer Lifecycle                             │
├─────────────────────────────────────────────┤
│ START: When segment is sent                 │
│ STOP:  When ACK is received                 │
│ EXPIRE: When RTO is reached                 │
│ ACTION: Retransmit segment                  │
└─────────────────────────────────────────────┘
```

### Visual Representation

```
Timeline:

T=0ms    Send SEQ=1000
         ├──────────────────► Start Timer (RTO = 1000ms)
         │
T=50ms   │ (packet traveling)
         │
T=100ms  │ Packet arrives at receiver
         │
T=150ms  │ ACK traveling back
         │
T=200ms  │◄──────────────── ACK=1100 received
         └─ STOP TIMER ✓
         
         RTT = 200ms (measured)
         

Timeout Scenario:

T=0ms    Send SEQ=1000
         ├──────────────────► Start Timer (RTO = 1000ms)
         │
T=50ms   │ ✗ Packet LOST!
         │
T=500ms  │ (still waiting...)
         │
T=1000ms └─ TIMEOUT! ⏰
         
         ├──────────────────► Retransmit SEQ=1000
         │                    Reset Timer (RTO = 2000ms)
         │
T=1100ms │ Retransmit arrives
         │
T=1200ms │◄──────────────── ACK=1100 received
         └─ STOP TIMER ✓
```

---

## Retransmission Timeout (RTO)

### What is RTO?

**RTO (Retransmission Timeout)** is the duration TCP waits before retransmitting. It must be:

```
Too Short:                      Too Long:
┌─────────────────┐            ┌─────────────────┐
│ • Spurious      │            │ • Slow recovery │
│   retransmits   │            │ • Poor latency  │
│ • Network       │            │ • User waits    │
│   congestion    │            │   forever       │
│ • Wasted        │            │ • Timeouts too  │
│   bandwidth     │            │   generous      │
└─────────────────┘            └─────────────────┘

                 ┌─────────────────┐
                 │   JUST RIGHT    │
                 ├─────────────────┤
                 │ RTO ≈ RTT + ε   │
                 │ where ε is a    │
                 │ safety margin   │
                 └─────────────────┘
```

### RTO Guidelines (RFC 6298)

```
Minimum RTO:  1 second
Maximum RTO:  60 seconds (typically)
Initial RTO:  1 second (before any RTT measurement)

Ideal RTO = SRTT + 4 × RTTVAR

Where:
- SRTT: Smoothed Round-Trip Time (average)
- RTTVAR: Round-Trip Time Variation (jitter)
```

---

## Measuring Round-Trip Time (RTT)

### What is RTT?

**RTT (Round-Trip Time)** is the time it takes for a segment to reach the receiver and for the acknowledgment to return.

```
Measuring RTT:

Client                                    Server
  │                                         │
T=0   │─── SEQ=1000 ─────────────────────►│
  │                                         │
T=25  │         (network delay)             │
  │                                         │
T=50  │                                     │ Received
  │                                         │ Process (1ms)
T=51  │                                     │
  │                                         │
T=76  │         (network delay)             │
  │                                         │
T=100 │◄── ACK=1100 ─────────────────────│
  │                                         │
  
RTT = 100ms (time from send to ACK)

Components:
- Forward propagation: 50ms
- Processing: 1ms
- Return propagation: 49ms
- Total: 100ms
```

### RTT Variability

Network delays aren't constant:

```
Sample RTT Measurements (same connection):

Measurement  RTT     Reason
─────────────────────────────────────────
1            100ms   Normal
2            150ms   Network congestion
3            95ms    Faster route
4            200ms   Router queue buildup
5            80ms    Direct path
6            120ms   Cross-traffic
7            300ms   Severe congestion
8            90ms    Back to normal

Average: 142ms
Variation: ±110ms

This is why we need SMOOTHING!
```

---

## RTO Calculation (RFC 6298)

### The Algorithm

TCP uses **Exponential Weighted Moving Average (EWMA)** to smooth RTT measurements:

```
Variables:
──────────────────────────────────────────
SRTT     Smoothed Round-Trip Time
RTTVAR   RTT Variation (jitter)
RTO      Retransmission Timeout
R'       Latest RTT measurement

Constants:
──────────────────────────────────────────
α = 1/8  Weight for SRTT update
β = 1/4  Weight for RTTVAR update
K = 4    Safety multiplier
G = 100  Clock granularity (ms)
```

### Step-by-Step Calculation

```
First Measurement (R' = 100ms):
────────────────────────────────────────────
SRTT    = R'          = 100ms
RTTVAR  = R' / 2      = 50ms
RTO     = SRTT + 4×RTTVAR = 100 + 200 = 300ms


Subsequent Measurements:
────────────────────────────────────────────
R' = 150ms (new measurement)

1. Calculate difference:
   diff = |SRTT - R'| = |100 - 150| = 50ms

2. Update RTTVAR:
   RTTVAR = (1-β)×RTTVAR + β×diff
   RTTVAR = 0.75×50 + 0.25×50
   RTTVAR = 37.5 + 12.5 = 50ms

3. Update SRTT:
   SRTT = (1-α)×SRTT + α×R'
   SRTT = 0.875×100 + 0.125×150
   SRTT = 87.5 + 18.75 = 106.25ms

4. Calculate RTO:
   RTO = SRTT + max(G, K×RTTVAR)
   RTO = 106.25 + max(100, 4×50)
   RTO = 106.25 + 200 = 306.25ms
   
5. Clamp RTO:
   RTO = clamp(RTO, 1000ms, 60000ms)
   RTO = 1000ms (minimum enforced)
```

### Why This Works

```
EWMA gives MORE weight to RECENT measurements:

Old measurements fade over time:
┌────────────────────────────────────────┐
│ Measurement Weight in SRTT             │
├────────────────────────────────────────┤
│ Most recent:    12.5% (1/8)            │
│ 1 ago:          10.9% (7/8 × 1/8)      │
│ 2 ago:           9.6% (49/64 × 1/8)    │
│ 3 ago:           8.4%                  │
│ 4 ago:           7.3%                  │
│ 5+ ago:         51.3% (combined)       │
└────────────────────────────────────────┘

Result: Adapts to changes but not too quickly
```

---

## Karn's Algorithm

### The Problem

When a segment is retransmitted, we can't tell which transmission was acknowledged:

```
Scenario: Ambiguous ACK
────────────────────────────────────────────

T=0     Send SEQ=1000 ─────────────────────►
        (Original)
        
T=1000  TIMEOUT! ⏰
        
T=1001  Send SEQ=1000 ─────────────────────►
        (Retransmit)
        
T=1100  ◄───────────────── ACK=1100 received

Question: Which transmission does this ACK?
- Original (RTT = 1100ms) ← WRONG if lost!
- Retransmit (RTT = 99ms) ← WRONG if delayed!

If we measure wrong RTT → Wrong RTO → More problems!
```

### Karn's Solution

**DON'T measure RTT for retransmitted segments!**

```
Karn's Algorithm:
────────────────────────────────────────────
1. When sending a segment:
   - Start timer
   - Mark as "original transmission"

2. If timeout occurs:
   - Retransmit
   - Mark as "retransmitted"
   - DON'T use for RTT measurement

3. When ACK arrives:
   - If segment was retransmitted → Ignore for RTT
   - If segment was original → Use for RTT

4. Only measure RTT from non-retransmitted segments
```

### Implementation

```rust
pub struct Segment {
    seq: u32,
    data: Vec<u8>,
    timestamp: Option<Instant>,
    retransmit_count: u32,  // ← Track retransmissions
}

fn process_ack(&mut self, ack: u32) {
    if let Some(seg) = self.retransmission_queue.front() {
        // Karn's Algorithm: Only measure RTT for
        // non-retransmitted segments
        if seg.retransmit_count == 0 {
            if let Some(sent_time) = seg.timestamp {
                let rtt = sent_time.elapsed().as_millis() as u32;
                self.update_rtt(rtt);  // ← Safe to use!
            }
        } else {
            // Retransmitted segment - DON'T measure RTT
            println!("Ignoring RTT measurement (retransmitted)");
        }
    }
}
```

---

## Exponential Backoff

### The Principle

After each timeout, **DOUBLE the RTO**:

```
Exponential Backoff Strategy:
────────────────────────────────────────────

Attempt  RTO       Multiplier
──────────────────────────────────────────
1        1s        × 1
2        2s        × 2
3        4s        × 4
4        8s        × 8
5        16s       × 16
6        32s       × 32
7        60s       × 64 (capped at 60s)
8        60s       (stay at max)
...
15       60s       (give up after 15 attempts)
```

### Why Exponential Backoff?

```
Reason 1: Network Congestion
────────────────────────────────────────────
If packets are lost due to congestion,
aggressive retransmits make it WORSE!

Without backoff:
T=0s    Send ────×
T=1s    Resend ───×  } All add to
T=2s    Resend ───×  } congestion!
T=3s    Resend ───×  }

With backoff:
T=0s    Send ────×
T=1s    Resend ───×
T=3s    Resend ───×   (congestion clearing)
T=7s    Resend ───✓   (network recovered)


Reason 2: Failure Detection
────────────────────────────────────────────
If receiver is dead/unreachable, we don't
want to spam forever. Backoff allows us to
detect persistent failures quickly.

Total time to give up:
1+2+4+8+16+32+60+...+60 ≈ 15 minutes
(after 15 attempts)
```

### Implementation

```rust
fn check_retransmission_timeout(&mut self) -> Vec<RetransmitAction> {
    for segment in self.retransmission_queue.iter_mut() {
        if now >= segment.retransmit_at {
            segment.retransmit_count += 1;
            
            // Exponential backoff (capped at 64x)
            let backoff = 2u32.pow(segment.retransmit_count.min(6));
            let new_rto = (self.timers.rto * backoff).min(60000);
            
            segment.retransmit_at = 
                now + Duration::from_millis(new_rto);
            
            println!("Attempt #{}: RTO={}ms", 
                segment.retransmit_count, new_rto);
            
            // Give up after 15 attempts
            if segment.retransmit_count >= 15 {
                return RetransmitAction::GiveUp;
            }
        }
    }
}
```

---

## Retransmission Strategies

### Strategy 1: Go-Back-N

Retransmit **everything** from the lost segment:

```
Sent:     [1000] [1100] [1200] [1300]
            ↓      ✗      ↓      ↓
Received: [1000]        [1200] [1300]

Timeout on 1100!

Retransmit: [1100] [1200] [1300]
             ↓      ↓      ↓
            ALL retransmitted (even if received)

Pros: Simple
Cons: Wasteful (retransmits already-received data)
```

### Strategy 2: Selective Retransmit (with SACK)

Retransmit **only** the lost segment:

```
Sent:     [1000] [1100] [1200] [1300]
            ↓      ✗      ↓      ↓
Received: [1000]        [1200] [1300]
                         └─────┴───► Buffered

Receiver sends: ACK=1100, SACK:1200-1399

Timeout on 1100!

Retransmit: [1100] only
             ↓
            [1100] fills the gap
            
Deliver: [1000][1100][1200][1300] to application

Pros: Efficient
Cons: Requires SACK support
```

### Strategy 3: Timer per Segment vs Single Timer

```
Per-Segment Timer (Complex):
────────────────────────────────────────────
Segment   Timer
[1000]    RTO=1s  ────► Expire T+1s
[1100]    RTO=1s  ────► Expire T+1s
[1200]    RTO=1s  ────► Expire T+1s

Pros: Precise timeout for each segment
Cons: Overhead (many timers)


Single Timer (Simple - TCP uses this):
────────────────────────────────────────────
Oldest unACKed segment controls timer:

Send [1000] → Start timer (RTO=1s)
Send [1100] → Timer still running
Send [1200] → Timer still running

ACK 1100 arrives:
  - Oldest unACKed is now [1200]
  - Reset timer for [1200]

Pros: Simple, one timer
Cons: May delay detection of later losses
```

---

## Implementation Details

### Our TCP Implementation

#### 1. Timer Management

```rust
pub struct TcpTimers {
    /// Current RTO in milliseconds
    pub rto: u32,
    
    /// Smoothed RTT
    pub srtt: u32,
    
    /// RTT variation
    pub rttvar: u32,
    
    /// When to check next retransmission
    pub retransmit_timer: Option<Instant>,
    
    /// Consecutive timeouts (for backoff)
    pub consecutive_timeouts: u32,
}
```

#### 2. Starting the Timer

```rust
pub fn queue_for_retransmission(
    &mut self, 
    seq: u32, 
    flags: u8, 
    data: Vec<u8>
) {
    let now = Instant::now();
    let retransmit_at = now + Duration::from_millis(self.timers.rto);
    
    let segment = Segment {
        seq,
        data,
        timestamp: Some(now),
        retransmit_count: 0,
        retransmit_at: Some(retransmit_at),
    };
    
    self.retransmission_queue.push_back(segment);
    
    // Set timer to earliest retransmission
    if self.timers.retransmit_timer.is_none() {
        self.timers.retransmit_timer = Some(retransmit_at);
    }
}
```

#### 3. Checking for Timeouts

```rust
pub fn check_retransmission_timeout(&mut self) -> Vec<RetransmitAction> {
    let now = Instant::now();
    let mut actions = Vec::new();
    
    // Check if timer expired
    if let Some(timer) = self.timers.retransmit_timer {
        if now < timer {
            return actions; // Not yet
        }
    }
    
    // Find expired segments
    for segment in self.retransmission_queue.iter_mut() {
        if let Some(retransmit_at) = segment.retransmit_at {
            if now >= retransmit_at {
                // TIMEOUT!
                segment.retransmit_count += 1;
                
                // Exponential backoff
                let backoff = 2u32.pow(segment.retransmit_count.min(6));
                let new_rto = (self.timers.rto * backoff).min(60000);
                
                segment.retransmit_at = 
                    Some(now + Duration::from_millis(new_rto));
                
                if segment.retransmit_count >= 15 {
                    actions.push(RetransmitAction::GiveUp {
                        seq: segment.seq,
                        reason: "Max retries exceeded".into(),
                    });
                } else {
                    actions.push(RetransmitAction::Retransmit {
                        seq: segment.seq,
                        data: segment.data.clone(),
                        attempt: segment.retransmit_count,
                    });
                }
            }
        }
    }
    
    actions
}
```

#### 4. Stopping the Timer (on ACK)

```rust
pub fn process_ack(&mut self, ack: u32) -> bool {
    // Remove acknowledged segments
    self.retransmission_queue.retain(|seg| {
        seg.seq + seg.data.len() as u32 > ack
    });
    
    if self.retransmission_queue.is_empty() {
        // All data ACKed - stop timer
        self.timers.retransmit_timer = None;
    } else {
        // Reset timer for remaining segments
        let now = Instant::now();
        let next_timeout = now + Duration::from_millis(self.timers.rto);
        self.timers.retransmit_timer = Some(next_timeout);
    }
    
    true
}
```

#### 5. RTT Measurement (with Karn's Algorithm)

```rust
pub fn update_rtt(&mut self, measured_rtt: u32) {
    if self.timers.srtt == 0 {
        // First measurement (RFC 6298)
        self.timers.srtt = measured_rtt;
        self.timers.rttvar = measured_rtt / 2;
        self.timers.rto = self.timers.srtt + 4 * self.timers.rttvar;
    } else {
        // Subsequent measurements
        let diff = if self.timers.srtt > measured_rtt {
            self.timers.srtt - measured_rtt
        } else {
            measured_rtt - self.timers.srtt
        };
        
        // RTTVAR = (1 - β) × RTTVAR + β × |SRTT - R'|
        self.timers.rttvar = (3 * self.timers.rttvar + diff) / 4;
        
        // SRTT = (1 - α) × SRTT + α × R'
        self.timers.srtt = (7 * self.timers.srtt + measured_rtt) / 8;
        
        // RTO = SRTT + max(G, K × RTTVAR)
        self.timers.rto = self.timers.srtt + 4 * self.timers.rttvar.max(25);
    }
    
    // Clamp between 1s and 60s
    self.timers.rto = self.timers.rto.clamp(1000, 60000);
    
    println!("RTT updated: measured={}ms, SRTT={}ms, RTO={}ms",
        measured_rtt, self.timers.srtt, self.timers.rto);
}
```

---

## Real-World Examples

### Example 1: Perfect Network (No Loss)

```
Connection over fast local network (RTT = 10ms)

T=0ms    Send SEQ=1000 ──────────────────►
T=5ms    (packet arrives)
T=6ms    ◄────────────────── ACK=1100
T=6ms    ✓ Measure RTT = 6ms

RTO Calculation:
Initial:  SRTT=6ms, RTTVAR=3ms, RTO=1000ms (min)
After 10: SRTT=6ms, RTTVAR=1ms, RTO=1000ms (min)

Result: RTO stays at minimum (1s) because network
        is so fast. Timer never expires!
```

### Example 2: Congested Network

```
Connection over congested network

T=0ms     Send SEQ=1000 ──────────────────►
T=500ms   (stuck in queue)
T=600ms   ◄────────────────── ACK=1100
T=600ms   ✓ Measure RTT = 600ms

RTO Calculation:
SRTT = 600ms
RTTVAR = 300ms
RTO = 600 + 4×300 = 1800ms

T=600ms   Send SEQ=1100 ──────────────────►
T=2400ms  TIMEOUT! (no ACK after 1800ms) ⏰

T=2400ms  Retransmit SEQ=1100 (attempt #1)
          New RTO = 1800 × 2 = 3600ms
          
T=3000ms  ◄────────────────── ACK=1200
          ✓ Success on retry!
          
Don't measure this RTT (retransmitted!)
```

### Example 3: Satellite Connection (High Latency)

```
Connection via satellite (RTT = 600ms)

T=0ms     Send SEQ=1000 ──────────────────►
T=300ms   (uplink to satellite)
T=600ms   (downlink to ground)
T=600ms   ◄────────────────── ACK=1100
T=600ms   ✓ Measure RTT = 600ms

RTO Calculation:
After several measurements:
SRTT = 600ms
RTTVAR = 50ms (low variation)
RTO = 600 + 4×50 = 800ms

This is good! RTO adapts to high latency without
spurious timeouts.
```

### Example 4: Mobile Network (Variable Latency)

```
Connection over 4G with handoffs

Measurement  RTT     Reason
────────────────────────────────────────
1            50ms    Good signal
2            100ms   Moving
3            200ms   Weak signal
4            80ms    Better signal
5            500ms   Tower handoff!
6            60ms    New tower

RTO Evolution:
After meas 1: SRTT=50ms,  RTTVAR=25ms,  RTO=1000ms
After meas 3: SRTT=85ms,  RTTVAR=45ms,  RTO=1000ms
After meas 5: SRTT=158ms, RTTVAR=148ms, RTO=1000ms
After meas 6: SRTT=146ms, RTTVAR=126ms, RTO=1000ms

RTTVAR captures the variation, keeping RTO safe!
```

---

## Performance Implications

### Impact of RTO on Throughput

```
Scenario: 1% packet loss, 100ms RTT

With Optimal RTO (200ms):
────────────────────────────────────────
Packet lost → Wait 200ms → Retransmit
Recovery time: ~200ms per loss
Throughput: ~98% of maximum

With Too-Short RTO (50ms):
────────────────────────────────────────
Packet delayed (not lost) → Timeout at 50ms
Spurious retransmit → Wasted bandwidth
Throughput: ~70% (congestion from duplicates)

With Too-Long RTO (2000ms):
────────────────────────────────────────
Packet lost → Wait 2000ms → Retransmit
Recovery time: ~2000ms per loss
Throughput: ~60% (idle time waiting)
```

### Timeout vs Fast Retransmit

```
Fast Retransmit Performance:
────────────────────────────────────────
Loss detected: 3 duplicate ACKs (3×RTT)
Recovery time: ~300ms for 100ms RTT
Preferred!

Timeout Performance:
────────────────────────────────────────
Loss detected: RTO expiration
Recovery time: 1000ms+ (minimum RTO)
Slower, but catches all losses
```

---

## Key Takeaways

### 🎯 Core Principles

1. **RTO must adapt to network conditions** - Static timeouts fail
2. **Smooth RTT measurements** - Don't overreact to variations
3. **Karn's Algorithm is essential** - Never measure RTT on retransmits
4. **Exponential backoff prevents congestion collapse** - Double RTO on timeout
5. **Minimum 1 second RTO** - Prevents spurious retransmissions

### 🔧 Implementation Checklist

```
✓ Measure RTT for every non-retransmitted ACK
✓ Update SRTT and RTTVAR using EWMA
✓ Calculate RTO = SRTT + 4×RTTVAR
✓ Clamp RTO between 1s and 60s
✓ Start timer when sending unACKed data
✓ Stop timer when all data is ACKed
✓ Double RTO on each timeout (exponential backoff)
✓ Give up after 15 retransmission attempts
✓ Don't measure RTT from retransmitted segments
```

### 📊 Performance Metrics

| Metric | Good | Bad |
|--------|------|-----|
| **Spurious Retransmits** | < 1% | > 5% |
| **RTO / RTT Ratio** | 2-4× | > 10× or < 1.5× |
| **Timeout Recovery Time** | ~RTO | >> RTO |
| **RTT Variance** | < 50ms | > 200ms |

---

## Further Reading

- **RFC 6298** - Computing TCP's Retransmission Timer ⭐ MUST READ
- **RFC 2988** - Computing TCP's RTO (obsoleted by 6298)
- **RFC 2018** - TCP Selective Acknowledgment Options
- **RFC 5681** - TCP Congestion Control
- **Phil Karn's original paper** - "Improving Round-Trip Time Estimates in Reliable Transport Protocols" (1987)

---

## Conclusion

The retransmission timer is TCP's **safety net** - the last line of defense against packet loss. While fast retransmit handles common losses quickly, the timeout mechanism ensures **no packet is ever permanently lost**.

The elegance of TCP's RTO algorithm lies in its **adaptability**. From blazing-fast local networks to sluggish satellite links, from stable fiber to chaotic mobile networks - TCP's retransmission timer adapts to them all. By measuring, smoothing, and carefully adjusting, it finds the sweet spot between aggressive recovery and patient waiting.

Understanding the retransmission timer deeply is essential for:
- **Diagnosing slow connections** - Is it the network or bad RTO?
- **Tuning TCP performance** - Adjust for specific scenarios
- **Implementing TCP correctly** - Get the timer wrong, and everything breaks

Every HTTP request, every video stream, every SSH session relies on this timer ticking away, ready to catch any packet that dares to go missing.

**Master the timer, master reliability! 🎯**

---

*Part of the 0xTCP project - Building TCP from scratch in Rust*
*[Sequence Numbers](./sequence_and_ack_number_tracking.md) | [Data Transmission](./data_transmission_and_ack_handling.md)*
