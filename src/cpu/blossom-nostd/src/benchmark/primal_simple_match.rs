//! Benchmark of Primal Simple Match
//!
//! This module contains several functions that
//!

use crate::benchmark::dual_module_counter::*;
use crate::dual_module_stackless::*;
use crate::interface::*;
use crate::primal_module_embedded::*;
use crate::util::*;

pub struct PrimalSimpleMatch<const MAX_NODE_NUM: usize> {
    pub primal_module: PrimalModuleEmbedded<MAX_NODE_NUM>,
    pub dual_module: DualModuleStackless<DualModuleCounterDriver>,
}

impl<const MAX_NODE_NUM: usize> PrimalSimpleMatch<MAX_NODE_NUM> {
    pub const fn new() -> Self {
        Self {
            primal_module: PrimalModuleEmbedded::new(),
            dual_module: DualModuleStackless::new(DualModuleCounterDriver::new()),
        }
    }

    pub fn run(&mut self, count: usize) {
        debug_assert!(count * 2 <= MAX_NODE_NUM);
        let mut index = 0;
        for _ in 0..count {
            self.primal_module.resolve(
                &mut self.dual_module,
                CompactObstacle::Conflict {
                    node_1: OptionCompactNodeIndex::new(index),
                    node_2: OptionCompactNodeIndex::new(index + 1),
                    touch_1: OptionCompactNodeIndex::new(index),
                    touch_2: OptionCompactNodeIndex::new(index + 1),
                    vertex_1: ni!(123),
                    vertex_2: ni!(234),
                },
            );
            index += 2;
        }
    }

    pub fn reset(&mut self) {
        self.primal_module.reset();
        self.dual_module.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn benchmark_primal_simple_match_basic() {
        // cargo test benchmark_primal_simple_match_basic -- --nocapture
        const N: usize = 128;
        let mut tester: PrimalSimpleMatch<N> = PrimalSimpleMatch::new();
        for _ in 0..3 {
            tester.run(N / 2);
            println!("count_set_speed: {}", tester.dual_module.driver.count_set_speed);
            println!("count_set_blossom: {}", tester.dual_module.driver.count_set_blossom);
            tester.reset();
        }
    }
}
