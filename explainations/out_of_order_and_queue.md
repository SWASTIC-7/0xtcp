# Out-of-Order Segments & Reassembly Queue: TCP's Puzzle Solver

## Introduction

Imagine receiving a 1000-piece jigsaw puzzle through the mail, but the pieces arrive **randomly** - piece #547, then #23, then #891. You can't complete the puzzle until you have all pieces in order, but do you throw away the later pieces? Of course not! You **keep them** and wait for the missing ones.

This is exactly what TCP's **Reassembly Queue** does. When network packets arrive out of order (which happens frequently on the Internet), TCP doesn't discard them. Instead, it buffers them intelligently and delivers data to the application only when it can maintain perfect byte-order.

The Reassembly Queue is one of TCP's unsung heroes - it dramatically reduces retransmissions and improves throughput, especially on lossy or reordered networks. Let's dive deep into how it works.

---

## Table of Contents

1. [The Out-of-Order Problem](#the-out-of-order-problem)
2. [What is a Reassembly Queue?](#what-is-a-reassembly-queue)
3. [How Reassembly Works](#how-reassembly-works)
4. [Data Structures & Algorithms](#data-structures--algorithms)
5. [Memory Management](#memory-management)
6. [Integration with ACKs](#integration-with-acks)
7. [Implementation Deep Dive](#implementation-deep-dive)
8. [Real-World Examples](#real-world-examples)
9. [Performance Implications](#performance-implications)

---

## The Out-of-Order Problem

### Why Do Packets Arrive Out of Order?

The Internet doesn't guarantee in-order delivery:

```
Reasons for Reordering:
┌─────────────────────────────────────────────┐
│ 1. Multiple Network Paths                   │
│    Different routes have different delays    │
│                                             │
│ 2. Packet Prioritization                    │
│    Routers may prioritize certain packets   │
│                                             │
│ 3. Link Failures & Rerouting                │
│    Packets take detours around failures     │
│                                             │
│ 4. Parallelism in Network Equipment         │
│    Multi-core routers process packets       │
│    in parallel                              │
└─────────────────────────────────────────────┘
```

### The Naive Approach (Drop Everything)

```
Without Reassembly Queue:
────────────────────────────────────────────────

Sender                          Receiver
  │                               │
  │─── SEQ=1000 (100 bytes) ────►│ ✓ Delivered to app
  │                               │ RCV.NXT = 1100
  │─── SEQ=1100 (100 bytes) ────×│ LOST!
  │                               │
  │─── SEQ=1200 (100 bytes) ────►│ ✓ Out of order
  │                               │ Expected 1100, got 1200
  │                               │ ✗ DISCARD!
  │◄── ACK=1100 ──────────────────│
  │                               │
  │─── SEQ=1300 (100 bytes) ────►│ ✓ Out of order
  │                               │ Expected 1100, got 1300
  │                               │ ✗ DISCARD!
  │◄── ACK=1100 ──────────────────│

Problem: Sender must retransmit 1100, 1200, AND 1300!
Result: Wasted bandwidth (retransmitting data we already have)
```

---

## What is a Reassembly Queue?

### Definition

A **Reassembly Queue** (also called **Out-of-Order Queue**) is a buffer that stores TCP segments that arrived ahead of their expected position in the byte stream.

### Key Properties

```
┌─────────────────────────────────────────────┐
│ Reassembly Queue Properties                 │
├─────────────────────────────────────────────┤
│ • Ordered by sequence number                │
│ • Stores segments until gaps are filled     │
│ • Prevents data loss from reordering        │
│ • Bounded by memory limits                  │
│ • Merges overlapping segments               │
│ • Delivers data only when contiguous        │
└─────────────────────────────────────────────┘
```

### Visual Representation

```
Application
    ↑
    │ read() only returns in-order data
    │
┌───┴────────────────────────────────────────┐
│  TCP Receive Buffer                        │
├────────────────────────────────────────────┤
│ In-Order Data: 1000-1099 (ready)           │
│                                            │
│ GAP: 1100-1199 (missing!)                  │
│                                            │
│ Reassembly Queue:                          │
│   ├─ 1200-1299 (buffered)                  │
│   ├─ 1300-1399 (buffered)                  │
│   └─ 1400-1499 (buffered)                  │
└────────────────────────────────────────────┘
         ↑
    RCV.NXT = 1100 (next expected byte)
```

---

## How Reassembly Works

### Step-by-Step Process

```
Scenario: Bytes 1-100 arrive, then 301-400, then 101-200

Step 1: Bytes 1-100 arrive (in order)
────────────────────────────────────────────────
┌────────────────────────────────────────────┐
│ Buffer:                                    │
│ [1-100] ← delivered to application         │
│                                            │
│ RCV.NXT = 101                              │
│ Reassembly Queue: (empty)                  │
└────────────────────────────────────────────┘


Step 2: Bytes 301-400 arrive (out of order!)
────────────────────────────────────────────────
┌────────────────────────────────────────────┐
│ Buffer:                                    │
│ [1-100] ← already delivered                │
│                                            │
│ GAP: [101-300] ← missing!                  │
│                                            │
│ RCV.NXT = 101 (unchanged)                  │
│ Reassembly Queue:                          │
│   └─ [301-400] ← BUFFERED                  │
└────────────────────────────────────────────┘

ACK sent: ACK=101 (we still need 101!)


Step 3: Bytes 201-300 arrive (still out of order)
────────────────────────────────────────────────────
┌────────────────────────────────────────────┐
│ Buffer:                                    │
│ [1-100] ← already delivered                │
│                                            │
│ GAP: [101-200] ← still missing!            │
│                                            │
│ RCV.NXT = 101 (unchanged)                  │
│ Reassembly Queue:                          │
│   ├─ [201-300] ← BUFFERED (inserted)       │
│   └─ [301-400] ← BUFFERED                  │
└────────────────────────────────────────────┘

ACK sent: ACK=101 (duplicate ACK #2)


Step 4: Bytes 101-200 arrive (fills the gap!)
────────────────────────────────────────────────────
┌────────────────────────────────────────────┐
│ Buffer:                                    │
│ [1-100]   ← already delivered              │
│ [101-200] ← just arrived!                  │
│ [201-300] ← from reassembly queue          │
│ [301-400] ← from reassembly queue          │
│     ↓                                      │
│ Deliver [101-400] to application! ✓        │
│                                            │
│ RCV.NXT = 401 (advanced!)                  │
│ Reassembly Queue: (empty)                  │
└────────────────────────────────────────────┘

ACK sent: ACK=401 (all caught up!)
```

### Algorithm Flow

```
On segment arrival with SEQ and DATA:
──────────────────────────────────────────────

1. IF SEQ < RCV.NXT:
      → Duplicate or old data
      → Discard
      → Send ACK with current RCV.NXT

2. ELSE IF SEQ == RCV.NXT:
      → In-order segment!
      → Deliver DATA to application
      → RCV.NXT += len(DATA)
      → Check reassembly queue:
         WHILE queue has segment at RCV.NXT:
            → Deliver that segment to app
            → RCV.NXT += len(segment)
            → Remove from queue
      → Send ACK with new RCV.NXT

3. ELSE (SEQ > RCV.NXT):
      → Out-of-order segment
      → Insert into reassembly queue
      → Send ACK with current RCV.NXT (duplicate)
      → Optionally send SACK blocks
```

---

## Data Structures & Algorithms

### Option 1: Ordered List (Simple)

```rust
// Simple but O(n) insertion
pub struct ReassemblyQueue {
    segments: Vec<BufferedSegment>,
}

#[derive(Clone)]
struct BufferedSegment {
    seq_start: u32,
    seq_end: u32,
    data: Vec<u8>,
}

impl ReassemblyQueue {
    fn insert(&mut self, seq: u32, data: Vec<u8>) {
        let seg_end = seq.wrapping_add(data.len() as u32);
        
        // Find insertion point (maintain sorted order)
        let pos = self.segments
            .iter()
            .position(|s| s.seq_start > seq)
            .unwrap_or(self.segments.len());
        
        self.segments.insert(pos, BufferedSegment {
            seq_start: seq,
            seq_end: seg_end,
            data,
        });
    }
}
```

### Option 2: BTreeMap (Efficient)

```rust
use std::collections::BTreeMap;

// O(log n) insertion and lookup
pub struct ReassemblyQueue {
    // Key: seq_start, Value: (seq_end, data)
    segments: BTreeMap<u32, (u32, Vec<u8>)>,
    total_bytes: usize,
    max_bytes: usize, // Memory limit
}

impl ReassemblyQueue {
    fn insert(&mut self, seq: u32, data: Vec<u8>) -> Result<(), &'static str> {
        let seg_end = seq.wrapping_add(data.len() as u32);
        
        // Check memory limit
        if self.total_bytes + data.len() > self.max_bytes {
            return Err("Reassembly queue full");
        }
        
        // Remove and merge overlapping segments
        let mut to_remove = Vec::new();
        let mut merged_start = seq;
        let mut merged_end = seg_end;
        let mut merged_data = data.clone();
        
        for (&start, &(end, ref existing_data)) in &self.segments {
            if start > merged_end {
                break; // No more overlaps
            }
            
            // Overlap detected - merge
            if end > merged_end {
                // Extend our data
                let extra_bytes = end.wrapping_sub(merged_end) as usize;
                let offset = (merged_end.wrapping_sub(start)) as usize;
                merged_data.extend_from_slice(&existing_data[offset..]);
                merged_end = end;
            }
            
            to_remove.push(start);
        }
        
        // Remove old segments
        for start in to_remove {
            if let Some((_, old_data)) = self.segments.remove(&start) {
                self.total_bytes -= old_data.len();
            }
        }
        
        // Insert merged segment
        self.total_bytes += merged_data.len();
        self.segments.insert(merged_start, (merged_end, merged_data));
        
        Ok(())
    }
    
    /// Get next contiguous segment starting at `seq`
    pub fn get_contiguous(&mut self, seq: u32) -> Option<Vec<u8>> {
        if let Some(&(seg_end, ref data)) = self.segments.get(&seq) {
            let result = data.clone();
            self.total_bytes -= data.len();
            self.segments.remove(&seq);
            Some(result)
        } else {
            None
        }
    }
    
    /// Check if we have a segment starting at `seq`
    pub fn has_segment_at(&self, seq: u32) -> bool {
        self.segments.contains_key(&seq)
    }
    
    /// Get stats
    pub fn stats(&self) -> (usize, usize) {
        (self.segments.len(), self.total_bytes)
    }
}
```

### Option 3: Interval Tree (Advanced)

```
For extremely high-performance scenarios:

Interval Tree Properties:
────────────────────────────────────────────────
• O(log n + k) for finding all overlaps
• O(log n) for insertion
• Automatically handles merging
• More complex to implement

Use when:
• Very high packet rates
• Many concurrent out-of-order segments
• Need fast overlap detection
```

---

## Memory Management

### The Memory Attack Problem

```
Attack Scenario: SYN Flood + OOO Segments
────────────────────────────────────────────────

Attacker sends:
1. SYN (establishes connection)
2. Thousands of out-of-order segments
   SEQ=1000, SEQ=2000, SEQ=3000, ...
   (never sends SEQ=0, so nothing is delivered)

Without limits:
→ Reassembly queue grows unbounded
→ Server runs out of memory
→ Denial of Service!
```

### Defense Strategies

```rust
pub struct ReassemblyQueue {
    segments: BTreeMap<u32, (u32, Vec<u8>)>,
    
    // Memory limits
    total_bytes: usize,
    max_bytes: usize,         // Per-connection limit
    max_segments: usize,      // Maximum # of segments
    
    // Time-based expiry
    insertion_times: HashMap<u32, Instant>,
    max_age: Duration,        // Expire old segments
}

impl ReassemblyQueue {
    fn enforce_limits(&mut self) {
        // Limit 1: Total bytes
        while self.total_bytes > self.max_bytes {
            self.evict_oldest();
        }
        
        // Limit 2: Number of segments
        while self.segments.len() > self.max_segments {
            self.evict_oldest();
        }
        
        // Limit 3: Age of segments
        let now = Instant::now();
        let mut to_remove = Vec::new();
        
        for (&seq, &insertion_time) in &self.insertion_times {
            if now.duration_since(insertion_time) > self.max_age {
                to_remove.push(seq);
            }
        }
        
        for seq in to_remove {
            self.remove_segment(seq);
        }
    }
    
    fn evict_oldest(&mut self) {
        // FIFO eviction: remove first segment
        if let Some((&seq, _)) = self.segments.iter().next() {
            self.remove_segment(seq);
        }
    }
}
```

### Recommended Limits

```
Conservative (embedded systems):
────────────────────────────────────────────────
max_bytes:    64 KB per connection
max_segments: 64
max_age:      10 seconds

Standard (servers):
────────────────────────────────────────────────
max_bytes:    256 KB per connection
max_segments: 256
max_age:      30 seconds

High-performance (datacenters):
────────────────────────────────────────────────
max_bytes:    1 MB per connection
max_segments: 1024
max_age:      60 seconds
```

---

## Integration with ACKs

### Cumulative ACKs with Reassembly

```
Sender                          Receiver
  │                               │
  │─── SEQ=1000, LEN=100 ───────►│ ✓ In order
  │                               │ RCV.NXT=1100
  │◄── ACK=1100 ──────────────────│
  │                               │
  │─── SEQ=1100, LEN=100 ───────×│ LOST!
  │                               │
  │─── SEQ=1200, LEN=100 ───────►│ ✓ Buffered in queue
  │                               │ RCV.NXT=1100 (unchanged)
  │◄── ACK=1100 ──────────────────│ Duplicate ACK #1
  │                               │
  │─── SEQ=1300, LEN=100 ───────►│ ✓ Buffered in queue
  │                               │ RCV.NXT=1100 (unchanged)
  │◄── ACK=1100 ──────────────────│ Duplicate ACK #2
  │                               │
  │─── SEQ=1400, LEN=100 ───────►│ ✓ Buffered in queue
  │                               │ RCV.NXT=1100 (unchanged)
  │◄── ACK=1100 ──────────────────│ Duplicate ACK #3
  │                               │
  Fast Retransmit triggered!
  │                               │
  │─── SEQ=1100, LEN=100 ───────►│ ✓ Fills gap!
  │                               │ RCV.NXT=1100
  │                               │ → Deliver 1100-1199
  │                               │ → Check queue: found 1200!
  │                               │ → Deliver 1200-1299
  │                               │ → Check queue: found 1300!
  │                               │ → Deliver 1300-1399
  │                               │ → Check queue: found 1400!
  │                               │ → Deliver 1400-1499
  │                               │ RCV.NXT=1500
  │◄── ACK=1500 ──────────────────│ ✓ All caught up!
```

### SACK (Selective Acknowledgment)

With SACK, receiver can explicitly tell sender what it has:

```
Sender                          Receiver
  │                               │
  │─── SEQ=1000 ─────────────────►│ ✓
  │─── SEQ=1100 ─────────────────×│ LOST!
  │─── SEQ=1200 ─────────────────►│ ✓ Buffered
  │─── SEQ=1300 ─────────────────►│ ✓ Buffered
  │                               │
  │◄── ACK=1100, ──────────────────│
  │    SACK: 1200-1399            │
  │                               │
  Sender knows:
  - Missing: 1100-1199
  - Have: 1200-1399
  → Retransmit only 1100-1199!
```

---

## Implementation Deep Dive

### Core Data Structure

```rust
// filepath: /home/nazr/Desktop/projects/tcp/src/tcb.rs (additions)

use std::collections::BTreeMap;

pub struct ReassemblyQueue {
    /// Buffered segments: seq_start -> (seq_end, data)
    segments: BTreeMap<u32, (u32, Vec<u8>)>,
    
    /// Total bytes buffered
    total_bytes: usize,
    
    /// Memory limits
    max_bytes: usize,
    max_segments: usize,
}

impl ReassemblyQueue {
    pub fn new(max_bytes: usize, max_segments: usize) -> Self {
        Self {
            segments: BTreeMap::new(),
            total_bytes: 0,
            max_bytes,
            max_segments,
        }
    }
    
    /// Insert an out-of-order segment
    pub fn insert(&mut self, seq: u32, data: Vec<u8>) -> Result<(), &'static str> {
        let seg_end = seq.wrapping_add(data.len() as u32);
        
        // Check memory limit
        if self.total_bytes + data.len() > self.max_bytes {
            return Err("Reassembly queue memory limit exceeded");
        }
        
        if self.segments.len() >= self.max_segments {
            return Err("Reassembly queue segment limit exceeded");
        }
        
        // Check for duplicates/overlaps
        if self.has_overlap(seq, seg_end) {
            return self.merge_with_existing(seq, seg_end, data);
        }
        
        // Insert new segment
        self.total_bytes += data.len();
        self.segments.insert(seq, (seg_end, data));
        
        Ok(())
    }
    
    /// Check if there's an overlap
    fn has_overlap(&self, seq: u32, seq_end: u32) -> bool {
        for (&start, &(end, _)) in &self.segments {
            if !(seq_end <= start || seq >= end) {
                return true; // Overlap detected
            }
        }
        false
    }
    
    /// Merge overlapping segments
    fn merge_with_existing(&mut self, seq: u32, seq_end: u32, mut data: Vec<u8>) 
        -> Result<(), &'static str> 
    {
        // Find all overlapping segments
        let mut overlaps: Vec<u32> = Vec::new();
        let mut merged_start = seq;
        let mut merged_end = seq_end;
        let mut merged_data = Vec::new();
        
        for (&start, &(end, ref seg_data)) in &self.segments {
            if seq_end > start && seq < end {
                // Overlap!
                overlaps.push(start);
                merged_start = merged_start.min(start);
                merged_end = merged_end.max(end);
            }
        }
        
        // Build merged data
        // (Simplified - production needs careful byte-level merging)
        merged_data = data.clone();
        
        // Remove old segments
        for start in overlaps {
            if let Some((_, old_data)) = self.segments.remove(&start) {
                self.total_bytes -= old_data.len();
            }
        }
        
        // Insert merged
        self.total_bytes += merged_data.len();
        self.segments.insert(merged_start, (merged_end, merged_data));
        
        Ok(())
    }
    
    /// Get next contiguous segment starting at `seq`
    pub fn get_contiguous(&mut self, seq: u32) -> Option<Vec<u8>> {
        if let Some(&(seg_end, ref data)) = self.segments.get(&seq) {
            let result = data.clone();
            self.total_bytes -= data.len();
            self.segments.remove(&seq);
            Some(result)
        } else {
            None
        }
    }
    
    /// Check if we have a segment starting at `seq`
    pub fn has_segment_at(&self, seq: u32) -> bool {
        self.segments.contains_key(&seq)
    }
    
    /// Get stats
    pub fn stats(&self) -> (usize, usize) {
        (self.segments.len(), self.total_bytes)
    }
}
```

### Integration with TCB

```rust
// filepath: /home/nazr/Desktop/projects/tcp/src/tcb.rs

impl Tcb {
    // ...existing code...
    
    /// Process received data with reassembly
    pub fn process_data(&mut self, seq: u32, data: &[u8]) -> Option<Vec<u8>> {
        // Check if segment is acceptable
        if !self.is_segment_acceptable(seq, data.len() as u32) {
            return None;
        }
        
        // Case 1: In-order segment
        if seq == self.rcv.nxt {
            self.rcv.nxt = self.rcv.nxt.wrapping_add(data.len() as u32);
            
            // Build contiguous data
            let mut delivered = data.to_vec();
            
            // Check reassembly queue for more contiguous data
            while self.reassembly_queue.has_segment_at(self.rcv.nxt) {
                if let Some(buffered) = self.reassembly_queue.get_contiguous(self.rcv.nxt) {
                    self.rcv.nxt = self.rcv.nxt.wrapping_add(buffered.len() as u32);
                    delivered.extend_from_slice(&buffered);
                } else {
                    break;
                }
            }
            
            println!("Delivered {} bytes (in-order + buffered)", delivered.len());
            Some(delivered)
        }
        // Case 2: Out-of-order segment
        else if seq > self.rcv.nxt {
            println!("Out-of-order: got SEQ={}, expected {}", seq, self.rcv.nxt);
            
            // Buffer in reassembly queue
            match self.reassembly_queue.insert(seq, data.to_vec()) {
                Ok(_) => {
                    let (seg_count, bytes) = self.reassembly_queue.stats();
                    println!("Buffered: {} segments, {} bytes total", seg_count, bytes);
                }
                Err(e) => {
                    eprintln!("Failed to buffer segment: {}", e);
                }
            }
            
            None // Don't deliver yet
        }
        // Case 3: Old/duplicate segment
        else {
            println!("Duplicate segment: SEQ={} < RCV.NXT={}", seq, self.rcv.nxt);
            None
        }
    }
    
    // Update buffer_segment to use new queue
    fn buffer_segment(&mut self, seq: u32, data: &[u8]) {
        let _ = self.reassembly_queue.insert(seq, data.to_vec());
    }
    
    fn get_next_buffered_segment(&mut self) -> Option<Vec<u8>> {
        self.reassembly_queue.get_contiguous(self.rcv.nxt)
    }
}
```

---

## Real-World Examples

### Example 1: Simple Reordering

```
Scenario: 3 segments, middle one arrives last

T=0ms    Send [SEQ=1000, 100B] ──────────────►
T=1ms    Send [SEQ=1100, 100B] ──────────────×
T=2ms    Send [SEQ=1200, 100B] ──────────────►

Network reorders - packets arrive: 1000, 1200, 1100

Receiver Timeline:
──────────────────────────────────────────────

T=50ms   Receive [SEQ=1000]
         → In order!
         → Deliver to app: bytes 1000-1099
         → RCV.NXT = 1100
         → ACK=1100

T=52ms   Receive [SEQ=1200]
         → Out of order! (expected 1100)
         → Buffer in reassembly queue:
            [1200-1299]
         → ACK=1100 (duplicate)

T=53ms   Receive [SEQ=1100]
         → In order! (fills gap)
         → Deliver to app: bytes 1100-1199
         → RCV.NXT = 1200
         → Check queue: found 1200!
         → Deliver to app: bytes 1200-1299
         → RCV.NXT = 1300
         → Check queue: found 1300!
         → Deliver to app: bytes 1300-1399
         → RCV.NXT = 1400
         → Check queue: found 1400!
         → Deliver to app: bytes 1400-1499
         → RCV.NXT = 1500
         → Reassembly queue empty
         → ACK=1500

Total delivered: 300 bytes
Without reassembly: would've retransmitted 1200!
```

### Example 2: Multiple Gaps

```
Scenario: Packet loss with multiple holes

Sent:     [1000] [1100] [1200] [1300] [1400] [1500]
Received: [1000]   ✗    [1200]   ✗    [1400] [1500]
                 LOST           LOST

Receiver State:
──────────────────────────────────────────────

After [1000]: RCV.NXT=1100, Queue=[]
After [1200]: RCV.NXT=1100, Queue=[1200-1299]
After [1400]: RCV.NXT=1100, Queue=[1200-1299, 1400-1499]
After [1500]: RCV.NXT=1100, Queue=[1200-1299, 1400-1599]

Cumulative ACKs: ACK=1100 (repeated 3 times)
→ Fast Retransmit of 1100!

After [1100] retransmitted:
  → Deliver 1100-1199
  → Check queue: found 1200!
  → Deliver 1200-1299
  → RCV.NXT=1300
  → Still missing 1300!
  → Queue=[1400-1599]
  → ACK=1300

Fast Retransmit of 1300!

After [1300] retransmitted:
  → Deliver 1300-1399
  → Check queue: found 1400!
  → Deliver 1400-1599
  → RCV.NXT=1600
  → Queue=[]
  → ACK=1600

✓ All data delivered!
```

### Example 3: Overlapping Segments

```
Scenario: Packet fragmentation causes overlaps

Sent:     [SEQ=1000, 200 bytes]
Arrives as:
  - [SEQ=1000, 100 bytes]  (first half)
  - [SEQ=1100, 100 bytes]  (second half)
  - [SEQ=1050, 150 bytes]  (overlaps both!)

Receiver Processing:
──────────────────────────────────────────────

1. Receive [1000-1099]:
   → Deliver 1000-1099
   → RCV.NXT=1100

2. Receive [1100-1199] (out of order):
   → Overlap with already-delivered data!
   → Extract new bytes: 1100-1199
   → Buffer in queue: [1100-1199]

3. Receive [1050-1199] (overlapping):
   → Extract new bytes: 1100-1199
   → Merge with buffered: 1100-1199
   → Deliver 1100-1199
   → RCV.NXT=1200
   → Queue empty

Result: Handled gracefully, no duplicates delivered
```

---

## Performance Implications

### Throughput Improvement

```
Test Scenario:
- 100 Mbps link
- 50ms RTT
- 5% packet loss
- 10% reordering rate
- 1 MB file transfer

Without Reassembly Queue:
──────────────────────────────────────────────
Reordered packets: ~100 segments
All discarded → retransmitted
Additional retransmits: ~100 segments
Wasted bandwidth: ~146 KB
Transfer time: ~2.5 seconds

With Reassembly Queue:
──────────────────────────────────────────────
Reordered packets: ~100 segments
All buffered → not retransmitted
Additional retransmits: Only lost packets (~50)
Saved bandwidth: ~146 KB
Transfer time: ~1.2 seconds

Improvement: 2× faster!
```

### Memory Usage

```
Average Case (typical web browsing):
──────────────────────────────────────────────
Segments in queue: 0-5
Memory per connection: < 10 KB
Total overhead: negligible

Worst Case (high reordering):
──────────────────────────────────────────────
Segments in queue: 50-100
Memory per connection: 100-200 KB
With 10,000 connections: 1-2 GB

Defense: Enforce limits!
```

### CPU Overhead

```
Operation Costs:
──────────────────────────────────────────────
Vec (ordered list):
  - Insert: O(n)      ~500ns for 100 segments
  - Lookup: O(n)      ~300ns
  - Merge: O(n²)      ~5μs

BTreeMap:
  - Insert: O(log n)  ~100ns for 100 segments
  - Lookup: O(log n)  ~80ns
  - Merge: O(n log n) ~1μs

Recommendation: BTreeMap for production
```

---

## Key Takeaways

### 🎯 Core Principles

1. **Never discard valid out-of-order data** - Buffer it!
2. **Deliver only contiguous data** - Maintain byte-order
3. **Enforce strict memory limits** - Prevent DoS attacks
4. **Merge overlapping segments** - Handle fragmentation
5. **Use efficient data structures** - BTreeMap > Vec

### 🔧 Implementation Checklist

```
✓ Use BTreeMap for O(log n) operations
✓ Set per-connection memory limits (256 KB typical)
✓ Set maximum segment count (256 typical)
✓ Implement overlap detection and merging
✓ Check reassembly queue after every in-order delivery
✓ Send duplicate ACKs for out-of-order segments
✓ Optionally: Send SACK blocks
✓ Monitor queue size and alert on limits
✓ Implement time-based expiry (30s typical)
```

### 📊 Performance Metrics

| Metric | Good | Bad |
|--------|------|-----|
| **Queue utilization** | < 10% of limit | > 80% of limit |
| **Average queue size** | < 5 segments | > 50 segments |
| **Queue dwell time** | < 100ms | > 1s |
| **Merge operations** | < 1% of inserts | > 10% of inserts |

---

## Further Reading

- **RFC 793** - Transmission Control Protocol (Section 3.9)
- **RFC 2018** - TCP Selective Acknowledgment Options
- **RFC 4653** - Improving TCP's Robustness to Blind In-Window Attacks
- **"TCP/IP Illustrated, Volume 2"** - Gary R. Wright & W. Richard Stevens

---

## Conclusion

The Reassembly Queue is TCP's **memory** - it remembers out-of-order data until the missing pieces arrive, then delivers everything in perfect order. Without it, TCP would waste enormous bandwidth retransmitting data it already received.

The elegance of the reassembly queue lies in its simplicity:
- **Buffer** what arrives early
- **Wait** for missing pieces
- **Deliver** when contiguous
- **Protect** against abuse with limits

This mechanism is essential for:
- **High throughput** on reordering networks
- **Efficient bandwidth usage** (fewer retransmits)
- **Robustness** against packet loss and reordering
- **Protection** against memory exhaustion attacks

Every video stream, every download, every SSH session benefits from this invisible buffer, silently reordering the chaos of the Internet into the perfect byte stream your application expects.

**Master the queue, master reliable delivery! 🧩**

---

*Part of the 0xTCP project - Building TCP from scratch in Rust*
*Previous: [Duplicate ACKs & Fast Retransmit](./duplicate_ack.md) | Next: Flow Control*