# Duplicate SACK (D-SACK): TCP's Detective Tool

## Introduction

Imagine you're a detective investigating a crime scene. The evidence tells you what happened, but you need to understand **why** it happened. Did someone steal the package, or did the delivery person accidentally deliver it twice? This is exactly what **D-SACK (Duplicate SACK)** does for TCP.

While regular SACK tells the sender "I have these byte ranges," D-SACK goes further: **"I received this data more than once."** This simple addition transforms TCP from a protocol that just fixes problems into one that **learns from them**, enabling smarter retransmission strategies, better network diagnostics, and adaptive congestion control.

RFC 2883 extends SACK with this detective capability, and the impact is profound: reduced spurious retransmissions, faster network issue detection, and self-tuning TCP stacks that adapt to real-world conditions.

---

## Table of Contents

1. [The Problem: Why Do We Need D-SACK?](#the-problem-why-do-we-need-d-sack)
2. [What is D-SACK?](#what-is-d-sack)
3. [D-SACK Format & Rules](#d-sack-format--rules)
4. [How D-SACK Works](#how-d-sack-works)
5. [Use Cases & Benefits](#use-cases--benefits)
6. [Sender's Response to D-SACK](#senders-response-to-d-sack)
7. [Implementation Deep Dive](#implementation-deep-dive)
8. [Real-World Examples](#real-world-examples)
9. [D-SACK vs Regular SACK](#d-sack-vs-regular-sack)
10. [Performance Impact](#performance-impact)

---

## The Problem: Why Do We Need D-SACK?

### Mystery #1: Spurious Retransmissions

```
Sender                          Network                         Receiver
  │                               │                               │
  │─── SEQ=1000, LEN=100 ───────►│                               │
  │                               │ ⏰ Slow network...            │
  │                               │                               │
  ⏰ RTO expires (too early!)     │                               │
  │                               │                               │
  │─── SEQ=1000, LEN=100 ───────►│ Fast path                     │
  │    (retransmit)               │─────────────────────────────►│ ✓ Received
  │                               │                               │ RCV.NXT=1100
  │◄──────────────────────────────────────────── ACK=1100 ────────│
  │                               │                               │
  │                               │ Original arrives!             │
  │                               │─────────────────────────────►│ ✓ Duplicate!
  │                               │                               │
  │◄──────────────────────────────────────────── ACK=1100 ────────│

Question: Was the retransmission necessary?

Without D-SACK:
→ Sender doesn't know the retransmission was spurious
→ RTO remains aggressive
→ More spurious retransmissions in future

With D-SACK:
→ Receiver reports: "I got 1000-1099 twice!"
→ Sender learns: "My RTO is too aggressive"
→ Increase RTO, reduce future spurious retransmissions
```

### Mystery #2: Network Duplication

```
Sender                          Network                         Receiver
  │                               │                               │
  │─── SEQ=1000, LEN=100 ───────►│                               │
  │                               │ ✂️ Packet duplicated!        │
  │                               ├──────────────────────────────►│ ✓ Original
  │                               │                               │ RCV.NXT=1100
  │                               │                               │
  │                               └──────────────────────────────►│ ✓ Duplicate!
  │                               (duplicate)                     │
  │                                                               │
  │◄──────────────────────────────────────────── ACK=1100 ────────│
  │                               D-SACK: 1000-1100               │

Question: Why did I receive this twice?

Without D-SACK:
→ Sender thinks: "Normal delivery"
→ Network problem goes undetected
→ May happen repeatedly

With D-SACK:
→ Receiver reports: "Got 1000-1099 twice!"
→ Sender learns: "Network is duplicating packets"
→ Can alert operators, avoid aggressive timeouts
```

### Mystery #3: Packet Reordering

```
Sender                          Network                         Receiver
  │                               │                               │
  │─── SEQ=1000, LEN=100 ───┐    │                               │
  │                          │    │                               │
  │─── SEQ=1100, LEN=100 ───┼───►│ Takes fast path               │
  │                          │    │─────────────────────────────►│ ✓ Out of order
  │                          │    │                               │ Expected 1000
  │                          │    │                               │ Buffer 1100-1199
  │                          │    │                               │
  │◄──────────────────────────────────────────── ACK=1000 ────────│
  │                          │    │           SACK: 1100-1200     │
  │                          │    │                               │
T=21ms   Fast Retransmit triggered                             │
  │                          │    │                               │
  │─── SEQ=1000, LEN=100 ───┼───►│                               │
  │    (retransmit)          │    │─────────────────────────────►│ ✓ Fills gap
  │                          │    │                               │ RCV.NXT=1200
  │                          │    │                               │
T=50ms   Original SEQ=1000 arrives late!         │
         ──────────────────────────────────────────►│ ✓ Duplicate!
                                          Receiver │
T=51ms   Receiver sends ACK with D-SACK        │
         Sender ◄── ACK=1200, ◄─────────────────│
                    D-SACK: 1000-1100
          
         Sender learns:
         "Fast retransmit was unnecessary!"
         "Original packet wasn't lost, just slow"
         "My RTO is too aggressive"
         "Increase RTO, reduce future spurious retransmissions"
```

---

## What is D-SACK?

### Definition

**D-SACK (Duplicate SACK)** is an extension to the SACK option that allows the receiver to inform the sender when it has received **duplicate segments** - data that arrived more than once.

### Key Properties

```
┌─────────────────────────────────────────────┐
│ D-SACK Properties                           │
├─────────────────────────────────────────────┤
│ Extension of:       SACK (RFC 2018)        │
│ RFC:                2883                    │
│ Purpose:            Report duplicates       │
│ Format:             Same as SACK blocks     │
│ Detection:          First block < ACK       │
│ Information:        "Received X twice"      │
│ Benefits:           Adaptive algorithms     │
└─────────────────────────────────────────────┘
```

### How It's Different from Regular SACK

```
Regular SACK:
────────────────────────────────────────────────────
Reports: "I have these byte ranges"
Example: ACK=1000, SACK: [1200-1300, 1400-1500]
Meaning: "Missing 1000-1199 and 1300-1399"

Purpose: Help sender know what to retransmit


D-SACK:
────────────────────────────────────────────────────
Reports: "I received these bytes MORE THAN ONCE"
Example: ACK=1100, SACK: [1000-1100]
                          └─────┬─────┘
                    Below ACK = Duplicate!
                    
Meaning: "I got 1000-1099 twice"

Purpose: Help sender learn about network behavior
```

---

## D-SACK Format & Rules

### The Key Rule: First Block Below ACK

```
D-SACK Detection Rule (RFC 2883):
────────────────────────────────────────────────────

If the FIRST SACK block has:
  Left Edge < ACK number

Then it's a D-SACK block!

Example 1: D-SACK
────────────────────────────────────────────────────
ACK: 1500
SACK Block 1: [1200-1300]  ← 1200 < 1500: D-SACK!
SACK Block 2: [2000-2100]  ← Regular SACK

Interpretation:
- Bytes 1200-1299 received twice (duplicate)
- Bytes 2000-2099 received once (out of order)


Example 2: Regular SACK (Not D-SACK)
────────────────────────────────────────────────────
ACK: 1500
SACK Block 1: [2000-2100]  ← 2000 > 1500: NOT D-SACK
SACK Block 2: [2500-2600]  ← Regular SACK

Interpretation:
- All blocks are out-of-order data (not duplicates)
```

### Format Details

```
TCP Option Structure (same as SACK):
────────────────────────────────────────────────────
┌──────────┬──────────┬────────────────────────────┐
│ Kind: 5  │ Length:N │  SACK Blocks (8 bytes ea.) │
└──────────┴──────────┴────────────────────────────┘

Each Block:
┌─────────────────┬─────────────────┐
│  Left Edge (4B) │ Right Edge (4B) │
└─────────────────┴─────────────────┘

D-SACK Block (First Position):
┌─────────────────┬─────────────────┐
│  Left Edge      │ Right Edge      │
│  < ACK number   │  ≤ ACK number   │
└─────────────────┴─────────────────┘
     ↑
     This makes it a D-SACK!
```

### RFC 2883 Rules Summary

```
Rule 1: First Block Identifies D-SACK
────────────────────────────────────────────────────
Only the FIRST SACK block can be a D-SACK
Subsequent blocks are regular SACK (out-of-order data)


Rule 2: D-SACK Reports Already-ACKed Data
────────────────────────────────────────────────────
D-SACK block must be:
- Left Edge < ACK (already acknowledged)
- Right Edge ≤ ACK (completely acknowledged)


Rule 3: Combine D-SACK with Regular SACK
────────────────────────────────────────────────────
Can report both:
- First block: Duplicate data (D-SACK)
- Subsequent blocks: Out-of-order data (SACK)


Rule 4: Most Recent Duplicate First
────────────────────────────────────────────────────
If multiple duplicates, report most recent first
(same priority rule as regular SACK)


Rule 5: Optional Feature
────────────────────────────────────────────────────
Receivers MAY send D-SACK
Senders MUST accept and process D-SACK
Backward compatible (ignored by old implementations)
```

---

## How D-SACK Works

### Scenario 1: Spurious Retransmission (RTO Too Aggressive)

```
Complete Flow with D-SACK:
────────────────────────────────────────────────────

T=0ms    Sender transmits SEQ=1000
         Sender ───► SEQ=1000, LEN=100 ───────────┐
                                                   │
T=50ms   Packet traveling through slow network... │
                                                   │
T=100ms  RTO expires! (too aggressive)            │
         Sender thinks: "Packet lost!"            │
         Sender ───► SEQ=1000, LEN=100 ──────────►│ Fast path
         (retransmit)                    Receiver │ ✓ Received
                                         RCV.NXT=1100
                                                   │
         Sender ◄──────────────────── ACK=1100 ◄─────│
                                                   │
T=150ms  Original packet finally arrives!         │
         ───────────────────────────────────────►│ ✓ Duplicate!
                                         Receiver │
                                                   │
         Sender ◄──────────────────── ACK=1100, ◄──│
                 D-SACK: 1000-1100
          
         Sender learns:                           │
         "Aha! My retransmission was spurious!"   │
         "Original packet wasn't lost, just slow" │
         "My RTO is too aggressive"               │
                                                   │
         Action: RTO ← RTO × 2                    │
         New RTO: 2 seconds (was 1 second)        │
```

### Scenario 2: Network Duplication (Faulty Router)

```
Network Duplication Flow:
────────────────────────────────────────────────────

T=0ms    Sender transmits normally
         Sender ───► SEQ=1000, LEN=100 ───────────►│
                                                    │
         Router ✂️ Duplicates packet!              │
                                                    │
T=10ms   First copy arrives                        │
         ──────────────────────────────────────────►│ ✓ Received
                                          Receiver  │ RCV.NXT=1100
                                                    │
T=11ms   Receiver ACKs first copy                  │
         Sender ◄──────────────────── ACK=1100 ◄─────│
                                                    │
T=15ms   Second copy arrives (duplicate!)          ││
         ──────────────────────────────────────────►│ ✓ Duplicate!
                                          Receiver  │
                                                    │
         Sender ◄──────────────────── ACK=1100, ◄──│
                 D-SACK: 1000-1100
         
         Sender learns:                            │
         "Network duplication (not my retransmit)" │
                                                   │
         Action: Alert monitoring system
         Human investigates and finds broken load balancer
         
Result: Network issue detected and fixed
```

### Scenario 3: Packet Reordering + Fast Retransmit

```
Reordering with D-SACK:
────────────────────────────────────────────────────

T=0ms    Send two segments
         Sender ───► SEQ=1000, LEN=100 ───┐
         Sender ───► SEQ=1100, LEN=100 ───┼───►│ Takes fast path
                                          │     │ Arrives first!
                                          │     │
T=10ms   Second segment arrives first     │     │
         ──────────────────────────────────┼────►│ ✓ Out of order
                                          │  Receiver
                                          │     │ Buffer 1100-1199
                                          │     │ Expected: 1000
                                          │     │
T=11ms   Receiver sends duplicate ACK     │     │
         Sender ◄── ACK=1000, ◄───────────┼─────│
                    SACK: [1100-1200]     │
                                          │
T=21ms   Fast Retransmit triggered                             │
  │                          │    │                               │
  │─── SEQ=1000, LEN=100 ───┼───►│                               │
  │    (retransmit)          │    │─────────────────────────────►│ ✓ Fills gap
  │                          │    │                               │ RCV.NXT=1200
  │                          │    │                               │
T=50ms   Original SEQ=1000 arrives late!         │
         ──────────────────────────────────────────►│ ✓ Duplicate!
                                          Receiver │
T=51ms   Receiver sends ACK with D-SACK        │
         Sender ◄── ACK=1200, ◄─────────────────│
                    D-SACK: 1000-1100
          
         Sender learns:
         "Fast retransmit was unnecessary!"
         "Original packet wasn't lost, just slow"
         "My RTO is too aggressive"
         "Increase RTO, reduce future spurious retransmissions"
```

---

## Use Cases & Benefits

### Use Case 1: RTO Adaptation

```
Problem: How aggressive should RTO be?
────────────────────────────────────────────────────

Too Aggressive:
- Frequent spurious retransmissions
- Wasted bandwidth
- Congestion

Too Conservative:
- Slow recovery from actual loss
- Poor throughput

Solution: D-SACK provides feedback!
────────────────────────────────────────────────────

When D-SACK received:
1. Count spurious retransmissions
2. If spurious_count > threshold:
   → RTO is too aggressive
   → Increase RTO (double it)
3. Track over time:
   → Adaptive RTO tuning

Example:
────────────────────────────────────────────────────
Initial RTO: 1 second
Spurious retransmits detected via D-SACK: 5 in 1 minute

Action: RTO ← 2 seconds

Result: Fewer spurious retransmits, better bandwidth usage
```

### Use Case 2: Network Diagnostics

```
Detecting Network Problems:
────────────────────────────────────────────────────

D-SACK Pattern Analysis:

Pattern 1: Frequent D-SACK, no retransmissions
→ Network is duplicating packets
→ Faulty router or load balancer
→ Alert operators

Pattern 2: D-SACK after every fast retransmit
→ High reordering, not loss
→ Increase reordering threshold
→ Don't reduce cwnd aggressively

Pattern 3: D-SACK for specific byte ranges
→ Middlebox interference
→ Proxy or firewall modifying packets
→ Investigate path

Pattern 4: No D-SACK despite retransmissions
→ Actual packet loss
→ Congestion control working correctly
```

### Use Case 3: Congestion Control Tuning

```
Improving Congestion Control:
────────────────────────────────────────────────────

Traditional TCP (without D-SACK):
────────────────────────────────────────────────────
3 dup ACKs → Assume loss → Reduce cwnd by 50%

Problem: If it was reordering, not loss:
→ Unnecessarily conservative
→ Lower throughput


With D-SACK:
────────────────────────────────────────────────────
3 dup ACKs → Fast retransmit
D-SACK received → "Was reordering!"
Action: Don't reduce cwnd (or reduce less)

Example:
────────────────────────────────────────────────────
cwnd = 100 packets

Without D-SACK:
Fast retransmit → cwnd = 50 ❌

With D-SACK:
Fast retransmit → D-SACK received
→ cwnd = 90 (only 10% reduction) ✓
→ Or no reduction at all

Result: Higher throughput on reordering networks
```

### Use Case 4: Reordering Tolerance

```
Adaptive Reordering Threshold:
────────────────────────────────────────────────────

Standard TCP: 3 dup ACKs → Fast retransmit

Problem: On high-reordering networks:
→ Too many spurious fast retransmits

Solution: Learn from D-SACK!
────────────────────────────────────────────────────

Track D-SACK patterns:
If D-SACK received after fast retransmit:
  reordering_events++
  
Calculate: reordering_threshold = 3 + (reordering_events / 10)

Example:
────────────────────────────────────────────────────
Network with 20% reordering

Standard: 3 dup ACKs → Fast retransmit
Result: 20% spurious retransmits

Adaptive: After learning from 50 D-SACKs:
Threshold: 3 + (50 / 10) = 8 dup ACKs
Result: <5% spurious retransmits ✓

Benefit: Better throughput, less wasted bandwidth
```

---

## Sender's Response to D-SACK

### Detection Algorithm

```rust
fn process_d_sack(sack_blocks: &[SackBlock], ack: u32) -> Option<DSackInfo> {
    if sack_blocks.is_empty() {
        return None;
    }
    
    let first_block = &sack_blocks[0];
    
    // D-SACK rule: first block's left edge < cumulative ACK
    if first_block.left_edge < ack {
        // This is a D-SACK!
        Some(DSackInfo {
            dup_start: first_block.left_edge,
            dup_end: first_block.right_edge,
            ack_number: ack,
        })
    } else {
        None
    }
}

struct DSackInfo {
    dup_start: u32,
    dup_end: u32,
    ack_number: u32,
}
```

### Response Actions

```
Action 1: Update RTO Statistics
────────────────────────────────────────────────────
If D-SACK matches a recent retransmission:
  spurious_retransmit_count++
  
  If spurious_retransmit_count > threshold:
    RTO = RTO × 2  // Increase RTO
    threshold = reset
    
  Log: "Spurious retransmit detected for SEQ=X"


Action 2: Undo Congestion Window Reduction
────────────────────────────────────────────────────
If D-SACK received after fast retransmit:
  // Retransmission was spurious
  cwnd = cwnd_before_retransmit  // Undo reduction
  ssthresh = ssthresh_before_retransmit
  
  Log: "Undoing cwnd reduction (was reordering)"


Action 3: Update Reordering Estimate
────────────────────────────────────────────────────
If D-SACK after fast retransmit:
  reordering_events++
  reordering_degree = max(reordering_degree, dup_ack_count)
  
  // Adjust future threshold
  dupthresh = 3 + (reordering_degree / 10)
  
  Log: "Reordering detected, threshold now {}", dupthresh


Action 4: Network Diagnostics
────────────────────────────────────────────────────
If D-SACK but no recent retransmission:
  // Network duplicated packet
  network_duplication_count++
  
  If network_duplication_count > threshold:
    alert("Network duplicating packets!")
    
  Log: "Network duplication for SEQ=X"


Action 5: Update RTT Measurements
────────────────────────────────────────────────────
If D-SACK received:
  // Don't use this RTT sample for RTO calculation
  // (Karn's algorithm - ignore retransmitted segments)
  
  discard_rtt_sample()
```

---

## Implementation Deep Dive

### Data Structures

```rust
// filepath: /home/nazr/Desktop/projects/tcp/src/tcb.rs

#[derive(Debug, Clone)]
pub struct DSackInfo {
    /// Start of duplicate range
    pub dup_start: u32,
    /// End of duplicate range (exclusive)
    pub dup_end: u32,
    /// Was this from our retransmission or network duplication?
    pub source: DuplicateSource,
    /// Timestamp when detected
    pub timestamp: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DuplicateSource {
    /// We retransmitted spuriously (RTO too aggressive)
    SpuriousRetransmit,
    /// Network duplicated the packet
    NetworkDuplication,
    /// Fast retransmit was unnecessary (packet reordered)
    SpuriousFastRetransmit,
    /// Unknown cause
    Unknown,
}

#[derive(Debug, Clone)]
pub struct DSackStatistics {
    /// Total D-SACKs received
    pub total_dsacks: u32,
    /// Spurious retransmits detected
    pub spurious_retransmits: u32,
    /// Network duplications detected
    pub network_duplications: u32,
    /// Spurious fast retransmits
    pub spurious_fast_retransmits: u32,
    /// Estimated reordering degree (max dup ACKs before original arrives)
    pub reordering_degree: u32,
}

impl Tcb {
    // ...existing code...
    
    pub fn new(quad: Quad) -> Self {
        Self {
            // ...existing code...
            sack: SackInfo {
                enabled: false,
                blocks_to_send: Vec::new(),
                received_blocks: Vec::new(),
                dsack_stats: DSackStatistics {
                    total_dsacks: 0,
                    spurious_retransmits: 0,
                    network_duplications: 0,
                    spurious_fast_retransmits: 0,
                    reordering_degree: 3,  // Start with RFC default
                },
            },
            // ...existing code...
        }
    }
}

#[derive(Debug, Clone)]
pub struct SackInfo {
    pub enabled: bool,
    pub blocks_to_send: Vec<SackBlock>,
    pub received_blocks: Vec<SackBlock>,
    
    /// D-SACK statistics and tracking
    pub dsack_stats: DSackStatistics,
}
```

### D-SACK Detection

```rust
// filepath: /home/nazr/Desktop/projects/tcp/src/tcb.rs

impl Tcb {
    // ...existing code...
    
    /// Process SACK blocks and detect D-SACK
    pub fn process_sack_blocks(&mut self, blocks: Vec<SackBlock>) {
        if !self.sack.enabled || blocks.is_empty() {
            return;
        }
        
        // Check for D-SACK (first block < cumulative ACK)
        if let Some(dsack_info) = self.detect_dsack(&blocks) {
            self.handle_dsack(dsack_info);
        }
        
        // Process remaining blocks as regular SACK
        let regular_sacks = if blocks[0].left_edge < self.snd.una {
            &blocks[1..]  // Skip first block (D-SACK)
        } else {
            &blocks[..]   // All blocks are regular SACK
        };
        
        self.sack.received_blocks = regular_sacks.to_vec();
        self.update_scoreboard_with_sack(regular_sacks);
    }
    
    /// Detect if first SACK block is a D-SACK
    fn detect_dsack(&self, blocks: &[SackBlock]) -> Option<DSackInfo> {
        let first_block = &blocks[0];
        
        // D-SACK rule: first block's left edge < cumulative ACK
        if first_block.left_edge < self.snd.una {
            println!("D-SACK detected: [{}-{}] (ACK={})",
                first_block.left_edge,
                first_block.right_edge,
                self.snd.una);
            
            // Determine source of duplicate
            let source = self.identify_duplicate_source(first_block);
            
            Some(DSackInfo {
                dup_start: first_block.left_edge,
                dup_end: first_block.right_edge,
                source,
                timestamp: Instant::now(),
            })
        } else {
            None
        }
    }
    
    /// Identify why we got a duplicate
    fn identify_duplicate_source(&self, block: &SackBlock) -> DuplicateSource {
        // Check if this matches a segment we recently retransmitted
        for segment in &self.retransmission_queue {
            let seg_start = segment.seq;
            let seg_end = seg_start.wrapping_add(segment.data.len() as u32);
            
            // Does D-SACK block match a retransmitted segment?
            if seg_start == block.left_edge && seg_end == block.right_edge {
                if segment.retransmit_count > 0 {
                    // We retransmitted this
                    if segment.retransmit_count == 1 && 
                       segment.timestamp.map(|t| t.elapsed().as_millis() < 100).unwrap_or(false) {
                        // Fast retransmit within 100ms
                        return DuplicateSource::SpuriousFastRetransmit;
                    } else {
                        // RTO-based retransmit
                        return DuplicateSource::SpuriousRetransmit;
                    }
                }
            }
        }
        
        // Didn't match our retransmission - network duplication
        DuplicateSource::NetworkDuplication
    }
    
    /// Handle detected D-SACK
    fn handle_dsack(&mut self, dsack: DSackInfo) {
        self.sack.dsack_stats.total_dsacks += 1;
        
        match dsack.source {
            DuplicateSource::SpuriousRetransmit => {
                self.handle_spurious_retransmit(dsack);
            }
            DuplicateSource::NetworkDuplication => {
                self.handle_network_duplication(dsack);
            }
            DuplicateSource::SpuriousFastRetransmit => {
                self.handle_spurious_fast_retransmit(dsack);
            }
            DuplicateSource::Unknown => {
                println!("Unknown duplicate source for SEQ={}", dsack.dup_start);
            }
        }
    }
    
    /// Handle spurious RTO-based retransmission
    fn handle_spurious_retransmit(&mut self, dsack: DSackInfo) {
        self.sack.dsack_stats.spurious_retransmits += 1;
        
        println!("⚠️  Spurious retransmit detected for SEQ={}-{}", 
            dsack.dup_start, dsack.dup_end);
        
        // RTO was too aggressive - increase it
        let old_rto = self.timers.rto;
        self.timers.rto = (self.timers.rto * 2).min(60000);  // Double, max 60s
        
        println!("Increasing RTO: {}ms → {}ms", old_rto, self.timers.rto);
        
        // Reset consecutive timeout counter (wasn't a real timeout)
        self.timers.consecutive_timeouts = 0;
    }
    
    /// Handle network packet duplication
    fn handle_network_duplication(&mut self, dsack: DSackInfo) {
        self.sack.dsack_stats.network_duplications += 1;
        
        println!("⚠️  Network duplication detected for SEQ={}-{}", 
            dsack.dup_start, dsack.dup_end);
        
        // Don't adjust RTO (wasn't our fault)
        // Log for network diagnostics
        
        if self.sack.dsack_stats.network_duplications % 10 == 0 {
            println!("⚠️  WARNING: {} network duplications detected. Check network path!",
                self.sack.dsack_stats.network_duplications);
        }
    }
    
    /// Handle spurious fast retransmit (packet was reordered)
    fn handle_spurious_fast_retransmit(&mut self, dsack: DSackInfo) {
        self.sack.dsack_stats.spurious_fast_retransmits += 1;
        
        println!("⚠️  Spurious fast retransmit detected for SEQ={}-{}", 
            dsack.dup_start, dsack.dup_end);
        
        // Increase reordering tolerance
        self.sack.dsack_stats.reordering_degree = 
            (self.sack.dsack_stats.reordering_degree + 1).min(10);
        
        println!("Adjusting reordering threshold: {} dup ACKs", 
            self.sack.dsack_stats.reordering_degree);
        
        // Consider undoing congestion window reduction
        // (In practice, would need to store cwnd before reduction)
    }
}
```

### Generating D-SACK Blocks

```rust
// filepath: /home/nazr/Desktop/projects/tcp/src/tcb.rs

impl Tcb {
    // ...existing code...
    
    /// Process received data and detect duplicates for D-SACK
    pub fn receive_data(&mut self, seq: u32, data: &[u8]) -> Option<Vec<u8>> {
        let seg_end = seq.wrapping_add(data.len() as u32);
        
        // Check if this is duplicate data
        if seg_end <= self.rcv.nxt {
            // Complete duplicate - entire segment already received
            println!("Duplicate segment detected: SEQ={}-{} (already have up to {})",
                seq, seg_end, self.rcv.nxt);
            
            // Generate D-SACK block for this duplicate
            self.add_dsack_block(seq, seg_end);
            
            return None;  // Don't process duplicate data
        }
        
        // Check for partial overlap with already-received data
        if seq < self.rcv.nxt {
            // Partial duplicate
            let dup_end = self.rcv.nxt;
            println!("Partial duplicate: SEQ={}-{}", seq, dup_end);
            
            // Generate D-SACK for overlapping portion
            self.add_dsack_block(seq, dup_end);
            
            // Process only new data
            let new_data_offset = (self.rcv.nxt - seq) as usize;
            return self.process_data(self.rcv.nxt, &data[new_data_offset..]);
        }
        
        // Not a duplicate - process normally
        self.process_data(seq, data)
    }
    
    /// Add D-SACK block to be sent in next ACK
    fn add_dsack_block(&mut self, start: u32, end: u32) {
        let dsack_block = SackBlock {
            left_edge: start,
            right_edge: end,
        };
        
        // D-SACK goes in FIRST position
        self.sack.blocks_to_send.insert(0, dsack_block);
        
        // Limit total blocks (D-SACK + regular SACK)
        if self.sack.blocks_to_send.len() > 4 {
            self.sack.blocks_to_send.truncate(4);
        }
    }
    
    /// Generate SACK blocks including D-SACK if present
    pub fn generate_sack_blocks(&mut self) {
        if !self.sack.enabled {
            return;
        }
        
        // Keep D-SACK block if present (already at position 0)
        let has_dsack = !self.sack.blocks_to_send.is_empty() && 
                        self.sack.blocks_to_send[0].left_edge < self.rcv.nxt;
        
        let start_idx = if has_dsack { 1 } else { 0 };
        
        // Generate regular SACK blocks from reassembly queue
        let mut regular_blocks = Vec::new();
        let mut current_start: Option<u32> = None;
        let mut current_end: u32 = 0;
        
        for (&seq, &(seq_end, _)) in &self.reassembly_queue.segments {
            if seq < self.rcv.nxt {
                continue;
            }
            
            match current_start {
                None => {
                    current_start = Some(seq);
                    current_end = seq_end;
                }
                Some(start) => {
                    if seq <= current_end {
                        current_end = current_end.max(seq_end);
                    } else {
                        regular_blocks.push(SackBlock {
                            left_edge: start,
                            right_edge: current_end,
                        });
                        current_start = Some(seq);
                        current_end = seq_end;
                    }
                }
            }
            
            if regular_blocks.len() >= 3 {
                break;
            }
        }
        
        if let Some(start) = current_start {
            if regular_blocks.len() < 3 {
                regular_blocks.push(SackBlock {
                    left_edge: start,
                    right_edge: current_end,
                });
            }
        }
        
        // Combine D-SACK (if present) with regular SACK blocks
        if has_dsack {
            // Keep D-SACK at position 0, append regular blocks
            self.sack.blocks_to_send.truncate(1);
            self.sack.blocks_to_send.extend(regular_blocks);
        } else {
            self.sack.blocks_to_send = regular_blocks;
        }
    }
}
```

---

## Real-World Examples

### Example 1: Aggressive RTO Learning

```
Scenario: New TCP connection with unknown RTT

Initial State:
────────────────────────────────────────────────────
RTO: 1 second (default)
RTT: Unknown
Network: Transcontinental fiber (actual RTT ~80ms)


T=0ms    Send SEQ=1000
         Sender ───► SEQ=1000 ─────────────────────►
         
T=80ms   Packet arrives
                                         Receiver ✓
         Sender ◄──────────────────── ACK=1100 ◄─────│
         
         Measured RTT: 80ms
         Calculate RTO: SRTT=80ms, RTTVAR=40ms
         RTO = 80 + 4×40 = 240ms


T=100ms  Send SEQ=1100
         Sender ───► SEQ=1100 ────────────────────┐
                                                   │ Slow network path
T=340ms  RTO expires! (240ms timeout)             │
         Sender thinks: "Packet lost!"            │
         Sender ───► SEQ=1100 ────────────────────►│ Fast path
         (retransmit)                    Receiver │ ✓ Received
                                         RCV.NXT=1200
         
         Sender ◄──────────────────── ACK=1200 ◄──│


T=450ms  Original packet finally arrives!         │
         ───────────────────────────────────────►│ ✓ Duplicate!
                                         Receiver │
                                                   │
         Sender ◄──────────────────── ACK=1200, ◄──│
                 D-SACK: 1100-1200
          
         Sender learns:                           │
         "Spurious retransmit! RTO too aggressive!"
         RTO ← 480ms (doubled)


T=500ms  Send SEQ=1200
         Sender ───► SEQ=1200 ──────────────────────►
                                           Receiver ✓
         Sender ◄──────────────────── ACK=1300 ◄─────│
         
         Measured RTT: 50ms
         Calculate RTO: SRTT=65ms, RTTVAR=10ms
         RTO = 65 + 4×10 = 105ms


Result: Adaptive RTO tuning based on real network behavior
```

### Example 2: Detecting Faulty Middlebox

```
Scenario: Broken load balancer duplicating packets

T=0ms    Sender transmits normally
         Sender ───► SEQ=1000, LEN=100 ───────────►│
                                                    │
         Router ✂️ Duplicates packet!              │
                                                    │
T=10ms   First copy arrives                        │
         ──────────────────────────────────────────►│ ✓ Received
                                          Receiver  │ RCV.NXT=1100
                                                    │
T=11ms   Receiver ACKs first copy                  │
         Sender ◄──────────────────── ACK=1100 ◄─────│
                                                    │
T=15ms   Second copy arrives (duplicate!)          ││
         ──────────────────────────────────────────►│ ✓ Duplicate!
                                          Receiver  │
                                                    │
         Sender ◄──────────────────── ACK=1100, ◄──│
                 D-SACK: 1000-1100
         
         Sender learns:                            │
         "Network duplication (not my retransmit)" │
                                                   │
         Action: Alert monitoring system
         Human investigates and finds broken load balancer
         
Result: Network issue detected and fixed
```

### Example 3: High Reordering Datacenter

```
Scenario: ECMP routing causing heavy reordering

Network: Datacenter with 16 parallel paths
Problem: Different paths have different delays (10-50ms)


T=0ms    Send 20 segments rapidly (SEQ=1000-2999)
         All take different paths through ECMP


T=15ms   Fast paths arrive (8 segments)
         SEQ: 1000, 1200, 1400, 1600, 1800, 2000, 2200, 2400
         
         Receiver ◄───── Most out of order! Buffer all
         RCV.NXT still at 1100 (missing 1100)


T=20ms   More arrivals (6 segments)
         SEQ: 1300, 1500, 1700, 1900, 2100, 2300
         
         Receiver sends ACKs:
         ◄─ ACK=1100, SACK: [1200-2500] (many ranges)
         ◄─ ACK=1100 (dup)
         ◄─ ACK=1100 (dup)
         ◄─ ACK=1100 (dup) ← 3 dup ACKs!


T=21ms   Fast Retransmit triggered                             │
  │                          │    │                               │
  │─── SEQ=1100, LEN=100 ───┼───►│                               │
  │    (retransmit)          │    │─────────────────────────────►│ ✓ Fills gap
  │                          │    │                               │ RCV.NXT=1200
  │                          │    │                               │
T=50ms   Original SEQ=1100 arrives late!         │
         ──────────────────────────────────────────►│ ✓ Duplicate!
                                          Receiver │
T=51ms   Receiver sends ACK with D-SACK        │
         Sender ◄── ACK=1200, ◄─────────────────│
                    D-SACK: 1100-1200
          
         Sender learns:
         "Fast retransmit was unnecessary!"
         "Original packet wasn't lost, just slow"
         "My RTO is too aggressive"
         "Increase RTO, reduce future spurious retransmissions"
```