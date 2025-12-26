//! Web PTY Server - Ring Buffer module
//!
//! Lock-free ring buffer for PTY output with:
//! - Capped by bytes AND frames (whichever hits first)
//! - Drop-oldest on overflow with truncation sentinel
//! - Atomic metrics for operability

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::Instant;
use std::collections::VecDeque;
use std::sync::Mutex;
use tracing::warn;

/// Output frame with sequence number for replay watermarking
#[derive(Debug, Clone)]
pub struct OutputFrame {
    /// Monotonically increasing sequence number
    pub seq: u64,
    /// Timestamp when frame was captured
    pub timestamp: Instant,
    /// Raw output data
    pub data: Vec<u8>,
}

/// Ring buffer for PTY output with bounded capacity
/// 
/// Uses a Mutex<VecDeque> internally with atomic counters for metrics.
/// This is simpler than a lock-free queue and sufficient for single-writer scenarios.
pub struct RingBuffer {
    /// Current sequence number counter
    next_seq: AtomicU64,
    /// Maximum bytes allowed
    max_bytes: usize,
    /// Maximum frames allowed
    max_frames: usize,
    /// Current byte count
    current_bytes: AtomicUsize,
    /// Frame storage (protected by mutex, but metrics are atomic)
    frames: Mutex<VecDeque<OutputFrame>>,
    /// Total frames dropped (atomic for lock-free reads)
    drops: AtomicU64,
    /// Truncation events (for "buffer truncated" sentinel)
    truncations: AtomicU64,
    /// Last truncation notification time (for rate limiting)
    last_truncation_notice: Mutex<Option<Instant>>,
}

/// Result of pushing to ring buffer
#[derive(Debug, Clone, PartialEq)]
pub enum PushResult {
    /// Frame added successfully
    Ok,
    /// Frame added, but buffer was truncated (emit sentinel)
    Truncated { frames_dropped: u64 },
}

impl RingBuffer {
    /// Create a new ring buffer with specified limits
    pub fn new(max_bytes: usize, max_frames: usize) -> Self {
        Self {
            next_seq: AtomicU64::new(0),
            max_bytes,
            max_frames,
            current_bytes: AtomicUsize::new(0),
            frames: Mutex::new(VecDeque::with_capacity(max_frames.min(1024))),
            drops: AtomicU64::new(0),
            truncations: AtomicU64::new(0),
            last_truncation_notice: Mutex::new(None),
        }
    }
    
    /// Push a new frame to the ring buffer
    /// 
    /// Returns `Truncated` if frames were dropped (rate-limited to once per second)
    pub fn push(&self, data: Vec<u8>) -> PushResult {
        let frame_len = data.len();
        let mut dropped_this_push = 0u64;
        
        let mut frames = self.frames.lock().unwrap();
        
        // Drop oldest until we have room (check both byte and frame limits)
        while !frames.is_empty() && 
              (self.current_bytes.load(Ordering::SeqCst) + frame_len > self.max_bytes ||
               frames.len() >= self.max_frames)
        {
            if let Some(dropped) = frames.pop_front() {
                self.current_bytes.fetch_sub(dropped.data.len(), Ordering::SeqCst);
                self.drops.fetch_add(1, Ordering::SeqCst);
                dropped_this_push += 1;
            }
        }
        
        // Allocate sequence number
        let seq = self.next_seq.fetch_add(1, Ordering::SeqCst);
        
        // Add new frame
        self.current_bytes.fetch_add(frame_len, Ordering::SeqCst);
        frames.push_back(OutputFrame {
            seq,
            timestamp: Instant::now(),
            data,
        });
        
        // Check if we should emit truncation sentinel (rate-limited)
        if dropped_this_push > 0 {
            let should_notify = {
                let mut last = self.last_truncation_notice.lock().unwrap();
                let now = Instant::now();
                if last.map(|t| now.duration_since(t).as_secs() >= 1).unwrap_or(true) {
                    *last = Some(now);
                    self.truncations.fetch_add(1, Ordering::SeqCst);
                    true
                } else {
                    false
                }
            };
            
            if should_notify {
                warn!("Ring buffer truncated: {} frames dropped", dropped_this_push);
                return PushResult::Truncated { frames_dropped: dropped_this_push };
            }
        }
        
        PushResult::Ok
    }
    
    /// Get current sequence number (for watermark)
    pub fn current_seq(&self) -> u64 {
        self.next_seq.load(Ordering::SeqCst).saturating_sub(1)
    }
    
    /// Drain frames since a watermark (exclusive)
    /// 
    /// Returns frames with seq > watermark, capped at max_count
    pub fn drain_since(&self, watermark: u64, max_count: usize) -> Vec<OutputFrame> {
        let frames = self.frames.lock().unwrap();
        frames.iter()
            .filter(|f| f.seq > watermark)
            .take(max_count)
            .cloned()
            .collect()
    }
    
    /// Get all buffered frames (for initial replay)
    pub fn get_all(&self) -> Vec<OutputFrame> {
        let frames = self.frames.lock().unwrap();
        frames.iter().cloned().collect()
    }
    
    /// Get metrics for /metrics endpoint
    pub fn metrics(&self) -> RingMetrics {
        let frames = self.frames.lock().unwrap();
        RingMetrics {
            frame_count: frames.len(),
            byte_count: self.current_bytes.load(Ordering::SeqCst),
            drops: self.drops.load(Ordering::SeqCst),
            truncations: self.truncations.load(Ordering::SeqCst),
            current_seq: self.next_seq.load(Ordering::SeqCst),
        }
    }
    
    /// Clear the buffer (for cleanup)
    pub fn clear(&self) {
        let mut frames = self.frames.lock().unwrap();
        frames.clear();
        self.current_bytes.store(0, Ordering::SeqCst);
    }
}

/// Metrics from ring buffer
#[derive(Debug, Clone, Default)]
pub struct RingMetrics {
    pub frame_count: usize,
    pub byte_count: usize,
    pub drops: u64,
    pub truncations: u64,
    pub current_seq: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_ring_buffer_push_and_seq() {
        let ring = RingBuffer::new(1024, 10);
        
        assert_eq!(ring.push(vec![1, 2, 3]), PushResult::Ok);
        assert_eq!(ring.current_seq(), 0);
        
        assert_eq!(ring.push(vec![4, 5, 6]), PushResult::Ok);
        assert_eq!(ring.current_seq(), 1);
        
        let frames = ring.get_all();
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].seq, 0);
        assert_eq!(frames[1].seq, 1);
    }
    
    #[test]
    fn test_ring_buffer_byte_cap() {
        // 10 bytes max, 100 frames max
        let ring = RingBuffer::new(10, 100);
        
        // Push 5 bytes
        assert_eq!(ring.push(vec![1, 2, 3, 4, 5]), PushResult::Ok);
        assert_eq!(ring.metrics().byte_count, 5);
        assert_eq!(ring.metrics().drops, 0);
        
        // Push 6 more bytes - should drop first frame
        let result = ring.push(vec![6, 7, 8, 9, 10, 11]);
        assert!(matches!(result, PushResult::Truncated { .. }));
        assert_eq!(ring.metrics().drops, 1);
        assert_eq!(ring.get_all().len(), 1);
    }
    
    #[test]
    fn test_ring_buffer_frame_cap() {
        // 1MB bytes, 3 frames max
        let ring = RingBuffer::new(1_048_576, 3);
        
        ring.push(vec![1]);
        ring.push(vec![2]);
        ring.push(vec![3]);
        assert_eq!(ring.get_all().len(), 3);
        
        // Push 4th - should drop first
        ring.push(vec![4]);
        
        let frames = ring.get_all();
        assert_eq!(frames.len(), 3);
        assert_eq!(frames[0].data, vec![2]); // First dropped
        assert_eq!(ring.metrics().drops, 1);
    }
    
    #[test]
    fn test_drain_since_watermark() {
        let ring = RingBuffer::new(1024, 10);
        
        ring.push(vec![1]); // seq 0
        ring.push(vec![2]); // seq 1
        ring.push(vec![3]); // seq 2
        ring.push(vec![4]); // seq 3
        
        // Get frames since seq 1
        let frames = ring.drain_since(1, 100);
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].seq, 2);
        assert_eq!(frames[1].seq, 3);
    }
    
    #[test]
    fn test_truncation_rate_limiting() {
        // Very small buffer to force truncation
        let ring = RingBuffer::new(5, 1);
        
        // First push
        ring.push(vec![1, 2, 3]);
        
        // Second push causes truncation - should notify
        let result1 = ring.push(vec![4, 5, 6]);
        assert!(matches!(result1, PushResult::Truncated { .. }));
        assert_eq!(ring.metrics().truncations, 1);
        
        // Third push also truncates - but should NOT increment truncations (rate limited)
        let _result2 = ring.push(vec![7, 8, 9]);
        // Still truncates but no notification within 1 second
        assert_eq!(ring.metrics().truncations, 1); // Still 1, not 2
    }
}
