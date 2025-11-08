# Selective Acknowledgment (SACK): TCP's Precision Tool

## Introduction

Imagine you're assembling a 1000-piece jigsaw puzzle, but someone only tells you: "I have all pieces up to #347." What about pieces #500-600 that arrived? Must they re-send those too? This is exactly the problem **Selective Acknowledgment (SACK)** solves in TCP.

Traditional TCP uses **cumulative acknowledgments** - they only tell the sender "I have everything up to byte X." If byte X+1 is lost but X+2 through X+1000 arrived safely, the sender has no idea and might retransmit all 1000 bytes unnecessarily. SACK changes this by saying: "I have bytes 1-1000, and also 1100-2000, but I'm missing 1001-1099."

This precision reduces retransmissions by **30-50%** on lossy networks, making SACK one of TCP's most impactful optimizations.

---

## Table of Contents

1. [The Retransmission Problem](#the-retransmission-problem)
2. [What is SACK?](#what-is-sack)
3. [SACK Option Format](#sack-option-format)
4. [How SACK Works](#how-sack-works)
5. [SACK Blocks](#sack-blocks)
6. [Sender's Scoreboard](#senders-scoreboard)
7. [SACK-Based Retransmission](#sack-based-retransmission)
8. [D-SACK (Duplicate SACK)](#d-sack-duplicate-sack)
9. [Implementation Deep Dive](#implementation-deep-dive)
10. [Real-World Examples](#real-world-examples)
11. [Performance Impact](#performance-impact)

---

## The Retransmission Problem

### Cumulative ACKs Waste Bandwidth

```
Scenario: 5 segments sent, one lost in the middle

Sender                          Receiver
  │                               │
  │─── SEQ=1000, LEN=100 ───────►│ ✓ Received
  │                               │ RCV.NXT=1100
  │◄── ACK=1100 ──────────────────│
  │                               │
  │─── SEQ=1100, LEN=100 ───────×│ LOST!
  │                               │
  │─── SEQ=1200, LEN=100 ───────►│ ✓ Received (out of order)
  │                               │ RCV.NXT=1100 (unchanged)
  │◄── ACK=1100 ──────────────────│ "Still need 1100"
  │                               │
  │─── SEQ=1300, LEN=100 ───────►│ ✓ Received (out of order)
  │◄── ACK=1100 ──────────────────│ "Still need 1100"
  │                               │
  │─── SEQ=1400, LEN=100 ───────►│ ✓ Received (out of order)
  │◄── ACK=1100 ──────────────────│ "Still need 1100"
  
  3 duplicate ACKs → Fast Retransmit!
  
  │─── SEQ=1100, LEN=100 ───────►│ ✓ Received
  │                               │
  │◄── ACK=1400 ──────────────────│ "Got everything
  │                               │  through 1399!"
```

### The Information Gap

```
What the Receiver Knows:
────────────────────────────────────────────────────
✓ Have: 1000-1099  (first segment)
✗ Missing: 1100-1199  (lost segment)
✓ Have: 1200-1299  (buffered)
✓ Have: 1300-1399  (buffered)
✓ Have: 1400-1499  (buffered)

What the Receiver Can Tell the Sender (without SACK):
────────────────────────────────────────────────────
ACK: 1100: "I need 1100"

That's it! Sender has no clue about 1200-1499.


What the Receiver Can Tell the Sender (with SACK):
────────────────────────────────────────────────────
ACK: 1100, SACK: [1200-1499]

"I need 1100, but I already have 1200-1499!"

Sender knows: Only retransmit 1100-1199 (100 bytes)
Savings: 300 bytes not retransmitted!
```

---

## What is SACK?

### Definition

**Selective Acknowledgment (SACK)** is a TCP option (RFC 2018) that allows the receiver to inform the sender about **non-contiguous blocks** of data successfully received, even when there are gaps in the sequence space.

### Key Properties

```
┌─────────────────────────────────────────────┐
│ SACK Properties                             │
├─────────────────────────────────────────────┤
│ TCP Option Kind:       5                    │
│ Negotiated via:        SACK-Permitted (SYN) │
│ Max SACK Blocks:       3-4 per segment      │
│ Block Format:          [Left Edge, Right E] │
│ Block Size:            8 bytes (2×u32)      │
│ Reduces Retransmits:   30-50%               │
│ RFC:                   2018, 2883 (D-SACK)  │
└─────────────────────────────────────────────┘
```

### Visual Comparison

```
Without SACK:
────────────────────────────────────────────────────
Byte Stream:
[1000-1099] [????-????] [1200-1299] [1300-1399]
    ✓          ✗ LOST       ✓           ✓

ACK: 1100
Message: "I have up to 1099"

Sender thinks:
  Maybe 1100+ is lost → retransmit 1100-1399 (300 bytes)


With SACK:
────────────────────────────────────────────────────
Byte Stream:
[1000-1099] [????-????] [1200-1299] [1300-1399]
    ✓          ✗ LOST       ✓           ✓

ACK: 1100, SACK: [1200-1399]
Message: "I have up to 1099, and also 1200-1399"

Sender thinks:
  Only 1100-1199 is missing → retransmit 100 bytes
  
Improvement: 3× less retransmission!
```

---

## SACK Option Format

### SACK-Permitted Option (Handshake)

Must be negotiated during the three-way handshake:

```
SYN Packet Options:
────────────────────────────────────────────────────
┌──────────┬──────────┐
│ Kind: 4  │ Length:2 │  ← SACK-Permitted
└──────────┴──────────┘

Client → Server:
SYN, Options=[MSS:1460, SACK-Permitted, WScale:7]
"I support SACK!"

Server → Client:
SYN-ACK, Options=[MSS:1460, SACK-Permitted, WScale:8]
"I support SACK too!"

Result: SACK enabled for this connection
```

### SACK Option Format (Data Transfer)

```
SACK Option Structure:
────────────────────────────────────────────────────
┌──────────┬──────────┬────────────────────────────┐
│ Kind: 5  │ Length:N │  SACK Blocks (8 bytes ea.) │
└──────────┴──────────┴────────────────────────────┘
    1 byte     1 byte       N-2 bytes

Length = 2 + (8 × number_of_blocks)

Each SACK Block:
┌─────────────────┬─────────────────┐
│  Left Edge (4B) │ Right Edge (4B) │
├─────────────────┼─────────────────┤
│  Start SEQ      │  End SEQ+1      │
└─────────────────┴─────────────────┘

Example:
Block [1200-1299]:
  Left Edge:  1200
  Right Edge: 1300 (1299 + 1, exclusive)
```

### Maximum Number of Blocks

```
TCP Header Space Constraints:
────────────────────────────────────────────────────

Maximum TCP header: 60 bytes
Minimum TCP header: 20 bytes
Available for options: 40 bytes

Common options space:
- MSS:    4 bytes
- WScale: 3 bytes
- TSopt:  10 bytes
- NOPs:   2 bytes
Total:    19 bytes

Remaining: 40 - 19 = 21 bytes

SACK option overhead: 2 bytes (Kind+Length)
Per block: 8 bytes

Maximum blocks = (21 - 2) / 8 = 2.375 → 2 blocks

With careful packing: 3-4 blocks maximum


Most Common: 3 SACK Blocks
────────────────────────────────────────────────────
SACK Option with 3 blocks:
Kind:   1 byte
Length: 1 byte (= 2 + 8×3 = 26)
Block1: 8 bytes
Block2: 8 bytes
Block3: 8 bytes
Total:  26 bytes
```

---

## How SACK Works

### Step-by-Step Process

```
Scenario: 5 segments, one lost in the middle

┌─────────────────────────────────────────────────────────┐
│ T=0ms: Sender transmits 5 segments                      │
└─────────────────────────────────────────────────────────┘

Sender                          Receiver
  │                               │
  │─── SEQ=1000, LEN=100 ───────►│ ✓ Received
  │                               │ RCV.NXT=1100
  │                               │ Deliver to app
  │◄── ACK=1100 ──────────────────│
  │                               │
  │─── SEQ=1100, LEN=100 ───────×│ LOST!
  │                               │
  │─── SEQ=1200, LEN=100 ───────►│ ✓ Received (out of order)
  │                               │ Buffer in reassembly queue
  │                               │ RCV.NXT=1100 (unchanged)
  │◄── ACK=1100 ──────────────────│ "Still need 1100"
  │                               │
  │─── SEQ=1300, LEN=100 ───────►│ ✓ Received (out of order)
  │◄── ACK=1100 ──────────────────│ "Still need 1100"
  │                               │
  │─── SEQ=1400, LEN=100 ───────►│ ✓ Received (out of order)
  │◄── ACK=1100 ──────────────────│ "Still need 1100"
  
  3 duplicate ACKs → Fast Retransmit!
  
  │─── SEQ=1100, LEN=100 ───────►│ ✓ Received
  │                               │
  │◄── ACK=1400 ──────────────────│ "Got everything
  │                               │  through 1399!"
```

### State Diagrams

```
Receiver's Reassembly Queue (with SACK):
────────────────────────────────────────────────────

After SEQ=1200 arrives:
┌────────────────────────────────────────────────┐
│ In-order: [1000-1099] ← delivered              │
│ Gap:      [1100-1199] ← MISSING                │
│ Buffered: [1200-1299] ← out of order           │
│                                                │
│ SACK Blocks to advertise:                     │
│   Block 1: [1200-1300]                         │
└────────────────────────────────────────────────┘

After SEQ=1300 arrives:
┌────────────────────────────────────────────────┐
│ In-order: [1000-1099] ← delivered              │
│ Gap:      [1100-1199] ← MISSING                │
│ Buffered: [1200-1399] ← merged blocks          │
│                                                │
│ SACK Blocks to advertise:                     │
│   Block 1: [1200-1400]                         │
└────────────────────────────────────────────────┘

After SEQ=1400 arrives:
┌────────────────────────────────────────────────┐
│ In-order: [1000-1099] ← delivered              │
│ Gap:      [1100-1199] ← MISSING                │
│ Buffered: [1200-1499] ← merged blocks          │
│                                                │
│ SACK Blocks to advertise:                     │
│   Block 1: [1200-1500]                         │
└────────────────────────────────────────────────┘
```

---

## SACK Blocks

### Block Rules (RFC 2018)

```
1. Most Recent Block First
────────────────────────────────────────────────────
When multiple blocks exist, list most recently received first.

Example:
Received: 1200-1299, then 1400-1499, then 1600-1699

SACK Blocks:
  Block 1: [1600-1700]  ← Most recent
  Block 2: [1400-1500]  ← Second most recent
  Block 3: [1200-1300]  ← Oldest

Rationale: Most recent = most likely to be useful


2. Merge Adjacent Blocks
────────────────────────────────────────────────────
If blocks become contiguous, merge them.

Received: [1200-1299]
Then:     [1300-1399]

Before merge:
  Block 1: [1200-1300]
  Block 2: [1300-1400]

After merge:
  Block 1: [1200-1400]  ← Single block

Saves option space for more coverage.


3. Maximum 4 Blocks (Practical Limit)
────────────────────────────────────────────────────
TCP option space limits SACK to 3-4 blocks typically.

Priority: Most recent blocks (closest to RCV.NXT)

Example with 5 gaps:
Gaps: [1100-1199], [1300-1399], [1500-1599], [1700-1799], [1900-1999]

Report only 3:
  Block 1: [1900-2000]  ← Closest to sender's window
  Block 2: [1700-1800]
  Block 3: [1500-1600]

Older gaps: Sender will eventually discover via timeout


4. Non-Overlapping
────────────────────────────────────────────────────
Blocks must not overlap.

✓ VALID:
  Block 1: [1200-1300]
  Block 2: [1400-1500]

✗ INVALID:
  Block 1: [1200-1400]
  Block 2: [1300-1500]  ← Overlaps with Block 1


5. Within Receive Window
────────────────────────────────────────────────────
All SACK blocks must fall within advertised receive window.

RCV.NXT = 1000
RCV.WND = 8000
Valid range: [1000-9000]

✓ VALID: Block [1200-1300]
✗ INVALID: Block [10000-11000]  ← Outside window
```

### Building SACK Blocks

```rust
// Example: Building SACK blocks from reassembly queue

struct SackBlock {
    left_edge: u32,   // Start SEQ (inclusive)
    right_edge: u32,  // End SEQ (exclusive)
}

fn generate_sack_blocks(reassembly_queue: &ReassemblyQueue, rcv_nxt: u32) 
    -> Vec<SackBlock> 
{
    let mut blocks = Vec::new();
    let mut current_start: Option<u32> = None;
    let mut current_end: u32 = 0;
    
    // Iterate through buffered segments (sorted by SEQ)
    for segment in reassembly_queue.segments.values() {
        let seg_start = segment.seq;
        let seg_end = seg_start.wrapping_add(segment.data.len() as u32);
        
        // Skip if before RCV.NXT (shouldn't happen)
        if seg_end <= rcv_nxt {
            continue;
        }
        
        match current_start {
            None => {
                // Start new block
                current_start = Some(seg_start);
                current_end = seg_end;
            }
            Some(start) => {
                // Check if contiguous with current block
                if seg_start <= current_end {
                    // Extend current block
                    current_end = current_end.max(seg_end);
                } else {
                    // Gap detected - save block
                    blocks.push(SackBlock {
                        left_edge: start,
                        right_edge: current_end,
                    });
                    
                    current_start = Some(seg_start);
                    current_end = seg_end;
                }
            }
        }
        
        // Limit to 4 blocks (option space)
        if blocks.len() >= 4 {
            break;
        }
    }
    
    // Add final block
    if let Some(start) = current_start {
        if blocks.len() < 4 {
            blocks.push(SackBlock {
                left_edge: start,
                right_edge: current_end,
            });
        }
    }
    
    blocks
}
```

---

## Sender's Scoreboard

### What is the Scoreboard?

The sender maintains a **scoreboard** tracking which bytes have been SACKed:

```
Sender's Scoreboard:
────────────────────────────────────────────────────

Sent bytes: 1000-1999 (1000 bytes)

┌──────────┬──────────┬──────────┬──────────┬──────────┐
│ 1000-1099 │ 1100-1199 │ 1200-1299 │ 1300-1399 │ 1400-1499 │
│ ACKed    │ LOST     │ SACKed   │ SACKed   │ SACKed   │
│    ✓     │    ?     │    ✓     │    ✓     │    ✓     │
└──────────┴──────────┴──────────┴──────────┴──────────┘

Scoreboard States:
- ACKed:   Cumulatively acknowledged
- SACKed:  Selectively acknowledged (but not cumulative)
- LOST:    Inferred missing (surrounded by SACKed blocks)
- Unknown: Not yet acknowledged

Only retransmit: LOST segments
Don't retransmit: ACKed or SACKed segments
```

### Scoreboard Data Structure

```rust
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SegmentState {
    Unsent,
    InFlight,     // Sent but not acknowledged
    SACKed,       // Selectively acknowledged
    Lost,         // Inferred lost (via SACK or dup ACKs)
    Retransmitted,
}

struct Scoreboard {
    /// Map: SEQ -> (length, state, timestamp)
    segments: BTreeMap<u32, (u32, SegmentState, Instant)>,
    
    /// Cumulative ACK boundary
    snd_una: u32,
    
    /// Highest sequence sent
    snd_max: u32,
}

impl Scoreboard {
    /// Update scoreboard based on received SACK blocks
    fn update_with_sack(&mut self, ack: u32, sack_blocks: &[SackBlock]) {
        // Update cumulative ACK
        self.snd_una = ack;
        
        // Remove fully ACKed segments
        self.segments.retain(|&seq, &(len, _, _)| {
            seq.wrapping_add(len) > ack
        });
        
        // Mark SACKed segments
        for block in sack_blocks {
            for (&seq, (len, state, _)) in self.segments.range_mut(
                block.left_edge..block.right_edge
            ) {
                if *state == SegmentState::InFlight {
                    *state = SegmentState::SACKed;
                    println!("SACKed: SEQ={}-{}", seq, seq + len);
                }
            }
        }
        
        // Infer losses: segments surrounded by SACKed blocks
        self.infer_losses();
    }
    
    /// Infer lost segments based on SACK holes
    fn infer_losses(&mut self) {
        let mut prev_sacked_end = self.snd_una;
        
        for (&seq, &(len, state, _)) in &self.segments {
            match state {
                SegmentState::SACKed => {
                    // Update boundary
                    prev_sacked_end = seq.wrapping_add(*len);
                }
                SegmentState::InFlight => {
                    // Check if there's a SACK beyond this segment
                    if self.has_sack_beyond(seq.wrapping_add(*len)) {
                        // Infer loss
                        *state = SegmentState::Lost;
                        println!("Inferred lost: SEQ={}-{}", seq, seq + len);
                    }
                }
                _ => {}
            }
        }
    }
    
    fn has_sack_beyond(&self, seq: u32) -> bool {
        self.segments
            .range(seq..)
            .any(|(_, &(_, state, _))| state == SegmentState::SACKed)
    }
    
    /// Get next segment to retransmit
    fn next_lost_segment(&self) -> Option<(u32, u32)> {
        for (&seq, &(len, state, _)) in &self.segments {
            if state == SegmentState::Lost {
                return Some((seq, len));
            }
        }
        None
    }
}
```

---

## SACK-Based Retransmission

### Algorithm

```
SACK-Based Retransmission (Simplified):
────────────────────────────────────────────────────

1. On receiving ACK with SACK:
   a. Update scoreboard with SACK blocks
   b. Mark SACKed bytes
   c. Infer losses (gaps between SACKed blocks)

2. Retransmission decision:
   a. If segment marked LOST and cwnd allows:
      → Retransmit immediately
   b. If 3 duplicate ACKs (with or without SACK):
      → Fast Retransmit first lost segment
   c. Otherwise:
      → Wait for timeout

3. After retransmitting:
   a. Mark segment as Retransmitted
   b. Start retransmit timer
   c. Don't retransmit SACKed segments
```

### Example Flow

```
Scenario: 10 segments sent, #2 and #5 lost

Sender                          Receiver
  │                               │
  │─── SEQ=1000, LEN=100 ───────►│ ✓ Received
  │                               │
  │─── SEQ=1100, LEN=100 ───────×│ LOST!
  │                               │
  │─── SEQ=1200, LEN=100 ───────►│ ✓ Received (out of order)
  │                               │
  │─── SEQ=1300, LEN=100 ───────►│ ✓ Received (out of order)
  │                               │
  │─── SEQ=1400, LEN=100 ───────►│ ✓ Received (out of order)
  │                               │
  │◄── ACK=1100, ──────────────────│ Duplicate ACK
  │    SACK:[1200-1300,           │ Segments 3-4
  │         1400-1500]            │ Segments 6-10
  
  Sender receives 3 duplicate ACKs → Fast Retransmit!
  But SACK tells us: only 1100-1199 missing!
  
  │─── SEQ=1100, LEN=100 ───────►│ ✓ Received
  │                               │
  │◄── ACK=1400 ──────────────────│ "Got everything
  │                               │  through 1399!"
```

---

## Implementation Deep Dive

### Step 1: Add SACK Structures to TCB

```rust
// filepath: /home/nazr/Desktop/projects/tcp/src/tcb.rs

#[derive(Debug, Clone, Copy)]
pub struct SackBlock {
    pub left_edge: u32,   // Start SEQ (inclusive)
    pub right_edge: u32,  // End SEQ (exclusive)
}

#[derive(Debug, Clone)]
pub struct SackInfo {
    /// Is SACK enabled for this connection?
    pub enabled: bool,
    
    /// SACK blocks to send in next segment
    pub blocks_to_send: Vec<SackBlock>,
    
    /// Received SACK blocks from peer
    pub received_blocks: Vec<SackBlock>,
}

pub struct Tcb {
    // ...existing code...
    
    /// SACK information
    pub sack: SackInfo,
}

impl Tcb {
    pub fn new(quad: Quad) -> Self {
        Self {
            // ...existing code...
            sack: SackInfo {
                enabled: false,
                blocks_to_send: Vec::new(),
                received_blocks: Vec::new(),
            },
        }
    }
    
    /// Generate SACK blocks from reassembly queue
    pub fn generate_sack_blocks(&mut self) {
        if !self.sack.enabled {
            return;
        }
        
        self.sack.blocks_to_send.clear();
        
        let mut current_start: Option<u32> = None;
        let mut current_end: u32 = 0;
        
        // Iterate through buffered segments (sorted by SEQ)
        for (&seq, &(seq_end, _)) in &self.reassembly_queue.segments {
            if seq < self.rcv.nxt {
                continue; // Skip already delivered
            }
            
            match current_start {
                None => {
                    // Start new block
                    current_start = Some(seq);
                    current_end = seq_end;
                }
                Some(start) => {
                    if seq <= current_end {
                        // Contiguous - extend
                        current_end = current_end.max(seq_end);
                    } else {
                        // Gap detected - save block
                        self.sack.blocks_to_send.push(SackBlock {
                            left_edge: start,
                            right_edge: current_end,
                        });
                        
                        current_start = Some(seq);
                        current_end = seq_end;
                    }
                }
            }
            
            // Limit to 4 blocks (option space)
            if self.sack.blocks_to_send.len() >= 4 {
                break;
            }
        }
        
        // Add final block
        if let Some(start) = current_start {
            if self.sack.blocks_to_send.len() < 4 {
                self.sack.blocks_to_send.push(SackBlock {
                    left_edge: start,
                    right_edge: current_end,
                });
            }
        }
    }
    
    /// Process received SACK blocks
    pub fn process_sack_blocks(&mut self, blocks: Vec<SackBlock>) {
        if !self.sack.enabled {
            return;
        }
        
        self.sack.received_blocks = blocks.clone();
        
        // Update retransmission queue based on SACK
        for block in &blocks {
            // Mark segments in this range as SACKed
            for segment in self.retransmission_queue.iter_mut() {
                let seg_start = segment.seq;
                let seg_end = seg_start.wrapping_add(segment.data.len() as u32);
                
                // Check if segment is within SACK block
                if seg_start >= block.left_edge && seg_end <= block.right_edge {
                    println!("SACKed: SEQ={}-{}", seg_start, seg_end);
                    segment.retransmit_count = u32::MAX; // Mark as SACKed (won't retransmit)
                }
            }
        }
        
        // Infer losses: segments between SACKed blocks
        self.infer_losses_from_sack();
    }
    
    fn infer_losses_from_sack(&mut self) {
        // If we have SACK blocks, segments before the first block
        // and between blocks are likely lost
        
        if self.sack.received_blocks.is_empty() {
            return;
        }
        
        let first_sack = self.sack.received_blocks[0].left_edge;
        
        // Mark segments before first SACK as potentially lost
        for segment in self.retransmission_queue.iter_mut() {
            let seg_start = segment.seq;
            let seg_end = seg_start.wrapping_add(segment.data.len() as u32);
            
            if seg_end <= first_sack && segment.retransmit_count == 0 {
                println!("Inferred loss: SEQ={} (before SACK)", seg_start);
                // Mark for immediate retransmission
                if let Some(retransmit_at) = segment.retransmit_at {
                    segment.retransmit_at = Some(Instant::now()); // Retransmit ASAP
                }
            }
        }
    }
}
```

### Step 2: Handshake Negotiation

```rust
// filepath: /home/nazr/Desktop/projects/tcp/src/tcp.rs

impl State {
    pub fn tcp_connection(
        state: &String,
        packet: &Packet,
        connections: &mut HashMap<Quad, Tcb>,
        quad: Quad,
    ) -> [u8; 1504] {
        let raw_packet = if state == "SYN" {
            let tcb = connections.entry(quad).or_insert_with(|| {
                let mut tcb = Tcb::new(quad);
                tcb.passive_open();
                tcb
            });

            // Check if peer sent SACK-Permitted
            if Self::has_sack_permitted(packet) {
                tcb.sack.enabled = true;
                println!("SACK enabled for connection");
            }

            let isn: u32 = 1000;
            
            tcb.process_syn(
                packet.tcp_header.sequence_number,
                packet.tcp_header.window,
                isn,
            );
            
            let ack_num = packet.tcp_header.sequence_number.wrapping_add(1);
            println!("Sending SYN-ACK: SEQ={}, ACK={}, WScale={}", 
                isn, ack_num, tcb.window.rcv_scale);

            // Build SYN-ACK with WScale option
            let response_packet = Self::create_syn_ack_with_options(
                packet,
                isn,
                ack_num,
                tcb.window.rcv_scale,
            );

            tcb.snd.nxt = isn.wrapping_add(1);
            tcb.queue_for_retransmission(isn, 0x12, vec![]);

            response_packet.create_packet()
        } else if state == "ACK" {
            // ...existing code...
        } else {
            [0u8; 1504]
        };
        
        raw_packet
    }
    
    fn has_sack_permitted(packet: &Packet) -> bool {
        // Parse TCP options for SACK-Permitted (Kind=4)
        // Placeholder - would need actual option parsing
        true // Assume enabled for now
    }
}
```