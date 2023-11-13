//! A dual module implemented with combinatorial logic, for easy modeling of the real hardware
//!
//! This is a software implementation of the Micro Blossom algorithm.
//! We assume all the vertices and edges have a single clock source.
//! On every clock cycle, each vertex/edge generates a new vertex/edge as the register state for the next clock cycle.
//! This directly corresponds to an RTL design in HDL language, but without any optimizations.
//! This is supposed to be an algorithm design for Micro Blossom.
//!

use crate::dual_module_adaptor::*;
use crate::dual_module_comb_edge::*;
use crate::dual_module_comb_vertex::*;
use crate::util::*;
use fusion_blossom::util::*;
use fusion_blossom::visualize::*;
use micro_blossom_nostd::dual_driver_tracked::*;
use micro_blossom_nostd::dual_module_stackless::*;
use micro_blossom_nostd::interface::*;
use micro_blossom_nostd::util::*;
use serde_json::json;

pub struct DualModuleCombDriver {
    pub initializer: SolverInitializer,
    pub vertices: Vec<Vertex>,
    pub edges: Vec<Edge>,
    pub maximum_growth: Weight,
}

pub type DualModuleComb = DualModuleStackless<DualDriverTracked<DualModuleCombDriver, MAX_NODE_NUM>>;

impl DualInterfaceWithInitializer for DualModuleComb {
    fn new_with_initializer(initializer: &SolverInitializer) -> Self {
        DualModuleStackless::new(DualDriverTracked::new(DualModuleCombDriver::new_empty(initializer)))
    }
}

impl DualModuleCombDriver {
    pub fn new_empty(initializer: &SolverInitializer) -> Self {
        let mut behavior = Self {
            initializer: initializer.clone(),
            vertices: vec![],
            edges: vec![],
            maximum_growth: Weight::MAX,
        };
        behavior.clear();
        behavior
    }

    pub fn clear(&mut self) {
        unimplemented!()
    }
}

impl DualStacklessDriver for DualModuleCombDriver {
    fn reset(&mut self) {
        self.clear();
    }
    fn set_speed(&mut self, _is_blossom: bool, node: CompactNodeIndex, speed: CompactGrowState) {
        unimplemented!()
    }
    fn set_blossom(&mut self, node: CompactNodeIndex, blossom: CompactNodeIndex) {
        unimplemented!()
    }
    fn find_obstacle(&mut self) -> (CompactObstacle, CompactWeight) {
        unimplemented!()
    }
    fn add_defect(&mut self, vertex: CompactVertexIndex, node: CompactNodeIndex) {
        unimplemented!()
    }
}

impl DualTrackedDriver for DualModuleCombDriver {
    fn set_maximum_growth(&mut self, length: CompactWeight) {
        self.maximum_growth = length as Weight;
    }
}

impl FusionVisualizer for DualModuleCombDriver {
    #[allow(clippy::unnecessary_cast)]
    fn snapshot(&self, abbrev: bool) -> serde_json::Value {
        json!({})
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::dual_module_rtl::tests::*;
    use crate::mwpm_solver::*;
    use fusion_blossom::example_codes::*;
    use fusion_blossom::mwpm_solver::*;
    use fusion_blossom::primal_module::*;

    // to use visualization, we need the folder of fusion-blossom repo
    // e.g. export FUSION_DIR=/Users/wuyue/Documents/GitHub/fusion-blossom

    #[test]
    fn dual_module_comb_basic_1() {
        // cargo test dual_module_comb_basic_1 -- --nocapture
        let visualize_filename = "dual_module_comb_basic_1.json".to_string();
        let defect_vertices = vec![18, 26, 34];
        dual_module_comb_basic_standard_syndrome(7, visualize_filename, defect_vertices);
    }

    pub fn dual_module_comb_basic_standard_syndrome(
        d: VertexNum,
        visualize_filename: String,
        defect_vertices: Vec<VertexIndex>,
    ) -> SolverDualComb {
        dual_module_rtl_embedded_basic_standard_syndrome_optional_viz(
            d,
            Some(visualize_filename.clone()),
            defect_vertices,
            |initializer| SolverDualComb::new(initializer),
        )
    }
}
