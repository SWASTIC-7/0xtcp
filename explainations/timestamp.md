# TCP Timestamp Option (RFC 7323): Precision Timing for Modern Networks

## Introduction

Imagine you're a detective trying to solve a mystery: "Was this package delivered late, or did my acknowledgment get lost?" Without timestamps, TCP is flying blind. The **Timestamp Option** gives TCP eyes on the clock, enabling it to:

1. **Measure Round-Trip Time (RTT) accurately** - Essential for adaptive retransmission
2. **Protect Against Wrapped Sequence Numbers (PAWS)** - Critical for high-speed networks
3. **Detect spurious retransmissions** - Avoid unnecessary congestion control

RFC 7323 (updating RFC 1323) defines this elegant mechanism that adds just **10 bytes** to the TCP header but provides invaluable timing information. Every modern TCP stack uses timestamps - they're not optional in today's Internet.

---

## Table of Contents

1. [The Timing Problem](#the-timing-problem)
2. [What is the Timestamp Option?](#what-is-the-timestamp-option)
3. [Timestamp Format & Fields](#timestamp-format--fields)
4. [How Timestamps Work](#how-timestamps-work)
5. [RTT Measurement with Timestamps](#rtt-measurement-with-timestamps)
6. [PAWS: Protection Against Wrapped Sequences](#paws-protection-against-wrapped-sequences)
7. [Eifel Detection Algorithm](#eifel-detection-algorithm)
8. [Implementation Deep Dive](#implementation-deep-dive)
9. [Real-World Examples](#real-world-examples)
10. [Performance Impact](#performance-impact)

---

## The Timing Problem

### Problem 1: Inaccurate RTT Measurement

Without timestamps, TCP faces Karn's Algorithm dilemma:

```
Scenario: Segment retransmitted

Sender                          Network                         Receiver
  │                               │                               │
  │─── SEQ=1000 (original) ──────►│                               │
  │    Timestamp: T0              │ ⏰ Delayed...                 │
  │                               │                               │
  ⏰ RTO expires                   │                               │
  │                               │                               │
  │─── SEQ=1000 (retrans) ────────┼──────────────────────────────►│
  │    Timestamp: T1              │                     Fast path │
  │                               │                               │
  │◄──────────────────────────────────────────── ACK=1100 ────────│
  │                               │                               │
  
Question: Which transmission was ACKed?

Without Timestamps:
────────────────────────────────────────────────────
If we measure RTT = ACK_time - T0:
  → RTT seems HUGE (includes retransmit delay)
  → RTO increases unnecessarily ❌

If we measure RTT = ACK_time - T1:
  → RTT seems TINY (fast retransmit path)
  → RTO decreases too much ❌

Karn's Algorithm solution: Don't measure RTT for retransmitted segments!
Problem: Fewer RTT samples = less accurate RTO ❌


With Timestamps:
────────────────────────────────────────────────────
ACK includes: TSval=T1, TSecr=(whatever we sent)
Sender knows: "This ACKs the retransmission (T1), not original (T0)"
→ Accurate RTT measurement! ✓
→ Always measure RTT, even after retransmission! ✓
```

### Problem 2: Sequence Number Wraparound

The 32-bit sequence number **wraps around** quickly on high-speed networks:

```
High-Speed Network Scenario:
────────────────────────────────────────────────────

Link speed: 100 Gbps
Sequence space: 32 bits = 4,294,967,296 bytes

Wrap time = 4 GB / (100 Gbps / 8)
          = 4,294,967,296 / 12,500,000,000
          = 0.34 seconds

Every 0.34 seconds, sequence numbers wrap around!


Dangerous Scenario:
────────────────────────────────────────────────────

T=0.0s   Send: SEQ=4,294,960,000 (near max)
        Data: 1000 bytes

T=0.1s   Send: SEQ=100 (wrapped around!)
        Data: 1000 bytes

T=5.0s   Old packet SEQ=4,294,960,000 arrives (delayed 5 seconds!)
        Receiver thinks: "Is this new data or old garbage?"


Without Timestamps:
────────────────────────────────────────────────────
Receiver compares SEQ numbers: 4,294,967,000 vs 100
With wraparound: 4,294,967,000 > 100 (looks like future data!)
→ Accepts old packet as new data ❌
→ Data corruption! ❌


With Timestamps (PAWS):
────────────────────────────────────────────────────
Old packet: TSval=1000 (from T=0s)
Current time: TSval=50000 (T=5s)

Receiver checks: Is 1000 >= 50000? No!
→ Packet is too old, discard! ✓
→ Protected against wrapped sequences ✓
```

### Problem 3: Spurious Retransmissions

```
Scenario: Premature timeout

Sender                          Network                         Receiver
  │                               │                               │
  │─── SEQ=1000, TSval=100 ───────►│                               │
  │                               │ ⏰ Slow network path...       │
  │                               │                               │
  ⏰ RTO expires (too early!)     │                               │
  │                               │                               │
  │─── SEQ=1000, TSval=200 ────────┼──────────────────────────────►│
  │    (retransmission)           │                     Fast path │
  │                               │                               │
  │◄──────────────────────────────────── ACK=1100, TSecr=200 ─────│
  │                               │        "Got retransmission"   │
  │                               │                               │
  │                               │ Original packet arrives!      │
  │                               │─────────────────────────────►│
  │                               │                               │
  │◄──────────────────────────────────── ACK=1100, TSecr=100 ─────│
  │                               │     "Got original too!"       │

Eifel Detection:
────────────────────────────────────────────────────
First ACK: TSecr=200 (retransmission)
Second ACK: TSecr=100 (original!)

Sender sees: TSecr=100 < 200 → Original packet WAS delivered!
Conclusion: Retransmission was spurious! ❌
Action: UNDO cwnd reduction ✓
        INCREASE RTO (was too aggressive) ✓
```

---

## What is the Timestamp Option?

### Definition

The **Timestamp Option (TSopt)** is a TCP option that includes two 32-bit timestamp values in every TCP segment:

1. **TSval (Timestamp Value)**: Sender's current timestamp
2. **TSecr (Timestamp Echo Reply)**: Echo of most recent timestamp received from peer

### Key Properties

```
┌─────────────────────────────────────────────┐
│ Timestamp Option Properties                 │
├─────────────────────────────────────────────┤
│ TCP Option Kind:    8                       │
│ Option Length:      10 bytes                │
│ TSval:              4 bytes (32-bit)        │
│ TSecr:              4 bytes (32-bit)        │
│ Negotiated:         During handshake        │
│ Mandatory:          After negotiation       │
│ Clock:              Arbitrary units         │
│ Resolution:         1-1000 ms typical       │
└─────────────────────────────────────────────┘
```

### Visual Format

```
Timestamp Option Structure:
────────────────────────────────────────────────────
┌──────────┬──────────┬────────────────┬────────────────┐
│ Kind: 8  │ Length:10│  TSval (4B)    │  TSecr (4B)    │
├──────────┼──────────┼────────────────┼────────────────┤
│  1 byte  │  1 byte  │   32 bits      │   32 bits      │
└──────────┴──────────┴────────────────┴────────────────┘

Example:
┌────┬────┬─────────────┬─────────────┐
│ 08 │ 0A │  00 00 27 10│  00 00 12 34│
└────┴────┴─────────────┴─────────────┘
  Kind Len    TSval=10000   TSecr=4660
```

---

## Timestamp Format & Fields

### TSval (Timestamp Value)

```
What is TSval?
────────────────────────────────────────────────────
The sender's current timestamp when sending this segment.

Format: 32-bit unsigned integer
Unit: Implementation-defined (milliseconds common)
Wraps: After 2^32 clock ticks (~49 days at 1ms resolution)

Examples:
- Boot time: TSval=0
- 10 seconds later: TSval=10000 (if 1ms clock)
- 24 hours later: TSval=86400000
- Wraps at: TSval=4,294,967,295 → 0
```

### TSecr (Timestamp Echo Reply)

```
What is TSecr?
────────────────────────────────────────────────────
Echo of the most recent TSval received from the peer.

Purpose: Allows sender to calculate RTT
Value: Copy of TSval from most recent acceptable segment

Special cases:
- SYN segment: TSecr=0 (no previous segment)
- After receiving TSval=12345: Always echo TSecr=12345
- Even in retransmissions: Echo most recent received TSval
```

### Clock Requirements

```
RFC 7323 Clock Requirements:
────────────────────────────────────────────────────

1. Monotonically Increasing:
   TSval(t+1) > TSval(t) for all t
   Never goes backward!

2. Resolution:
   Between 1 ms and 1 second per tick
   Recommendation: 1-10 ms

3. Wraps Gracefully:
   After 2^32 - 1, wraps to 0
   Must handle wraparound correctly

4. Not Wall Clock:
   Doesn't need to be actual time
   Can be arbitrary tick counter

5. Independent Per Connection:
   Each connection can use different clock
   (But usually share system-wide clock)


Example Implementations:
────────────────────────────────────────────────────

Linux: Uses jiffies (typically 1ms)
FreeBSD: Uses system ticks (1-10ms)
Windows: Uses GetTickCount() (10-16ms)

Your implementation:
Use: Instant::now().elapsed() since program start
Resolution: Microseconds (very precise!)
Store: Milliseconds (divide by 1000)
```

---

## How Timestamps Work

### Three-Way Handshake with Timestamps

```
Client and Server negotiate Timestamp Option:

┌─────────────────────────────────────────────────────────┐
│ Step 1: Client SYN                                      │
└─────────────────────────────────────────────────────────┘

Client → Server:
┌────────────────────────────────────────┐
│ SYN, SEQ=1000                          │
│ Options:                               │
│   ├─ MSS: 1460                         │
│   ├─ TSopt: TSval=100, TSecr=0         │
│   │         └─────┬─────┘              │
│   │           Client's clock           │
│   └─ SACK Permitted                    │
└────────────────────────────────────────┘

Client says: "I support timestamps. My clock is at 100."


┌─────────────────────────────────────────────────────────┐
│ Step 2: Server SYN-ACK                                  │
└─────────────────────────────────────────────────────────┘

Server → Client:
┌────────────────────────────────────────┐
│ SYN-ACK, SEQ=5000, ACK=1001            │
│ Options:                               │
│   ├─ MSS: 1460                         │
│   ├─ TSopt: TSval=5000, TSecr=100      │
│   │         └────┬────┘ └────┬─────┘  │
│   │         Server's  Echo of         │
│   │         clock     client's        │
│   └─ SACK Permitted                    │
└────────────────────────────────────────┘

Server says: "I support timestamps. My clock is at 5000.
              I'm echoing your TSval=100."


┌─────────────────────────────────────────────────────────┐
│ Step 3: Client ACK                                      │
└─────────────────────────────────────────────────────────┘

Client → Server:
┌────────────────────────────────────────┐
│ ACK, SEQ=1001, ACK=5001                │
│ Options:                               │
│   └─ TSopt: TSval=105, TSecr=5000      │
│             └───┬───┘  └────┬─────┘     │
│             5ms later  Echo server's   │
└────────────────────────────────────────┘

Client says: "My clock is now 105 (5ms later).
              I'm echoing your TSval=5000."

🎉 TIMESTAMPS NEGOTIATED! 🎉
Both sides must include TSopt in ALL future segments.
```

### Data Transfer with Timestamps

```
Complete Timestamp Flow:
────────────────────────────────────────────────────

T=0ms    Client sends data
         ┌─────────────────────────────────────┐
         │ SEQ=1001, LEN=1000                  │
         │ TSopt: TSval=100, TSecr=5000        │
         └─────────────────────────────────────┘
         Client ────────────────────────────────► Server
         
         Server receives at its time: 5050
         Server calculates: "Received TSval=100"
         Server stores: Most recent TSval from client = 100


T=50ms   Server ACKs data
         ┌─────────────────────────────────────┐
         │ ACK=2001                            │
         │ TSopt: TSval=5050, TSecr=100        │
         │        └────┬────┘ └───┬─────┘       │
         │        Server's    Echo of          │
         │        current     client's         │
         └─────────────────────────────────────┘
         Server ────────────────────────────────► Client
         
         Client receives ACK with TSecr=100
         Client calculates RTT:
           RTT = Current_time - TSecr
           RTT = 150 - 100 = 50ms ✓


T=100ms  Client sends more data
         ┌─────────────────────────────────────┐
         │ SEQ=2001, LEN=1000                  │
         │ TSopt: TSval=200, TSecr=5050        │
         │        └────┬────┘ └────┬─────┘     │
         │        100ms later  Echo server's   │
         └─────────────────────────────────────┘
         Client ────────────────────────────────► Server
```

---

## RTT Measurement with Timestamps

### The Algorithm (RFC 7323 Section 3.4)

```
RTT Measurement Algorithm:
────────────────────────────────────────────────────

1. On sending segment:
   Include TSval = current timestamp

2. On receiving ACK with TSecr:
   RTT = Current_time - TSecr
   
3. Update SRTT and RTTVAR:
   (Same as RFC 6298, but MORE samples!)

4. Calculate RTO:
   RTO = SRTT + 4 × RTTVAR
```

### Visual Example

```
Detailed RTT Measurement:
────────────────────────────────────────────────────

Sender Clock (milliseconds)
│
├─ T=1000: Send SEQ=1000, TSval=1000
│          │
│          ├─ Network delay: 25ms
│          │
│          ├─ T=1025: Received at peer
│          │          Peer stores: last_TSval=1000
│          │
│          ├─ Processing: 5ms
│          │
│          ├─ T=1030: Peer sends ACK, TSecr=1000
│          │
│          ├─ Network delay: 20ms
│          │
│          └─ T=1050: ACK arrives
│
├─ T=1050: Process ACK with TSecr=1000
│          Calculate: RTT = 1050 - 1000 = 50ms
│          Update SRTT: (7×old_SRTT + RTT) / 8
│          Update RTTVAR
│          Update RTO
│
└─ Result: Accurate 50ms RTT! ✓

Benefits:
────────────────────────────────────────────────────
✓ Works even after retransmissions
✓ Every ACK provides RTT sample
✓ No ambiguity (like Karn's Algorithm problem)
✓ More frequent updates = better RTO
```

### Comparison with Traditional RTT Measurement

```
Traditional (Karn's Algorithm):
────────────────────────────────────────────────────

Good scenario (no retransmission):
Send at T0, ACK at T1
RTT = T1 - T0 ✓

Bad scenario (with retransmission):
Send at T0 → timeout
Retransmit at T1, ACK at T2
Cannot measure RTT! ❌
(Don't know if ACK is for T0 or T1)

Result: Fewer RTT samples, less accurate RTO


With Timestamps:
────────────────────────────────────────────────────

Good scenario:
Send TSval=T0, ACK TSecr=T0
RTT = now - T0 ✓

Bad scenario (with retransmission):
Send TSval=T0 → timeout
Retransmit TSval=T1, ACK TSecr=T1
RTT = now - T1 ✓
(TSecr tells us it's the retransmission!)

Result: Every ACK gives RTT sample! ✓
More samples = more accurate RTO = better performance
```

---

## PAWS: Protection Against Wrapped Sequences

### The Wraparound Problem

```
High-Speed Transfer Scenario:
────────────────────────────────────────────────────

Link: 10 Gbps
Sequence space: 32 bits = 4,294,967,296 bytes

Wrap time = 4 GB / (10 Gbps / 8)
          = 4,294,967,296 / 1,250,000,000
          = 3.44 seconds

Every 3.44 seconds, sequence numbers wrap around!


Dangerous Scenario:
────────────────────────────────────────────────────

T=0.0s   Send: SEQ=4,294,960,000, TSval=1000, Data="Block 1"
T=0.1s   Send: SEQ=100 (wrapped around!), TSval=1100, Data="Block 2"
T=0.2s   Send: SEQ=5000 (wrapped around!), TSval=1200, Data="Block 3"

T=5.0s   Old packet arrives (network glitch)
         SEQ=4,294,960,000, TSval=1000, Data="Block 1"
         
         Receiver: Current TS.Recent=60000
         
         PAWS check:
         Is TSval (1000) < TS.Recent (60000)? YES!
         → DISCARD old packet ✓
         
         Without PAWS:
         SEQ=4,294,960,000 might look valid
         → Accept as new data ❌
         → Data corruption ❌

Result: PAWS prevents corruption from old packets ✓
Essential for high-speed networks!
```

### PAWS Algorithm (RFC 7323 Section 5)

```
PAWS Check (on every segment received):
────────────────────────────────────────────────────

1. Extract TSval from received segment

2. Compare with TS.Recent (most recent valid TSval):
   
   IF (TSval < TS.Recent) AND
      (SEQ is within window OR is SYN/RST):
      → DISCARD segment (too old)
      → Send ACK (in case remote thinks ACK was lost)
   
   ELSE:
      → Accept segment
      → Update TS.Recent = TSval

3. Special handling for idle connections:
   If connection idle > 24 days (wraparound window):
      → Reset PAWS check
      → Allow new timestamp


Example Check:
────────────────────────────────────────────────────

Receiver state:
TS.Recent = 50000 (last valid timestamp seen)
RCV.NXT = 1000000 (next expected byte)

Segment arrives:
SEQ = 4,294,960,000 (looks like old/wrapped data)
TSval = 1000 (much less than TS.Recent!)

PAWS check:
Is TSval (1000) < TS.Recent (50000)? YES!
→ DISCARD! ✓

Log: "PAWS: Discarded old segment TSval=1000 < TS.Recent=50000"
```

### Timestamp Wraparound Handling

```
Timestamp itself wraps after 2^32 ticks:
────────────────────────────────────────────────────

At 1ms resolution: wraps after 49.7 days
At 10ms resolution: wraps after 497 days

When timestamp wraps:
Old TSval: 4,294,967,290
New TSval: 5 (wrapped around)

Comparison must handle wraparound:
────────────────────────────────────────────────────

// Treat as signed 32-bit comparison
fn ts_before(a: u32, b: u32) -> bool {
    ((a as i32).wrapping_sub(b as i32)) < 0
}

Example:
TSval=5, TS.Recent=4,294,967,290
Is 5 < 4,294,967,290?

As unsigned: YES (5 < 4,294,967,290)
As signed wraparound: NO! (5 is AFTER wraparound)

ts_before(5, 4,294,967,290):
(5 - 4,294,967,290) as i32 = -4,294,967,285 as i32 = positive!
Result: FALSE (5 is not before, it's after wraparound) ✓
```

---

## Eifel Detection Algorithm

### What is Eifel?

RFC 3522 defines **Eifel Detection Algorithm** - uses timestamps to detect spurious retransmissions:

```
Spurious Retransmission Scenario:
────────────────────────────────────────────────────

Sender                          Network                         Receiver
  │                               │                               │
  │─── SEQ=1000, TSval=100 ───────►│                               │
  │                               │ ⏰ Delayed by congestion...   │
  │                               │                               │
  ⏰ RTO expires (premature!)     │                               │
  │                               │                               │
  │─── SEQ=1000, TSval=200 ────────┼──────────────────────────────►│
  │    (spurious retransmit)      │                     Fast path │
  │                               │                               │
  │ Reduces cwnd by 50% ❌        │                               │
  │                               │                               │
  │◄──────────────────────────────────── ACK=1100, TSecr=200 ─────│
  │                               │        "Got retransmission"   │
  │                               │                               │
  │                               │ Original packet arrives!      │
  │                               │─────────────────────────────►│
  │                               │                               │
  │◄──────────────────────────────────── ACK=1100, TSecr=100 ─────│
  │                               │     "Got original too!"       │

Eifel Detection:
────────────────────────────────────────────────────
First ACK: TSecr=200 (retransmission)
Second ACK: TSecr=100 (original!)

Sender sees: TSecr=100 < 200 → Original packet WAS delivered!
Conclusion: Retransmission was spurious! ❌
Action: UNDO cwnd reduction ✓
        INCREASE RTO (was too aggressive) ✓
```

### Eifel Algorithm

```
Eifel Detection (RFC 3522):
────────────────────────────────────────────────────

On retransmission:
  1. Store TSval of retransmitted segment: TS.retrans
  2. Set flag: awaiting_eifel_check = true

On receiving ACK:
  3. Check TSecr from ACK
  
  IF (awaiting_eifel_check) AND (TSecr < TS.retrans):
      → Original packet was delivered!
      → Retransmission was spurious
      → Action:
         ├─ Undo congestion window reduction
         ├─ Undo ssthresh reduction
         ├─ Increase RTO (double it)
         └─ Log: "Spurious retransmission detected"
  
  ELSE:
      → Retransmission was necessary
      → Keep congestion control changes


Benefits:
────────────────────────────────────────────────────
✓ Prevents unnecessary throughput reduction
✓ Adapts RTO more intelligently
✓ Better performance on variable-latency networks
✓ Particularly useful for wireless/mobile networks
```

---

## Implementation Deep Dive

### Data Structures

```rust
// filepath: /home/nazr/Desktop/projects/tcp/src/tcb.rs

#[derive(Debug, Clone, Copy)]
pub struct TimestampOption {
    /// Is timestamp option enabled?
    pub enabled: bool,
    
    /// Our timestamp value (monotonically increasing)
    pub tsval: u32,
    
    /// Last timestamp received from peer
    pub ts_recent: u32,
    
    /// Time when TS.Recent was last updated
    pub ts_recent_age: Option<Instant>,
    
    /// Timestamp when we started (for calculating TSval)
    pub start_time: Instant,
    
    /// Last TSval we sent in a retransmission (for Eifel)
    pub retrans_tsval: Option<u32>,
}

impl Tcb {
    pub fn new(quad: Quad) -> Self {
        Self {
            // ...existing code...
            timestamp: TimestampOption {
                enabled: false,
                tsval: 0,
                ts_recent: 0,
                ts_recent_age: None,
                start_time: Instant::now(),
                retrans_tsval: None,
            },
        }
    }
    
    /// Get current TSval (milliseconds since start)
    pub fn get_current_tsval(&self) -> u32 {
        if !self.timestamp.enabled {
            return 0;
        }
        
        let elapsed = self.timestamp.start_time.elapsed();
        (elapsed.as_millis() as u32).wrapping_add(1)  // Avoid 0
    }
    
    /// Check if we should include timestamp option
    pub fn should_include_timestamp(&self) -> bool {
        self.timestamp.enabled
    }
    
    /// Build timestamp option bytes (10 bytes)
    pub fn build_timestamp_option(&self, tecr: u32) -> [u8; 10] {
        let tsval = self.get_current_tsval();
        
        let mut option = [0u8; 10];
        option[0] = 8;  // Kind = Timestamp
        option[1] = 10; // Length = 10 bytes
        option[2..6].copy_from_slice(&tsval.to_be_bytes());
        option[6..10].copy_from_slice(&tecr.to_be_bytes());
        
        option
    }
    
    /// Process timestamp option from received segment
    pub fn process_timestamp_option(&mut self, tsval: u32, tecr: u32) -> bool {
        if !self.timestamp.enabled {
            return true;  // Not using timestamps
        }
        
        // PAWS check (RFC 7323 Section 5)
        if self.paws_check(tsval) {
            println!("⚠️  PAWS: Discarded old segment TSval={} < TS.Recent={}", 
                tsval, self.timestamp.ts_recent);
            return false;  // Reject old segment
        }
        
        // Update TS.Recent
        self.timestamp.ts_recent = tsval;
        self.timestamp.ts_recent_age = Some(Instant::now());
        
        // Calculate RTT from TSecr (if it echoes our TSval)
        if tecr > 0 {
            let current_ts = self.get_current_tsval();
            
            // Eifel detection: Check for spurious retransmission
            if let Some(retrans_ts) = self.timestamp.retrans_tsval {
                if tecr < retrans_ts {
                    self.handle_spurious_retransmission();
                    self.timestamp.retrans_tsval = None;
                }
            }
            
            // Measure RTT
            if current_ts >= tecr {  // Handle wraparound
                let rtt_ms = current_ts.wrapping_sub(tecr);
                self.update_rtt(rtt_ms);
            }
        }
        
        true  // Accept segment
    }
    
    /// PAWS check: Is this segment too old?
    fn paws_check(&self, tsval: u32) -> bool {
        // If connection has been idle > 24 days, reset PAWS
        if let Some(age) = self.timestamp.ts_recent_age {
            if age.elapsed().as_secs() > 24 * 24 * 3600 {
                return false;  // Don't apply PAWS after long idle
            }
        }
        
        // Check if TSval is before TS.Recent (with wraparound)
        Self::timestamp_before(tsval, self.timestamp.ts_recent)
    }
    
    /// Compare timestamps with wraparound handling
    fn timestamp_before(a: u32, b: u32) -> bool {
        // Treat as signed comparison
        ((a as i32).wrapping_sub(b as i32)) < 0
    }
    
    /// Handle detection of spurious retransmission (Eifel)
    fn handle_spurious_retransmission(&mut self) {
        println!("🔍 Eifel: Spurious retransmission detected!");
        
        // Undo congestion window reduction
        // (Would need to store pre-retransmit cwnd)
        
        // Increase RTO (it was too aggressive)
        self.timers.rto = (self.timers.rto * 2).min(60000);
        println!("Increased RTO to {}ms due to spurious retransmit", self.timers.rto);
        
        // Reset consecutive timeout counter
        self.timers.consecutive_timeouts = 0;
    }
}
```

---

## Real-World Examples

### Example 1: RTT Measurement on Variable-Latency Link

```
Scenario: Mobile network with variable latency

Connection Timeline:
────────────────────────────────────────────────────

T=0ms    Send: SEQ=1000, TSval=0
T=50ms   ACK: TSecr=0
         RTT = 50 - 0 = 50ms
         RTO = 200ms (initial)

T=100ms  Send: SEQ=1100, TSval=100
T=180ms  ACK: TSecr=100
         RTT = 180 - 100 = 80ms
         SRTT = (7×50 + 80)/8 = 53ms
         RTO updated

T=200ms  Send: SEQ=1200, TSval=200
T=500ms  ACK: TSecr=200
         RTT = 500 - 200 = 300ms! (congestion)
         SRTT = (7×53 + 300)/8 = 84ms
         RTTVAR increases significantly
         RTO = 84 + 4×RTTVAR = ~400ms

T=600ms  Send: SEQ=1300, TSval=600
T=650ms  ACK: TSecr=600
         RTT = 650 - 600 = 50ms (back to normal)
         SRTT adapts back down
         
Result: RTO adapts to network conditions ✓
Without timestamps: Fewer samples, slower adaptation ❌
```

### Example 2: PAWS on 100 Gbps Link

```
High-Speed Transfer Scenario:
────────────────────────────────────────────────────

Link: 10 Gbps
Sequence space: 32 bits = 4,294,967,296 bytes

Wrap time = 4 GB / (10 Gbps / 8)
          = 4,294,967,296 / 1,250,000,000
          = 3.44 seconds

Every 3.44 seconds, sequence numbers wrap around!


Dangerous Scenario:
────────────────────────────────────────────────────

T=0.0s   Send: SEQ=4,294,960,000, TSval=1000, Data="Block 1"
T=0.1s   Send: SEQ=100 (wrapped around!), TSval=1100, Data="Block 2"
T=0.2s   Send: SEQ=5000 (wrapped around!), TSval=1200, Data="Block 3"

T=5.0s   Old packet arrives (network glitch)
         SEQ=4,294,960,000, TSval=1000, Data="Block 1"
         
         Receiver: Current TS.Recent=60000
         
         PAWS check:
         Is TSval (1000) < TS.Recent (60000)? YES!
         → DISCARD old packet ✓
         
         Without PAWS:
         SEQ=4,294,960,000 might look valid
         → Accept as new data ❌
         → Data corruption ❌

Result: PAWS prevents corruption from old packets ✓
Essential for high-speed networks!
```

### Example 3: Eifel Detection Saves Throughput

```
Wireless Network Scenario:
────────────────────────────────────────────────────

Network: WiFi with occasional interference
Transfer: Large file download

T=0ms    Send: SEQ=1000-10000 (10 segments), TSval=100-109
         cwnd = 100KB, ssthresh = unlimited

T=150ms  Interference causes delay...
         No ACKs received

T=200ms  RTO expires!
         Reduce: cwnd = 50KB, ssthresh = 50KB
         Retransmit: SEQ=1000, TSval=200
         Store: retrans_tsval = 200

T=220ms  Original ACK arrives!
         ACK=11000, TSecr=109
         
         Eifel check:
         TSecr (109) < retrans_tsval (200)? YES!
         → Spurious retransmission detected!
         
         Action:
         ├─ Undo congestion window reduction
         ├─ Undo ssthresh reduction
         ├─ Increase RTO (double it)
         └─ Log: "Spurious retransmission detected"
  
T=240ms  Resume at full speed with cwnd=100KB ✓

Result:
────────────────────────────────────────────────────
With Eifel: Throughput temporarily affected, quickly recovers
Without Eifel: Throughput cut in half unnecessarily

Eifel saved: 50KB of throughput ✓
Better user experience on wireless networks ✓
```

---

## Performance Impact

### Overhead Analysis

```
Per-Segment Overhead:
────────────────────────────────────────────────────

Timestamp Option: 10 bytes
TCP header (no options): 20 bytes
TCP header (with TS): 30 bytes

Overhead: 10 / (1460 + 30) = 0.67%

Negligible overhead for massive benefits!


CPU Overhead:
────────────────────────────────────────────────────

Per segment:
- Get timestamp: ~50ns (read system clock)
- Compare timestamps: ~10ns (integer comparison)
- Update RTT: ~100ns (arithmetic)

Total: ~160ns per segment

On 10 Gbps link (860,000 segments/sec):
CPU time: 160ns × 860,000 = 0.137 seconds per second
         = 13.7% of one core

Acceptable for the benefits!
```

### Throughput Improvement

```
Benefits vs Costs:
────────────────────────────────────────────────────

Without Timestamps:
────────────────────────────────────────────────────
- Karn's Algorithm: Skip RTT after retransmission
- Fewer RTT samples → Less accurate RTO
- More spurious retransmissions
- No PAWS → Corruption on high-speed links
- Throughput: Lower due to conservative RTO

With Timestamps:
────────────────────────────────────────────────────
- Every ACK gives RTT sample
- More accurate RTO
- Eifel detects spurious retransmissions
- PAWS protects high-speed links
- Throughput: 10-30% higher on lossy networks

Cost: 0.67% bandwidth, 13.7% CPU
Benefit: 10-30% throughput improvement
ROI: Excellent! ✓
```

---

## Key Takeaways

### 🎯 Core Principles

1. **TSval = sender's current time** - Monotonically increasing
2. **TSecr = echo peer's TSval** - Allows RTT calculation
3. **Every segment includes timestamps** - After negotiation
4. **PAWS protects wraparound** - Essential for > 1 Gbps
5. **Eifel detects spurious retransmits** - Improves throughput

### 🔧 Implementation Checklist

```
✓ Negotiate timestamp option in SYN/SYN-ACK
✓ Use monotonically increasing clock (milliseconds)
✓ Include TSval in every segment
✓ Echo most recent received TSval in TSecr
✓ Calculate RTT from TSecr on every ACK
✓ Implement PAWS check for received segments
✓ Implement Eifel detection for spurious retransmissions
✓ Handle timestamp wraparound (49 days)
✓ Handle PAWS after long idle (> 24 days)
✓ Test with high-speed and variable-latency networks
```

### 📊 Timestamp Benefits Summary

| Benefit | Without TS | With TS | Improvement |
|---------|------------|---------|-------------|
| **RTT Samples** | Only non-retransmitted | Every ACK | 2-5× more |
| **RTO Accuracy** | Lower | Higher | 20-30% better |
| **High-speed Protection** | None | PAWS | Essential |
| **Spurious Detection** | None | Eifel | 10-30% throughput |
| **Overhead** | 0% | 0.67% | Negligible |

---

## Further Reading

- **RFC 7323** - TCP Extensions for High Performance (Timestamps + Window Scaling) ⭐ PRIMARY
- **RFC 1323** - TCP Extensions (original, obsoleted by 7323)
- **RFC 6298** - Computing TCP's Retransmission Timer
- **RFC 3522** - The Eifel Detection Algorithm for TCP
- **RFC 4015** - The Eifel Response Algorithm for TCP
- **"TCP Timestamp Option"** - IETF TCP Maintenance Working Group

---

## Conclusion

The TCP Timestamp Option is one of the most cost-effective optimizations in modern networking. For just **10 bytes per segment** (0.67% overhead), you get:

- **Accurate RTT measurement** - Even after retransmissions
- **High-speed protection** - PAWS prevents corruption
- **Spurious detection** - Eifel prevents unnecessary slowdowns
- **Better throughput** - 10-30% improvement on lossy networks

Every modern TCP implementation uses timestamps - Linux, Windows, macOS, BSD. It's not optional; it's **essential** for achieving good performance on today's Internet.

Understanding timestamps deeply is crucial for:
- **Implementing TCP correctly** - Handle all edge cases
- **Debugging performance issues** - Why is my RTO wrong?
- **Optimizing for high-speed networks** - PAWS is mandatory > 1 Gbps

The timestamp option proves that **small additions** can have **massive impact**. Ten bytes changed the Internet forever.

**Master timestamps, master modern TCP! ⏱️**

---

*Part of the 0xTCP project - Building TCP from scratch in Rust*
*Previous: [Duplicate SACK (D-SACK)](./d-sack.md) | Next: [Congestion Control](./congestion_control.md)*