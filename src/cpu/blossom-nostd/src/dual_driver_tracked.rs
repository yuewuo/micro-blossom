//! Dual Driver Tracked
//!
//! a dual driver that handles the event of blossom hitting zero in software,
//! while passing through all the other functionalities to another driver
//!

use crate::blossom_tracker::*;
use crate::dual_module_stackless::*;
use crate::interface::*;
use crate::util::*;

struct DualDriverTracked<D: DualStacklessDriver, const N: usize> {
    pub driver: D,
    pub blossom_tracker: BlossomTracker<N>,
}

impl<D: DualStacklessDriver, const N: usize> DualStacklessDriver for DualDriverTracked<D, N> {
    fn clear(&mut self) {
        self.driver.clear();
        self.blossom_tracker.clear();
    }

    fn set_speed(&mut self, is_blossom: bool, node: CompactNodeIndex, speed: CompactGrowState) {
        self.driver.set_speed(is_blossom, node, speed);
        if is_blossom {
            self.blossom_tracker.set_speed(node, speed);
        }
    }

    fn on_blossom_created(&mut self, blossom: CompactNodeIndex) {
        self.blossom_tracker.create_blossom(blossom);
    }

    fn set_blossom(&mut self, node: CompactNodeIndex, blossom: CompactNodeIndex) {
        self.driver.set_blossom(node, blossom);
    }

    fn find_obstacle(&mut self) -> MaxUpdateLength {
        let mut max_update_length = self.driver.find_obstacle();
        if matches!(max_update_length, MaxUpdateLength::GrowLength { .. } | MaxUpdateLength::None) {
            if let Some((length, blossom)) = self.blossom_tracker.get_maximum_growth() {
                if length == 0 {
                    max_update_length = MaxUpdateLength::BlossomNeedExpand { blossom };
                } else {
                    if let MaxUpdateLength::GrowLength { length: original_length } = &mut max_update_length {
                        *original_length = std::cmp::min(*original_length, length)
                    } else {
                        max_update_length = MaxUpdateLength::GrowLength { length }
                    }
                }
            }
        }
        max_update_length
    }

    fn grow(&mut self, length: CompactWeight) {
        self.driver.grow(length);
        self.blossom_tracker.advance_time(length as CompactTimestamp);
    }
}
