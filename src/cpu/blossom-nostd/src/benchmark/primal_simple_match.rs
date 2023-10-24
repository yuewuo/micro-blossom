//! Benchmark of Primal Simple Match
//!
//! This module contains several functions that
//!

use crate::benchmark::dual_module_counter::*;
use crate::dual_module_stackless::*;
use crate::interface::*;
use crate::primal_module_embedded::*;
use crate::util::*;

pub struct PrimalSimpleMatch<const MAX_NODE_NUM: usize, const DOUBLE_MAX_NODE_NUM: usize> {
    primal_module: PrimalModuleEmbedded<MAX_NODE_NUM, DOUBLE_MAX_NODE_NUM>,
    dual_module: DualModuleStackless<DualModuleCounterDriver>,
}

impl<const MAX_NODE_NUM: usize, const DOUBLE_MAX_NODE_NUM: usize> PrimalSimpleMatch<MAX_NODE_NUM, DOUBLE_MAX_NODE_NUM> {
    pub fn new() -> Self {
        Self {
            primal_module: PrimalModuleEmbedded::new(),
            dual_module: DualModuleStackless::new(DualModuleCounterDriver::new()),
        }
    }

    pub fn run(&mut self, count: usize) {
        assert!(count * 2 < MAX_NODE_NUM);
        let mut index = 0;
        for _ in 0..count {
            self.primal_module.resolve(
                &mut self.dual_module,
                MaxUpdateLength::Conflict {
                    node_1: ni!(index),
                    node_2: Some(ni!(index + 1)),
                    touch_1: ni!(index),
                    touch_2: Some(ni!(index + 1)),
                    vertex_1: ni!(123),
                    vertex_2: ni!(234),
                },
            );
            index += 2;
        }
    }

    pub fn clear(&mut self) {
        self.primal_module.clear();
        self.dual_module.clear();
    }
}
