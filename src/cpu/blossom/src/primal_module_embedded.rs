use crate::util::*;
use fusion_blossom::dual_module::*;
use fusion_blossom::primal_module::*;
use fusion_blossom::util::*;
use fusion_blossom::visualize::*;
use micro_blossom_nostd::primal_module_embedded::*;
use serde_json::json;

#[derive(Debug)]
pub struct PrimalModuleEmbeddedAdaptor {
    pub primal_module: PrimalModuleEmbedded<MAX_NODE_NUM, DOUBLE_MAX_NODE_NUM>,
}

impl PrimalModuleImpl for PrimalModuleEmbeddedAdaptor {
    fn new_empty(_initializer: &SolverInitializer) -> Self {
        Self {
            primal_module: PrimalModuleEmbedded::new(),
        }
    }

    fn clear(&mut self) {
        self.primal_module.clear();
    }

    fn load_defect_dual_node(&mut self, dual_node_ptr: &DualNodePtr) {
        unimplemented!();
    }

    fn resolve<D: DualModuleImpl>(
        &mut self,
        mut group_max_update_length: GroupMaxUpdateLength,
        interface_ptr: &DualModuleInterfacePtr,
        dual_module: &mut D,
    ) {
        unimplemented!();
    }

    fn intermediate_matching<D: DualModuleImpl>(
        &mut self,
        _interface: &DualModuleInterfacePtr,
        _dual_module: &mut D,
    ) -> IntermediateMatching {
        unimplemented!();
    }
}

impl FusionVisualizer for PrimalModuleEmbeddedAdaptor {
    fn snapshot(&self, abbrev: bool) -> serde_json::Value {
        json!({})
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fusion_blossom::dual_module_serial::*;
    use fusion_blossom::example_codes::*;

    // to use visualization, we need the folder of fusion-blossom repo
    // e.g. export FUSION_DIR=/Users/wuyue/Documents/GitHub/fusion-blossom

    /// test a simple blossom
    #[test]
    fn primal_module_embedded_basic_1() {
        // cargo test primal_module_embedded_basic_1 -- --nocapture
        let visualize_filename = "primal_module_serial_basic_1.json".to_string();
        let defect_vertices = vec![18, 26, 34];
        primal_module_embedded_basic_standard_syndrome(7, visualize_filename, defect_vertices, 4);
    }

    pub fn primal_module_embedded_basic_standard_syndrome_optional_viz(
        d: VertexNum,
        visualize_filename: Option<String>,
        defect_vertices: Vec<VertexIndex>,
        final_dual: Weight,
    ) -> (DualModuleInterfacePtr, PrimalModuleEmbeddedAdaptor, DualModuleSerial) {
        println!("{defect_vertices:?}");
        let half_weight = 500;
        let mut code = CodeCapacityPlanarCode::new(d, 0.1, half_weight);
        let mut visualizer = match visualize_filename.as_ref() {
            Some(visualize_filename) => {
                let visualizer = Visualizer::new(
                    option_env!("FUSION_DIR").map(|dir| dir.to_owned() + "/visualize/data/" + visualize_filename.as_str()),
                    code.get_positions(),
                    true,
                )
                .unwrap();
                print_visualize_link(visualize_filename.clone());
                Some(visualizer)
            }
            None => None,
        };
        // create dual module
        let initializer = code.get_initializer();
        let mut dual_module = DualModuleSerial::new_empty(&initializer);
        // create primal module
        let mut primal_module = PrimalModuleEmbeddedAdaptor::new_empty(&initializer);
        code.set_defect_vertices(&defect_vertices);
        let interface_ptr = DualModuleInterfacePtr::new_empty();
        primal_module.solve_visualizer(&interface_ptr, &code.get_syndrome(), &mut dual_module, visualizer.as_mut());
        let perfect_matching = primal_module.perfect_matching(&interface_ptr, &mut dual_module);
        let mut subgraph_builder = SubGraphBuilder::new(&initializer);
        subgraph_builder.load_perfect_matching(&perfect_matching);
        let subgraph = subgraph_builder.get_subgraph();
        if let Some(visualizer) = visualizer.as_mut() {
            visualizer
                .snapshot_combined(
                    "perfect matching and subgraph".to_string(),
                    vec![
                        &interface_ptr,
                        &dual_module,
                        &perfect_matching,
                        &VisualizeSubgraph::new(&subgraph),
                    ],
                )
                .unwrap();
        }
        assert_eq!(
            interface_ptr.sum_dual_variables(),
            subgraph_builder.total_weight(),
            "unmatched sum dual variables"
        );
        assert_eq!(
            interface_ptr.sum_dual_variables(),
            final_dual * 2 * half_weight,
            "unexpected final dual variable sum"
        );
        (interface_ptr, primal_module, dual_module)
    }

    pub fn primal_module_embedded_basic_standard_syndrome(
        d: VertexNum,
        visualize_filename: String,
        defect_vertices: Vec<VertexIndex>,
        final_dual: Weight,
    ) -> (DualModuleInterfacePtr, PrimalModuleEmbeddedAdaptor, DualModuleSerial) {
        primal_module_embedded_basic_standard_syndrome_optional_viz(d, Some(visualize_filename), defect_vertices, final_dual)
    }
}
