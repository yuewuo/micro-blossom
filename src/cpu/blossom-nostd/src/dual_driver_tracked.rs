//! Dual Driver Tracked
//!
//! a dual driver that handles the event of blossom hitting zero in software,
//! while passing through all the other functionalities to another driver
//!

use crate::blossom_tracker::*;
use crate::dual_module_stackless::*;
use crate::interface::*;
use crate::util::*;

pub trait DualTrackedDriver {
    /// with `DualDriverTracked`, the driver doesn't need to report any `BlossomNeedExpand` obstacles.
    /// the external driver should not grow more than this value before returning, to accommodate with this offloading.
    fn set_maximum_growth(&mut self, length: CompactWeight);
}

pub struct DualDriverTracked<D: DualStacklessDriver + DualTrackedDriver, const N: usize> {
    pub driver: D,
    pub blossom_tracker: BlossomTracker<N>,
}

impl<D: DualStacklessDriver + DualTrackedDriver, const N: usize> DualStacklessDriver for DualDriverTracked<D, N> {
    fn reset(&mut self) {
        self.driver.reset();
        self.driver.set_maximum_growth(CompactWeight::MAX);
        self.blossom_tracker.clear();
    }

    fn set_speed(&mut self, is_blossom: bool, node: CompactNodeIndex, speed: CompactGrowState) {
        self.driver.set_speed(is_blossom, node, speed);
        if is_blossom {
            self.blossom_tracker.set_speed(node, speed);
            if speed == CompactGrowState::Shrink {
                // prevent the driver to over grow
                if let Some((length, _blossom)) = self.blossom_tracker.get_maximum_growth() {
                    self.driver.set_maximum_growth(length);
                }
            }
        }
    }

    fn on_blossom_created(&mut self, blossom: CompactNodeIndex) {
        self.blossom_tracker.create_blossom(blossom);
    }

    fn on_blossom_expanded(&mut self, blossom: CompactNodeIndex) {
        self.blossom_tracker.set_speed(blossom, CompactGrowState::Stay);
    }

    fn on_blossom_absorbed_into_blossom(&mut self, child: CompactNodeIndex) {
        self.blossom_tracker.set_speed(child, CompactGrowState::Stay);
    }

    fn set_blossom(&mut self, node: CompactNodeIndex, blossom: CompactNodeIndex) {
        self.driver.set_blossom(node, blossom);
    }

    fn find_obstacle(&mut self) -> (CompactObstacle, CompactWeight) {
        let (mut obstacle, mut grown) = self.driver.find_obstacle();
        self.blossom_tracker.advance_time(grown as CompactTimestamp);
        while obstacle.is_finite_growth() {
            if let Some((length, blossom)) = self.blossom_tracker.get_maximum_growth() {
                if length == 0 {
                    return (CompactObstacle::BlossomNeedExpand { blossom }, grown);
                } else {
                    self.driver.set_maximum_growth(length);
                }
            } else {
                self.driver.set_maximum_growth(CompactWeight::MAX);
            }
            let (inc_obstacle, inc_grown) = self.driver.find_obstacle();
            self.blossom_tracker.advance_time(inc_grown as CompactTimestamp);
            obstacle = inc_obstacle;
            grown += inc_grown;
        }
        (obstacle, grown)
    }

    fn add_defect(&mut self, vertex: CompactVertexIndex, node: CompactNodeIndex) {
        self.driver.add_defect(vertex, node);
    }
}

impl<D: DualStacklessDriver + DualTrackedDriver, const N: usize> DualDriverTracked<D, N> {
    pub fn new(driver: D) -> Self {
        Self {
            driver,
            blossom_tracker: BlossomTracker::new(),
        }
    }
}
