use std::collections::VecDeque;
use std::net::Ipv4Addr;
use std::time::{Duration, Instant};

//                               +---------+ ---------\      active OPEN
//                               |  CLOSED |            \    -----------
//                               +---------+<---------\   \   create TCB
//                                 |     ^              \   \  snd SYN
//                    passive OPEN |     |   CLOSE        \   \
//                    ------------ |     | ----------       \   \
//                     create TCB  |     | delete TCB         \   \
//                                 V     |                      \   \
//                               +---------+            CLOSE    |    \
//                               |  LISTEN |          ---------- |     |
//                               +---------+          delete TCB |     |
//                    rcv SYN      |     |     SEND              |     |
//                   -----------   |     |    -------            |     V
//  +---------+      snd SYN,ACK  /       \   snd SYN          +---------+
//  |         |<-----------------           ------------------>|         |
//  |   SYN   |                    rcv SYN                     |   SYN   |
//  |   RCVD  |<-----------------------------------------------|   SENT  |
//  |         |                    snd ACK                     |         |
//  |         |------------------           -------------------|         |
//  +---------+   rcv ACK of SYN  \       /  rcv SYN,ACK       +---------+
//    |           --------------   |     |   -----------
//    |                  x         |     |     snd ACK
//    |                            V     V
//    |  CLOSE                   +---------+
//    | -------                  |  ESTAB  |
//    | snd FIN                  +---------+
//    |                   CLOSE    |     |    rcv FIN
//    V                  -------   |     |    -------
//  +---------+          snd FIN  /       \   snd ACK          +---------+
//  |  FIN    |<-----------------           ------------------>|  CLOSE  |
//  | WAIT-1  |------------------                              |   WAIT  |
//  +---------+          rcv FIN  \                            +---------+
//    | rcv ACK of FIN   -------   |                            CLOSE  |
//    | --------------   snd ACK   |                           ------- |
//    V        x                   V                           snd FIN V
//  +---------+                  +---------+                   +---------+
//  |FINWAIT-2|                  | CLOSING |                   | LAST-ACK|
//  +---------+                  +---------+                   +---------+
//    |                rcv ACK of FIN |                 rcv ACK of FIN |
//    |  rcv FIN       -------------- |    Timeout=2MSL -------------- |
//    |  -------              x       V    ------------        x       V
//     \ snd ACK                 +---------+delete TCB         +---------+
//      ------------------------>|TIME WAIT|------------------>| CLOSED  |
//                               +---------+                   +---------+

//                       TCP Connection State Diagram



/// Transmission Control Block - holds the state of a TCP connection
#[derive(Debug, Clone)]
pub struct Tcb {
    /// Connection identifiers
    pub quad: Quad,
    
    /// Connection state
    pub state: TcpState,
    
    /// Send sequence variables (RFC 793 Section 3.2)
    pub snd: SendSequence,
    
    /// Receive sequence variables (RFC 793 Section 3.2)
    pub rcv: ReceiveSequence,
    
    /// Retransmission queue
    pub retransmission_queue: VecDeque<Segment>,
    
    /// Out-of-order segments waiting to be processed
    pub reassembly_queue: VecDeque<Segment>,
    
    /// Window management
    pub window: WindowManagement,
    
    /// Timers
    pub timers: TcpTimers,
}

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub struct Quad {
    pub src: (Ipv4Addr, u16),
    pub dst: (Ipv4Addr, u16),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TcpState {
    Closed,
    Listen,
    SynSent,
    SynRcvd,
    Established,
    FinWait1,
    FinWait2,
    CloseWait,
    Closing,
    LastAck,
    TimeWait,
}

/// Send Sequence Space (RFC 793 Section 3.2)
/// 
///       1         2          3          4
///  ----------|----------|----------|----------
///         SND.UNA    SND.NXT    SND.UNA
///                              +SND.WND
///
/// 1 - old sequence numbers which have been acknowledged
/// 2 - sequence numbers of unacknowledged data
/// 3 - sequence numbers allowed for new data transmission
/// 4 - future sequence numbers which are not yet allowed
#[derive(Debug, Clone, Copy)]
pub struct SendSequence {
    /// send unacknowledged - oldest unacknowledged sequence number
    pub una: u32,
    
    /// send next - next sequence number to be sent
    pub nxt: u32,
    
    /// send window - number of bytes remote side is willing to accept
    pub wnd: u16,
    
    /// send urgent pointer
    pub up: u16,
    
    /// segment sequence number used for last window update
    pub wl1: u32,
    
    /// segment acknowledgment number used for last window update
    pub wl2: u32,
    
    /// initial send sequence number
    pub iss: u32,
}

/// Receive Sequence Space (RFC 793 Section 3.2)
///
///       1          2          3
///  ----------|----------|----------
///         RCV.NXT    RCV.NXT
///                   +RCV.WND
///
/// 1 - old sequence numbers which have been acknowledged
/// 2 - sequence numbers allowed for new reception
/// 3 - future sequence numbers which are not yet allowed
#[derive(Debug, Clone, Copy)]
pub struct ReceiveSequence {
    /// receive next - next sequence number expected
    pub nxt: u32,
    
    /// receive window - number of bytes we are willing to accept
    pub wnd: u16,
    
    /// receive urgent pointer
    pub up: u16,
    
    /// initial receive sequence number
    pub irs: u32,
}

#[derive(Debug, Clone)]
pub struct Segment {
    pub seq: u32,
    pub ack: u32,
    pub flags: u8,
    pub window: u16,
    pub data: Vec<u8>,
    pub timestamp: Option<std::time::Instant>,
    /// Number of times this segment has been retransmitted
    pub retransmit_count: u32,
    /// When this segment should be retransmitted (if timestamp is set)
    pub retransmit_at: Option<Instant>,
}

#[derive(Debug, Clone, Copy)]
pub struct WindowManagement {
    /// Maximum segment size
    pub mss: u16,
    
    /// Window scale factor (RFC 7323)
    pub scale: u8,
    
    /// Effective send window
    pub effective_wnd: u32,
    
    /// Congestion window (for congestion control)
    pub cwnd: u32,
    
    /// Slow start threshold
    pub ssthresh: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct TcpTimers {
    /// Retransmission timeout (RTO) in milliseconds
    pub rto: u32,
    
    /// Smoothed round-trip time in milliseconds
    pub srtt: u32,
    
    /// Round-trip time variation in milliseconds
    pub rttvar: u32,
    
    /// Time-Wait timer (2MSL)
    pub time_wait: Option<std::time::Instant>,
    
    /// Last time data was sent
    pub last_send: Option<std::time::Instant>,
    
    /// Last time ACK was received
    pub last_ack: Option<std::time::Instant>,
    
    /// Retransmission timer - when to check for retransmissions
    pub retransmit_timer: Option<Instant>,
    
    /// Number of consecutive retransmission timeouts
    pub consecutive_timeouts: u32,
}

impl Tcb {
    /// Create a new TCB in CLOSED state
    pub fn new(quad: Quad) -> Self {
        Self {
            quad,
            state: TcpState::Closed,
            snd: SendSequence {
                una: 0,
                nxt: 0,
                wnd: 0,
                up: 0,
                wl1: 0,
                wl2: 0,
                iss: 0,
            },
            rcv: ReceiveSequence {
                nxt: 0,
                wnd: 65535, // Default receive window
                up: 0,
                irs: 0,
            },
            retransmission_queue: VecDeque::new(),
            reassembly_queue: VecDeque::new(),
            window: WindowManagement {
                mss: 1460, // Standard MSS for Ethernet
                scale: 0,
                effective_wnd: 65535,
                cwnd: 1460 * 10, // Initial cwnd = 10 * MSS (RFC 6928)
                ssthresh: u32::MAX,
            },
            timers: TcpTimers {
                rto: 1000, // Initial RTO = 1 second
                srtt: 0,
                rttvar: 0,
                time_wait: None,
                last_send: None,
                last_ack: None,
                retransmit_timer: None,
                consecutive_timeouts: 0,
            },
        }
    }
    
    /// Initialize for active open (client)
    pub fn active_open(&mut self, iss: u32) {
        self.state = TcpState::SynSent;
        self.snd.iss = iss;
        self.snd.nxt = iss;
        self.snd.una = iss;
    }
    
    /// Initialize for passive open (server)
    pub fn passive_open(&mut self) {
        self.state = TcpState::Listen;
    }
    
    /// Process received SYN
    pub fn process_syn(&mut self, seq: u32, window: u16, iss: u32) {
        self.rcv.irs = seq;
        self.rcv.nxt = seq.wrapping_add(1);
        self.snd.wnd = window;
        
        match self.state {
            TcpState::Listen => {
                self.snd.iss = iss;
                self.snd.nxt = iss;
                self.snd.una = iss;
                self.state = TcpState::SynRcvd;
            }
            TcpState::SynSent => {
                self.state = TcpState::Established;
            }
            _ => {}
        }
    }
    
    /// Add segment to retransmission queue
    pub fn queue_for_retransmission(&mut self, seq: u32, flags: u8, data: Vec<u8>) {
        let now = Instant::now();
        let retransmit_at = now + Duration::from_millis(self.timers.rto as u64);
        
        let segment = Segment {
            seq,
            ack: 0,
            flags,
            window: 0,
            data,
            timestamp: Some(now),
            retransmit_count: 0,
            retransmit_at: Some(retransmit_at),
        };
        
        self.retransmission_queue.push_back(segment);
        self.timers.last_send = Some(now);
        
        // Set retransmission timer if not already set
        if self.timers.retransmit_timer.is_none() {
            self.timers.retransmit_timer = Some(retransmit_at);
        }
    }
    
    /// Check if retransmission timer has expired and return segments to retransmit
    pub fn check_retransmission_timeout(&mut self) -> Vec<RetransmitAction> {
        let now = Instant::now();
        let mut actions = Vec::new();
        
        // Check if retransmission timer has expired
        if let Some(timer) = self.timers.retransmit_timer {
            if now < timer {
                return actions; // Timer hasn't expired yet
            }
        } else {
            return actions; // No timer set
        }
        
        // Find segments that need retransmission
        let mut next_timer: Option<Instant> = None;
        let mut timeout_occurred = false;
        
        for segment in self.retransmission_queue.iter_mut() {
            if let Some(retransmit_at) = segment.retransmit_at {
                if now >= retransmit_at {
                    // This segment needs to be retransmitted
                    segment.retransmit_count += 1;
                    self.timers.consecutive_timeouts += 1;
                    timeout_occurred = true;
                    
                    // Exponential backoff: RTO = RTO * 2 (capped at 64x original)
                    let backoff_multiplier = 2u32.pow(segment.retransmit_count.min(6));
                    let new_rto = (self.timers.rto * backoff_multiplier).min(60000); // Max 60 seconds
                    
                    segment.retransmit_at = Some(now + Duration::from_millis(new_rto as u64));
                    
                    // Check if we've exceeded maximum retransmission attempts
                    if segment.retransmit_count >= 15 {
                        actions.push(RetransmitAction::GiveUp {
                            seq: segment.seq,
                            reason: "Maximum retransmission attempts exceeded".to_string(),
                        });
                    } else {
                        actions.push(RetransmitAction::Retransmit {
                            seq: segment.seq,
                            flags: segment.flags,
                            data: segment.data.clone(),
                            attempt: segment.retransmit_count,
                        });
                    }
                }
                
                // Track the next earliest retransmit time
                if let Some(next_time) = segment.retransmit_at {
                    next_timer = Some(match next_timer {
                        Some(current) => current.min(next_time),
                        None => next_time,
                    });
                }
            }
        }
        
        // Update congestion window on timeout (RFC 5681) - after the loop
        if timeout_occurred {
            self.handle_timeout();
        }
        
        // Update retransmission timer to next earliest deadline
        self.timers.retransmit_timer = next_timer;
        
        actions
    }
    
    /// Handle retransmission timeout - update congestion control variables
    fn handle_timeout(&mut self) {
        // On timeout, set ssthresh to max(FlightSize/2, 2*MSS) (RFC 5681)
        let flight_size = self.snd.nxt.wrapping_sub(self.snd.una);
        self.window.ssthresh = flight_size.max(2 * self.window.mss as u32) / 2;
        
        // Set cwnd to 1 MSS (enter slow start)
        self.window.cwnd = self.window.mss as u32;
        
        println!("Timeout! ssthresh={}, cwnd={}, consecutive_timeouts={}", 
            self.window.ssthresh, 
            self.window.cwnd,
            self.timers.consecutive_timeouts);
    }
    
    /// Process received ACK - enhanced with retransmission handling
    pub fn process_ack(&mut self, ack: u32, window: u16) -> bool {
        // Check if ACK is acceptable
        if !self.is_ack_acceptable(ack) {
            // Duplicate ACK handling
            return self.handle_duplicate_ack(ack);
        }
        
        // Calculate RTT if we can
        if let Some(last_send) = self.timers.last_send {
            if self.retransmission_queue.front()
                .map(|seg| seg.retransmit_count == 0)
                .unwrap_or(false) 
            {
                // Only measure RTT for non-retransmitted segments (Karn's Algorithm)
                let rtt = last_send.elapsed().as_millis() as u32;
                self.update_rtt(rtt);
            }
        }
        
        // Reset consecutive timeout counter on successful ACK
        self.timers.consecutive_timeouts = 0;
        self.timers.last_ack = Some(Instant::now());
        
        // Update send window
        self.snd.wnd = window;
        
        // Calculate how much new data was acknowledged
        let newly_acked = ack.wrapping_sub(self.snd.una);
        self.snd.una = ack;
        
        // Remove acknowledged segments from retransmission queue
        self.retransmission_queue.retain(|seg| {
            let seg_end = seg.seq.wrapping_add(seg.data.len() as u32);
            seg_end > ack
        });
        
        // Update retransmission timer
        if self.retransmission_queue.is_empty() {
            // No more unacknowledged data, stop the timer
            self.timers.retransmit_timer = None;
        } else {
            // Reset timer for remaining segments
            let now = Instant::now();
            let next_timeout = now + Duration::from_millis(self.timers.rto as u64);
            self.timers.retransmit_timer = Some(next_timeout);
            
            // Update retransmit_at for all remaining segments
            for segment in self.retransmission_queue.iter_mut() {
                segment.retransmit_at = Some(next_timeout);
            }
        }
        
        // Update congestion window (TCP Reno)
        if self.window.cwnd < self.window.ssthresh {
            // Slow start: cwnd += MSS for each ACK
            self.window.cwnd += self.window.mss as u32;
        } else {
            // Congestion avoidance: cwnd += MSS * MSS / cwnd
            let increment = (self.window.mss as u32 * self.window.mss as u32) / self.window.cwnd;
            self.window.cwnd += increment.max(1);
        }
        
        // Update state based on ACK
        match self.state {
            TcpState::SynRcvd => {
                if ack == self.snd.nxt {
                    self.state = TcpState::Established;
                }
            }
            TcpState::FinWait1 => {
                if ack == self.snd.nxt {
                    self.state = TcpState::FinWait2;
                }
            }
            TcpState::Closing => {
                if ack == self.snd.nxt {
                    self.state = TcpState::TimeWait;
                    self.start_time_wait();
                }
            }
            TcpState::LastAck => {
                if ack == self.snd.nxt {
                    self.state = TcpState::Closed;
                }
            }
            _ => {}
        }
        
        true
    }
    
    /// Handle duplicate ACK (simplified fast retransmit)
    fn handle_duplicate_ack(&mut self, _ack: u32) -> bool {
        // Could implement fast retransmit here (after 3 duplicate ACKs)
        // For now, just return false
        false
    }
    
    /// Check if ACK number is acceptable
    fn is_ack_acceptable(&self, ack: u32) -> bool {
        // ACK should be between SND.UNA and SND.NXT
        self.snd.una < ack && ack <= self.snd.nxt
    }
    
    /// Check if segment is acceptable (RFC 793 Section 3.3)
    fn is_segment_acceptable(&self, seq: u32, len: u32) -> bool {
        if len == 0 && self.rcv.wnd == 0 {
            return seq == self.rcv.nxt;
        }
        
        if len == 0 && self.rcv.wnd > 0 {
            return self.rcv.nxt <= seq && seq < self.rcv.nxt.wrapping_add(self.rcv.wnd as u32);
        }
        
        if len > 0 && self.rcv.wnd > 0 {
            let seg_end = seq.wrapping_add(len - 1);
            let wnd_end = self.rcv.nxt.wrapping_add(self.rcv.wnd as u32 - 1);
            
            (self.rcv.nxt <= seq && seq <= wnd_end) ||
            (self.rcv.nxt <= seg_end && seg_end <= wnd_end)
        } else {
            false
        }
    }
    
    /// Buffer out-of-order segment
    fn buffer_segment(&mut self, seq: u32, data: &[u8]) {
        let segment = Segment {
            seq,
            ack: 0,
            flags: 0,
            window: 0,
            data: data.to_vec(),
            timestamp: Some(std::time::Instant::now()),
            retransmit_count: 0,
            retransmit_at: None,
        };
        
        // Insert in order
        let pos = self.reassembly_queue
            .iter()
            .position(|s| s.seq > seq)
            .unwrap_or(self.reassembly_queue.len());
        
        self.reassembly_queue.insert(pos, segment);
    }
    
    /// Get next buffered segment if it's in order
    fn get_next_buffered_segment(&mut self) -> Option<Vec<u8>> {
        if let Some(seg) = self.reassembly_queue.front() {
            if seg.seq == self.rcv.nxt {
                let segment = self.reassembly_queue.pop_front().unwrap();
                self.rcv.nxt = self.rcv.nxt.wrapping_add(segment.data.len() as u32);
                return Some(segment.data);
            }
        }
        None
    }
    
    /// Start TIME-WAIT timer (2MSL)
    fn start_time_wait(&mut self) {
        self.timers.time_wait = Some(std::time::Instant::now());
    }
    
    /// Check if TIME-WAIT has expired (2MSL = 240 seconds typically)
    pub fn is_time_wait_expired(&self) -> bool {
        if let Some(start) = self.timers.time_wait {
            start.elapsed().as_secs() >= 240
        } else {
            false
        }
    }
    
    /// Calculate available send window
    pub fn available_window(&self) -> u32 {
        let in_flight = self.snd.nxt.wrapping_sub(self.snd.una);
        let wnd = std::cmp::min(self.snd.wnd as u32, self.window.cwnd);
        wnd.saturating_sub(in_flight)
    }
    
    /// Update RTT measurements (RFC 6298) - enhanced
    pub fn update_rtt(&mut self, measured_rtt: u32) {
        if self.timers.srtt == 0 {
            // First measurement (RFC 6298)
            self.timers.srtt = measured_rtt;
            self.timers.rttvar = measured_rtt / 2;
            self.timers.rto = self.timers.srtt + 4 * self.timers.rttvar;
        } else {
            // Subsequent measurements (RFC 6298)
            // RTTVAR = (1 - beta) * RTTVAR + beta * |SRTT - R'|
            // SRTT = (1 - alpha) * SRTT + alpha * R'
            // where alpha = 1/8, beta = 1/4
            
            let diff = if self.timers.srtt > measured_rtt {
                self.timers.srtt - measured_rtt
            } else {
                measured_rtt - self.timers.srtt
            };
            
            self.timers.rttvar = (3 * self.timers.rttvar + diff) / 4;
            self.timers.srtt = (7 * self.timers.srtt + measured_rtt) / 8;
            
            // RTO = SRTT + max(G, K*RTTVAR) where K=4, G=100ms
            self.timers.rto = self.timers.srtt + 4 * self.timers.rttvar.max(25);
        }
        
        // Clamp RTO between 1 second and 60 seconds (RFC 6298)
        self.timers.rto = self.timers.rto.clamp(1000, 60000);
        
        println!("RTT updated: measured={}ms, SRTT={}ms, RTTVAR={}ms, RTO={}ms",
            measured_rtt, self.timers.srtt, self.timers.rttvar, self.timers.rto);
    }
    
    /// Get time until next retransmission check (for select/poll)
    pub fn time_until_retransmit(&self) -> Option<Duration> {
        self.timers.retransmit_timer.map(|timer| {
            let now = Instant::now();
            if timer > now {
                timer.duration_since(now)
            } else {
                Duration::from_millis(0)
            }
        })
    }
}

impl Default for Tcb {
    fn default() -> Self {
        Self::new(Quad {
            src: (Ipv4Addr::new(0, 0, 0, 0), 0),
            dst: (Ipv4Addr::new(0, 0, 0, 0), 0),
        })
    }
}

/// Actions to take after checking retransmission timer
#[derive(Debug, Clone)]
pub enum RetransmitAction {
    Retransmit {
        seq: u32,
        flags: u8,
        data: Vec<u8>,
        attempt: u32,
    },
    GiveUp {
        seq: u32,
        reason: String,
    },
}


