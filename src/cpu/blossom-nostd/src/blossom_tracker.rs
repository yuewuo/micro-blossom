//! Blossom Tracker
//!
//! This is a software implementation of the obstacle detection of negative blossom shrinking.
//! It's supposed to only track all blossoms but not single vertices, to save memory.
//! This module needs to be called whenever a blossom is created or set speed.
//! A global step variable needs to be provided so that this module know what is the current dual value.
//!

use crate::util::*;
use core::cmp::Ordering;
use heapless::binary_heap::{BinaryHeap, Min};
use heapless::Vec;

// We need to maintain information about the blossoms, e.g., the dual variables of them.
// The blossom indices have nice property that they will never decreasing.
// In fact, the indices are allocated linearly, meaning it's guaranteed that the next index after K is always K+1.
// Utilizing this, we can reduce the memory usage of the mapping significantly.
pub struct BlossomTracker<const N: usize> {
    /// the priority queue of next timestamp
    hit_zero_events: BinaryHeap<HitZeroEvent, Min, N>,
    /// it is the responsibility of outer program to report the timestamp properly
    timestamp: Timestamp,
    /// the index of the first blossom, meaningless when length=0
    first_index: NodeIndex,
    /// the checkpoints of dual variables
    checkpoints: Vec<(Timestamp, Weight), N>,
    /// speed of the blossom
    grow_states: Vec<BlossomGrowState, N>,
}

struct HitZeroEvent {
    timestamp: Timestamp,
    /// the node that *probably* hits zero; it's probable because we never delete such event from the queue
    node_index: NodeIndex,
}

#[repr(u8)]
#[derive(PartialEq, Eq, Clone, Copy)]
pub enum BlossomGrowState {
    Grow,
    Shrink,
    Stay,
}

impl<const N: usize> BlossomTracker<N> {
    pub fn new() -> Self {
        Self {
            hit_zero_events: BinaryHeap::new(),
            timestamp: 0,
            first_index: NodeIndex::MAX,
            checkpoints: Vec::new(),
            grow_states: Vec::new(),
        }
    }

    pub fn advance_time(&mut self, delta: Timestamp) {
        self.timestamp += delta;
        debug_assert!(
            {
                self.remove_outdated_events();
                if let Some(event) = self.hit_zero_events.peek() {
                    self.timestamp <= event.timestamp
                } else {
                    true
                }
            },
            "hit one of the zero event"
        );
    }

    #[inline]
    fn assert_valid_node_index(&self, node_index: NodeIndex) {
        debug_assert!(node_index >= self.first_index && node_index < self.first_index + self.checkpoints.len() as NodeIndex);
    }

    #[inline]
    fn local_index_of(&self, node_index: NodeIndex) -> usize {
        self.assert_valid_node_index(node_index);
        (node_index - self.first_index) as usize
    }

    pub fn create_blossom(&mut self, node_index: NodeIndex) {
        debug_assert!(self.checkpoints.is_empty() || node_index == self.first_index + self.checkpoints.len() as NodeIndex);
        if self.checkpoints.is_empty() {
            self.first_index = node_index;
        }
        self.checkpoints.push((self.timestamp, 0)).ok().unwrap();
        self.grow_states.push(BlossomGrowState::Grow).ok().unwrap();
    }

    pub fn set_speed(&mut self, node_index: NodeIndex, grow_state: BlossomGrowState) {
        let local_index = self.local_index_of(node_index);
        // update checkpoint timestamp to the current timestamp and update its dual value accordingly
        if grow_state == self.grow_states[local_index] {
            return; // no need to set speed
        }
        let dual_value = self.local_get_dual_variable(local_index);
        self.checkpoints[local_index] = (self.timestamp, dual_value);
        self.grow_states[local_index] = grow_state;
        // insert a hit-zero event if the blossom becomes shrinking
        if grow_state == BlossomGrowState::Shrink {
            self.hit_zero_events
                .push(HitZeroEvent {
                    timestamp: self.timestamp + dual_value as Timestamp,
                    node_index,
                })
                .ok()
                .unwrap();
        }
    }

    fn local_get_dual_variable(&self, local_index: usize) -> Weight {
        let (timestamp, dual_value) = self.checkpoints[local_index];
        let delta = (self.timestamp - timestamp) as Weight;
        let dual_value = match self.grow_states[local_index] {
            BlossomGrowState::Grow => dual_value + delta,
            BlossomGrowState::Shrink => dual_value - delta,
            BlossomGrowState::Stay => dual_value,
        };
        debug_assert!(dual_value >= 0);
        dual_value
    }

    pub fn get_dual_variable(&self, node_index: NodeIndex) -> Weight {
        self.local_get_dual_variable(self.local_index_of(node_index))
    }

    #[inline]
    fn is_valid_event(&self, first_event: &HitZeroEvent) -> bool {
        let local_index = self.local_index_of(first_event.node_index);
        if self.grow_states[local_index] == BlossomGrowState::Shrink {
            let dual_value = self.local_get_dual_variable(local_index);
            let actual_timestamp = self.timestamp + dual_value as Timestamp;
            debug_assert!(
                first_event.timestamp <= actual_timestamp,
                "the first event should always capture growth"
            );
            if first_event.timestamp == actual_timestamp {
                return true;
            }
        }
        false
    }

    fn remove_outdated_events(&mut self) {
        while !self.hit_zero_events.is_empty() {
            if self.is_valid_event(self.hit_zero_events.peek().unwrap()) {
                return;
            }
            self.hit_zero_events.pop().unwrap();
        }
    }

    pub fn get_maximum_growth(&mut self) -> Weight {
        self.remove_outdated_events();
        if let Some(event) = self.hit_zero_events.peek() {
            debug_assert!(event.timestamp >= self.timestamp);
            (event.timestamp - self.timestamp) as Weight
        } else {
            Weight::MAX
        }
    }
}

impl Ord for HitZeroEvent {
    fn cmp(&self, other: &HitZeroEvent) -> Ordering {
        self.timestamp.cmp(&other.timestamp)
    }
}

impl PartialOrd for HitZeroEvent {
    fn partial_cmp(&self, other: &HitZeroEvent) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for HitZeroEvent {
    fn eq(&self, other: &Self) -> bool {
        self.timestamp == other.timestamp
    }
}

impl Eq for HitZeroEvent {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blossom_tracker_size() {
        // cargo test blossom_tracker_size -- --nocapture
        const N: usize = 1000000;
        let total_size = core::mem::size_of::<BlossomTracker<N>>();
        println!("memory: {} bytes per blossom", total_size / N);
        println!("memory overhead: {} bytes", total_size - (total_size / N) * N);
    }

    #[test]
    fn blossom_tracker_test_1() {
        // cargo test blossom_tracker_test_1 -- --nocapture
        let mut tracker = BlossomTracker::<10>::new();
        tracker.advance_time(10);
        const BLOSSOM_BIAS: NodeIndex = 0x11000;
        let node_1 = BLOSSOM_BIAS;
        let node_2 = BLOSSOM_BIAS + 1;

        tracker.create_blossom(node_1);
        assert_eq!(tracker.get_dual_variable(node_1), 0);
        assert_eq!(tracker.get_maximum_growth(), Weight::MAX);

        tracker.advance_time(20);
        assert_eq!(tracker.get_dual_variable(node_1), 20);
        assert_eq!(tracker.get_maximum_growth(), Weight::MAX);

        tracker.create_blossom(node_2);
        tracker.advance_time(30);
        assert_eq!(tracker.get_dual_variable(node_1), 50);
        assert_eq!(tracker.get_dual_variable(node_2), 30);
        assert_eq!(tracker.get_maximum_growth(), Weight::MAX);

        tracker.set_speed(node_1, BlossomGrowState::Stay);
        tracker.advance_time(10);
        assert_eq!(tracker.get_dual_variable(node_1), 50);
        assert_eq!(tracker.get_dual_variable(node_2), 40);
        assert_eq!(tracker.get_maximum_growth(), Weight::MAX);

        tracker.set_speed(node_1, BlossomGrowState::Grow);
        tracker.set_speed(node_2, BlossomGrowState::Shrink);
        tracker.advance_time(10);
        assert_eq!(tracker.get_dual_variable(node_1), 60);
        assert_eq!(tracker.get_dual_variable(node_2), 30);
        assert_eq!(tracker.get_maximum_growth(), 30);

        tracker.advance_time(30);
        assert_eq!(tracker.get_dual_variable(node_1), 90);
        assert_eq!(tracker.get_dual_variable(node_2), 0);
        assert_eq!(tracker.get_maximum_growth(), 0);

        tracker.set_speed(node_2, BlossomGrowState::Grow);
        assert_eq!(tracker.get_maximum_growth(), Weight::MAX);

        tracker.set_speed(node_2, BlossomGrowState::Shrink);
        assert_eq!(tracker.get_maximum_growth(), 0);

        tracker.set_speed(node_2, BlossomGrowState::Grow);
        tracker.advance_time(30);
        tracker.set_speed(node_2, BlossomGrowState::Shrink);
        tracker.set_speed(node_2, BlossomGrowState::Grow);
        tracker.advance_time(30);
        tracker.set_speed(node_2, BlossomGrowState::Shrink);
        assert_eq!(tracker.get_maximum_growth(), 60);
    }
}
