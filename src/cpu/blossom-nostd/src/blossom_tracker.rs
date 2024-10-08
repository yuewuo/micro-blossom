//! Blossom Tracker
//!
//! This is a software implementation of the obstacle detection of negative blossom shrinking.
//! It's supposed to only track all blossoms but not single vertices, to save memory.
//! This module needs to be called whenever a blossom is created or set speed.
//! A global step variable needs to be provided so that this module know what is the current dual value.
//!

use crate::heapless::binary_heap::{BinaryHeap, Min};
use crate::heapless::Vec;
use crate::util::*;
use core::cmp::Ordering;
#[cfg(any(test, feature = "std"))]
use derivative::Derivative;

// We need to maintain information about the blossoms, e.g., the dual variables of them.
// The blossom indices have nice property that they will never decreasing.
// In fact, the indices are allocated linearly, meaning it's guaranteed that the next index after K is always K+1.
// Utilizing this, we can reduce the memory usage of the mapping significantly.
#[cfg_attr(any(test, feature = "std"), derive(Derivative))]
#[cfg_attr(any(test, feature = "std"), derivative(Debug))]
pub struct BlossomTracker<const N: usize> {
    /// the priority queue of next timestamp
    #[cfg_attr(any(test, feature = "std"), derivative(Debug(format_with = "binary_heap_debug_formatter")))]
    hit_zero_events: BinaryHeap<HitZeroEvent, Min, N>,
    /// it is the responsibility of outer program to report the timestamp properly
    timestamp: CompactTimestamp,
    /// the index of the first blossom, meaningless when length=0
    first_index: CompactNodeIndex,
    /// the checkpoints of dual variables
    checkpoints: Vec<(CompactTimestamp, CompactWeight), N>,
    /// speed of the blossom
    grow_states: Vec<CompactGrowState, N>,
}

#[derive(Debug)]
struct HitZeroEvent {
    timestamp: CompactTimestamp,
    /// the node that *probably* hits zero; it's probable because we never delete such event from the queue
    node_index: CompactNodeIndex,
}

#[cfg(any(test, feature = "std"))]
fn binary_heap_debug_formatter<const N: usize, T: std::fmt::Debug + Ord>(
    binary_heap: &BinaryHeap<T, Min, N>,
    formatter: &mut std::fmt::Formatter,
) -> Result<(), std::fmt::Error> {
    formatter
        .debug_struct("BinaryHeap")
        .field("top", &binary_heap.peek())
        .field("len", &binary_heap.len())
        .finish()
}

impl<const N: usize> BlossomTracker<N> {
    pub const fn new() -> Self {
        Self {
            hit_zero_events: BinaryHeap::new(),
            timestamp: 0,
            first_index: match CompactNodeIndex::new(0).option() {
                Some(index) => index,
                None => unreachable!(),
            },
            checkpoints: Vec::new(),
            grow_states: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        self.hit_zero_events.clear();
        self.checkpoints.clear();
        self.grow_states.clear();
    }

    #[inline(always)]
    pub fn advance_time(&mut self, delta: CompactTimestamp) {
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
    fn assert_valid_node_index(&self, node_index: CompactNodeIndex) {
        debug_assert!(
            node_index.get() >= self.first_index.get()
                && node_index.get() < self.first_index.get() + self.checkpoints.len() as CompactNodeNum,
            "invalid node index {}, not within the range of [{}, {})",
            node_index,
            self.first_index,
            self.first_index.get() + self.checkpoints.len() as CompactNodeNum
        );
    }

    #[inline]
    fn local_index_of(&self, node_index: CompactNodeIndex) -> usize {
        self.assert_valid_node_index(node_index);
        (node_index.get() - self.first_index.get()) as usize
    }

    pub fn create_blossom(&mut self, node_index: CompactNodeIndex) {
        debug_assert!(
            self.checkpoints.is_empty()
                || node_index.get() == self.first_index.get() + self.checkpoints.len() as CompactNodeNum
        );
        if self.checkpoints.is_empty() {
            self.first_index = node_index;
        }
        self.checkpoints.push((self.timestamp, 0)).ok().unwrap();
        self.grow_states.push(CompactGrowState::Grow).ok().unwrap();
    }

    pub fn set_speed(&mut self, node_index: CompactNodeIndex, grow_state: CompactGrowState) {
        let local_index = self.local_index_of(node_index);
        // update checkpoint timestamp to the current timestamp and update its dual value accordingly
        if &grow_state == get!(self.grow_states, local_index) {
            return; // no need to set speed
        }
        let dual_value = self.local_get_dual_variable(local_index);
        set!(self.checkpoints, local_index, (self.timestamp, dual_value));
        set!(self.grow_states, local_index, grow_state);
        // insert a hit-zero event if the blossom becomes shrinking
        if grow_state == CompactGrowState::Shrink {
            self.hit_zero_events
                .push(HitZeroEvent {
                    timestamp: self.timestamp + dual_value as CompactTimestamp,
                    node_index,
                })
                .ok()
                .unwrap();
        }
    }

    fn local_get_dual_variable(&self, local_index: usize) -> CompactWeight {
        let (timestamp, dual_value) = *get!(self.checkpoints, local_index);
        let delta = (self.timestamp - timestamp) as CompactWeight;
        let dual_value = match *get!(self.grow_states, local_index) {
            CompactGrowState::Grow => dual_value + delta,
            CompactGrowState::Shrink => dual_value - delta,
            CompactGrowState::Stay => dual_value,
        };
        debug_assert!(dual_value >= 0);
        dual_value
    }

    pub fn get_dual_variable(&self, node_index: CompactNodeIndex) -> CompactWeight {
        self.local_get_dual_variable(self.local_index_of(node_index))
    }

    #[inline]
    fn is_valid_event(&self, first_event: &HitZeroEvent) -> bool {
        let local_index = self.local_index_of(first_event.node_index);
        if self.grow_states[local_index] == CompactGrowState::Shrink {
            let dual_value = self.local_get_dual_variable(local_index);
            let actual_timestamp = self.timestamp + dual_value as CompactTimestamp;
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

    #[inline(always)]
    fn remove_outdated_events(&mut self) {
        while !self.hit_zero_events.is_empty() {
            if self.is_valid_event(self.hit_zero_events.peek().unwrap()) {
                return;
            }
            self.hit_zero_events.pop().unwrap();
        }
    }

    #[inline(always)]
    pub fn get_maximum_growth(&mut self) -> Option<(CompactWeight, CompactNodeIndex)> {
        self.remove_outdated_events();
        self.hit_zero_events.peek().map(|event| {
            debug_assert!(event.timestamp >= self.timestamp);
            ((event.timestamp - self.timestamp) as CompactWeight, event.node_index)
        })
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
        let blossom_bias: CompactNodeIndex = ni!(0x1100);
        let node_1 = blossom_bias;
        let node_2 = ni!(blossom_bias.get() + 1);

        tracker.create_blossom(node_1);
        assert_eq!(tracker.get_dual_variable(node_1), 0);
        assert_eq!(tracker.get_maximum_growth(), None);

        tracker.advance_time(20);
        assert_eq!(tracker.get_dual_variable(node_1), 20);
        assert_eq!(tracker.get_maximum_growth(), None);

        tracker.create_blossom(node_2);
        tracker.advance_time(30);
        assert_eq!(tracker.get_dual_variable(node_1), 50);
        assert_eq!(tracker.get_dual_variable(node_2), 30);
        assert_eq!(tracker.get_maximum_growth(), None);

        tracker.set_speed(node_1, CompactGrowState::Stay);
        tracker.advance_time(10);
        assert_eq!(tracker.get_dual_variable(node_1), 50);
        assert_eq!(tracker.get_dual_variable(node_2), 40);
        assert_eq!(tracker.get_maximum_growth(), None);

        tracker.set_speed(node_1, CompactGrowState::Grow);
        tracker.set_speed(node_2, CompactGrowState::Shrink);
        tracker.advance_time(10);
        assert_eq!(tracker.get_dual_variable(node_1), 60);
        assert_eq!(tracker.get_dual_variable(node_2), 30);
        assert_eq!(tracker.get_maximum_growth(), Some((30, node_2)));

        tracker.advance_time(30);
        assert_eq!(tracker.get_dual_variable(node_1), 90);
        assert_eq!(tracker.get_dual_variable(node_2), 0);
        assert_eq!(tracker.get_maximum_growth(), Some((0, node_2)));

        tracker.set_speed(node_2, CompactGrowState::Grow);
        assert_eq!(tracker.get_maximum_growth(), None);

        tracker.set_speed(node_2, CompactGrowState::Shrink);
        assert_eq!(tracker.get_maximum_growth(), Some((0, node_2)));

        tracker.set_speed(node_2, CompactGrowState::Grow);
        tracker.advance_time(30);
        tracker.set_speed(node_2, CompactGrowState::Shrink);
        tracker.set_speed(node_2, CompactGrowState::Grow);
        tracker.advance_time(30);
        tracker.set_speed(node_2, CompactGrowState::Shrink);
        assert_eq!(tracker.get_maximum_growth(), Some((60, node_2)));
    }
}
