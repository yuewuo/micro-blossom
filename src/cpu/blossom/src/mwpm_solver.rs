use crate::dual_module_rtl::*;
use crate::primal_module_embedded_adaptor::*;
use crate::util::*;
use fusion_blossom::dual_module::*;
use fusion_blossom::dual_module_serial::*;
use fusion_blossom::mwpm_solver::*;
use fusion_blossom::primal_module::*;
use fusion_blossom::primal_module_serial::*;
use fusion_blossom::util::*;
use fusion_blossom::visualize::*;
use micro_blossom_nostd::interface::*;
use micro_blossom_nostd::primal_module_embedded::*;
use serde_json::json;

pub struct SolverDualRTL {
    dual_module: DualModuleRTL,
    primal_module: PrimalModuleSerialPtr,
    interface_ptr: DualModuleInterfacePtr,
    subgraph_builder: SubGraphBuilder,
}

impl FusionVisualizer for SolverDualRTL {
    fn snapshot(&self, abbrev: bool) -> serde_json::Value {
        let mut value = self.primal_module.snapshot(abbrev);
        snapshot_combine_values(&mut value, self.dual_module.snapshot(abbrev), abbrev);
        snapshot_combine_values(&mut value, self.interface_ptr.snapshot(abbrev), abbrev);
        value
    }
}

impl SolverDualRTL {
    pub fn new(initializer: &SolverInitializer) -> Self {
        Self {
            dual_module: DualModuleRTL::new_empty(initializer),
            primal_module: PrimalModuleSerialPtr::new_empty(initializer),
            interface_ptr: DualModuleInterfacePtr::new_empty(),
            subgraph_builder: SubGraphBuilder::new(initializer),
        }
    }
}

impl PrimalDualSolver for SolverDualRTL {
    fn clear(&mut self) {
        self.primal_module.clear();
        self.dual_module.clear();
        self.interface_ptr.clear();
        self.subgraph_builder.clear();
    }
    fn solve_visualizer(&mut self, syndrome_pattern: &SyndromePattern, visualizer: Option<&mut Visualizer>) {
        if !syndrome_pattern.erasures.is_empty() {
            assert!(
                syndrome_pattern.dynamic_weights.is_empty(),
                "erasures and dynamic_weights cannot be provided at the same time"
            );
            self.subgraph_builder.load_erasures(&syndrome_pattern.erasures);
        }
        if !syndrome_pattern.dynamic_weights.is_empty() {
            self.subgraph_builder.load_dynamic_weights(&syndrome_pattern.dynamic_weights);
        }
        self.primal_module
            .solve_visualizer(&self.interface_ptr, syndrome_pattern, &mut self.dual_module, visualizer);
    }
    fn perfect_matching_visualizer(&mut self, visualizer: Option<&mut Visualizer>) -> PerfectMatching {
        let perfect_matching = self
            .primal_module
            .perfect_matching(&self.interface_ptr, &mut self.dual_module);
        if let Some(visualizer) = visualizer {
            visualizer
                .snapshot_combined(
                    "perfect matching".to_string(),
                    vec![&self.interface_ptr, &self.dual_module, &perfect_matching],
                )
                .unwrap();
        }
        perfect_matching
    }
    fn subgraph_visualizer(&mut self, visualizer: Option<&mut Visualizer>) -> Vec<EdgeIndex> {
        let perfect_matching = self.perfect_matching();
        self.subgraph_builder.load_perfect_matching(&perfect_matching);
        let subgraph = self.subgraph_builder.get_subgraph();
        if let Some(visualizer) = visualizer {
            visualizer
                .snapshot_combined(
                    "perfect matching and subgraph".to_string(),
                    vec![
                        &self.interface_ptr,
                        &self.dual_module,
                        &perfect_matching,
                        &VisualizeSubgraph::new(&subgraph),
                    ],
                )
                .unwrap();
        }
        subgraph
    }
    fn sum_dual_variables(&self) -> Weight {
        self.interface_ptr.read_recursive().sum_dual_variables
    }
    fn generate_profiler_report(&self) -> serde_json::Value {
        json!({
            "dual": self.dual_module.generate_profiler_report(),
            "primal": self.primal_module.generate_profiler_report(),
        })
    }
}

pub struct SolverPrimalEmbedded {
    dual_module: DualModuleSerial,
    primal_module: PrimalModuleEmbeddedAdaptor,
    interface_ptr: DualModuleInterfacePtr,
    subgraph_builder: SubGraphBuilder,
}

impl FusionVisualizer for SolverPrimalEmbedded {
    fn snapshot(&self, abbrev: bool) -> serde_json::Value {
        let mut value = self.primal_module.snapshot(abbrev);
        snapshot_combine_values(&mut value, self.dual_module.snapshot(abbrev), abbrev);
        snapshot_combine_values(&mut value, self.interface_ptr.snapshot(abbrev), abbrev);
        value
    }
}

impl SolverPrimalEmbedded {
    pub fn new(initializer: &SolverInitializer) -> Self {
        Self {
            dual_module: DualModuleSerial::new_empty(initializer),
            primal_module: PrimalModuleEmbeddedAdaptor::new_empty(initializer),
            interface_ptr: DualModuleInterfacePtr::new_empty(),
            subgraph_builder: SubGraphBuilder::new(initializer),
        }
    }
}

impl PrimalDualSolver for SolverPrimalEmbedded {
    fn clear(&mut self) {
        self.primal_module.clear();
        self.dual_module.clear();
        self.interface_ptr.clear();
        self.subgraph_builder.clear();
    }
    fn solve_visualizer(&mut self, syndrome_pattern: &SyndromePattern, visualizer: Option<&mut Visualizer>) {
        if !syndrome_pattern.erasures.is_empty() {
            assert!(
                syndrome_pattern.dynamic_weights.is_empty(),
                "erasures and dynamic_weights cannot be provided at the same time"
            );
            self.subgraph_builder.load_erasures(&syndrome_pattern.erasures);
        }
        if !syndrome_pattern.dynamic_weights.is_empty() {
            self.subgraph_builder.load_dynamic_weights(&syndrome_pattern.dynamic_weights);
        }
        self.primal_module
            .solve_visualizer(&self.interface_ptr, syndrome_pattern, &mut self.dual_module, visualizer);
    }
    fn perfect_matching_visualizer(&mut self, visualizer: Option<&mut Visualizer>) -> PerfectMatching {
        let perfect_matching = self
            .primal_module
            .perfect_matching(&self.interface_ptr, &mut self.dual_module);
        if let Some(visualizer) = visualizer {
            visualizer
                .snapshot_combined(
                    "perfect matching".to_string(),
                    vec![&self.interface_ptr, &self.dual_module, &perfect_matching],
                )
                .unwrap();
        }
        perfect_matching
    }
    fn subgraph_visualizer(&mut self, visualizer: Option<&mut Visualizer>) -> Vec<EdgeIndex> {
        let perfect_matching = self.perfect_matching();
        self.subgraph_builder.load_perfect_matching(&perfect_matching);
        let subgraph = self.subgraph_builder.get_subgraph();
        if let Some(visualizer) = visualizer {
            visualizer
                .snapshot_combined(
                    "perfect matching and subgraph".to_string(),
                    vec![
                        &self.interface_ptr,
                        &self.dual_module,
                        &perfect_matching,
                        &VisualizeSubgraph::new(&subgraph),
                    ],
                )
                .unwrap();
        }
        subgraph
    }
    fn sum_dual_variables(&self) -> Weight {
        self.interface_ptr.read_recursive().sum_dual_variables
    }
    fn generate_profiler_report(&self) -> serde_json::Value {
        json!({
            "dual": self.dual_module.generate_profiler_report(),
            "primal": self.primal_module.generate_profiler_report(),
        })
    }
}

pub struct SolverEmbeddedRTL {
    dual_module: DualModuleRTL,
    primal_module: PrimalModuleEmbedded<MAX_NODE_NUM, DOUBLE_MAX_NODE_NUM>,
    subgraph_builder: SubGraphBuilder,
}

impl FusionVisualizer for SolverEmbeddedRTL {
    fn snapshot(&self, abbrev: bool) -> serde_json::Value {
        let value = self.dual_module.snapshot(abbrev);
        // snapshot_combine_values(&mut value, self.dual_module.snapshot(abbrev), abbrev);
        value
    }
}

impl SolverEmbeddedRTL {
    pub fn new(initializer: &SolverInitializer) -> Self {
        Self {
            dual_module: DualModuleRTL::new_empty(initializer),
            primal_module: PrimalModuleEmbedded::new(),
            subgraph_builder: SubGraphBuilder::new(initializer),
        }
    }
}

impl PrimalDualSolver for SolverEmbeddedRTL {
    fn clear(&mut self) {
        self.primal_module.reset();
        self.dual_module.reset();
        self.subgraph_builder.clear();
    }
    fn solve_visualizer(&mut self, syndrome_pattern: &SyndromePattern, visualizer: Option<&mut Visualizer>) {
        solve_visualizer_embedded(&mut self.primal_module, &mut self.dual_module, syndrome_pattern, visualizer)
    }
    fn perfect_matching_visualizer(&mut self, _visualizer: Option<&mut Visualizer>) -> PerfectMatching {
        unimplemented!()
    }
    fn subgraph_visualizer(&mut self, visualizer: Option<&mut Visualizer>) -> Vec<EdgeIndex> {
        unimplemented!()
    }
    fn sum_dual_variables(&self) -> Weight {
        unimplemented!()
        // self.interface_ptr.read_recursive().sum_dual_variables
    }
    fn generate_profiler_report(&self) -> serde_json::Value {
        json!({
            "dual": self.dual_module.generate_profiler_report(),
            // "primal": self.primal_module.generate_profiler_report(),
        })
    }
}

fn solve_visualizer_embedded(
    primal_module: &mut impl PrimalInterface,
    dual_module: &mut (impl DualInterface + FusionVisualizer),
    syndrome_pattern: &SyndromePattern,
    mut visualizer: Option<&mut Visualizer>,
) {
    assert!(syndrome_pattern.erasures.is_empty());
    assert!(syndrome_pattern.dynamic_weights.is_empty());
    if let Some(visualizer) = visualizer.as_mut() {
        visualizer.snapshot_combined("solved".to_string(), vec![dual_module]).unwrap();
    }
    let (mut obstacle, _) = dual_module.find_obstacle();
    while !obstacle.is_none() {
        debug_assert!(
            obstacle.is_obstacle(),
            "dual module should spontaneously process all finite growth"
        );
        if let Some(visualizer) = visualizer.as_mut() {
            visualizer.snapshot_combined("solved".to_string(), vec![dual_module]).unwrap();
        }
        primal_module.resolve(dual_module, obstacle);
        (obstacle, _) = dual_module.find_obstacle();
    }
    if let Some(visualizer) = visualizer.as_mut() {
        visualizer.snapshot_combined("solved".to_string(), vec![dual_module]).unwrap();
    }
}
