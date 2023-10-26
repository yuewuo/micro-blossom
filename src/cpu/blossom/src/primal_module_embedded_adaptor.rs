use crate::util::*;
use fusion_blossom::dual_module::*;
use fusion_blossom::primal_module::*;
use fusion_blossom::util::*;
use fusion_blossom::visualize::*;
use micro_blossom_nostd::interface::*;
use micro_blossom_nostd::primal_module_embedded::*;
use micro_blossom_nostd::util::*;
use serde_json::json;
use std::collections::BTreeMap;

#[derive(Debug)]
pub struct PrimalModuleEmbeddedAdaptor {
    /// the embedded primal module
    pub primal_module: PrimalModuleEmbedded<MAX_NODE_NUM, DOUBLE_MAX_NODE_NUM>,
    /// mapping between the integer index interface and the pointer interface
    pub index_to_ptr: BTreeMap<CompactNodeIndex, DualNodePtr>,
    pub ptr_to_index: BTreeMap<DualNodePtr, CompactNodeIndex>,
    /// debug mode: only resolve one conflict each time
    pub debug_resolve_only_one: bool,
}

/// mocking the interface of the embedded primal module
pub struct MockDualInterface<'a, D: DualModuleImpl> {
    index_to_ptr: &'a mut BTreeMap<CompactNodeIndex, DualNodePtr>,
    ptr_to_index: &'a mut BTreeMap<DualNodePtr, CompactNodeIndex>,
    interface_ptr: &'a DualModuleInterfacePtr,
    dual_module: &'a mut D,
}

impl<'a, D: DualModuleImpl> DualInterface for MockDualInterface<'a, D> {
    fn clear(&mut self) {
        #[cfg(all(test, debug_assertions))]
        println!("[dual] clear()");
        unreachable!("should not be called")
    }
    fn create_blossom(&mut self, primal_module: &impl PrimalInterface, blossom_index: CompactNodeIndex) {
        let mut nodes_circle = vec![];
        let mut links = vec![]; // the format is different in embedded primal to easy programming
        primal_module.iterate_blossom_children(blossom_index, |_primal_module, child_index, link| {
            nodes_circle.push(self.index_to_ptr.get(&child_index).unwrap().clone());
            links.push((
                self.index_to_ptr.get(&link.touch.unwrap()).unwrap().clone().downgrade(),
                self.index_to_ptr.get(&link.peer_touch.unwrap()).unwrap().clone().downgrade(),
            ));
        });
        #[cfg(all(test, debug_assertions))]
        println!("[dual] create_blossom({blossom_index}) (nodes_circle: {nodes_circle:?})");
        debug_assert!(nodes_circle.len() % 2 == 1, "must be an odd cycle");
        debug_assert!(nodes_circle.len() > 1, "must be a cycle of at least 3 nodes");
        let mut touching_children = vec![];
        let length = links.len();
        for i in 0..length {
            let left_touching = if i == 0 {
                links[length - 1].1.clone()
            } else {
                links[i - 1].1.clone()
            };
            touching_children.push((left_touching, links[i].0.clone()))
        }
        let blossom_node_ptr = self
            .interface_ptr
            .create_blossom(nodes_circle, touching_children, self.dual_module);
        self.ptr_to_index.insert(blossom_node_ptr.clone(), blossom_index);
        self.index_to_ptr.insert(blossom_index, blossom_node_ptr);
    }
    fn expand_blossom(&mut self, _primal_module: &impl PrimalInterface, blossom_index: CompactNodeIndex) {
        #[cfg(all(test, debug_assertions))]
        println!("[dual] expand_blossom({blossom_index})");
        self.interface_ptr
            .expand_blossom(self.index_to_ptr.get(&blossom_index).unwrap().clone(), self.dual_module);
    }
    fn set_grow_state(&mut self, node_index: CompactNodeIndex, grow_state: CompactGrowState) {
        #[cfg(all(test, debug_assertions))]
        println!("[dual] set_grow_state({node_index}, {grow_state:?})");
        self.interface_ptr.set_grow_state(
            self.index_to_ptr.get(&node_index).unwrap(),
            match grow_state {
                CompactGrowState::Grow => DualNodeGrowState::Grow,
                CompactGrowState::Shrink => DualNodeGrowState::Shrink,
                CompactGrowState::Stay => DualNodeGrowState::Stay,
            },
            self.dual_module,
        );
    }
    fn compute_maximum_update_length(&mut self) -> micro_blossom_nostd::interface::MaxUpdateLength {
        #[cfg(all(test, debug_assertions))]
        println!("[dual] compute_maximum_update_length()");
        unreachable!("should not be called")
    }
    fn grow(&mut self, _length: CompactWeight) {
        #[cfg(all(test, debug_assertions))]
        println!("[dual] grow(length)");
        unreachable!("should not be called")
    }
}

impl PrimalModuleImpl for PrimalModuleEmbeddedAdaptor {
    fn new_empty(_initializer: &SolverInitializer) -> Self {
        Self {
            primal_module: PrimalModuleEmbedded::new(),
            index_to_ptr: BTreeMap::new(),
            ptr_to_index: BTreeMap::new(),
            debug_resolve_only_one: true,
        }
    }

    fn clear(&mut self) {
        self.primal_module.clear();
        self.index_to_ptr.clear();
        self.ptr_to_index.clear();
    }

    fn load_defect_dual_node(&mut self, dual_node_ptr: &DualNodePtr) {
        let node = dual_node_ptr.read_recursive();
        debug_assert!(matches!(node.class, DualNodeClass::DefectVertex { .. }));
        debug_assert!(node.index == self.ptr_to_index.len());
        self.ptr_to_index.insert(dual_node_ptr.clone(), ni!(node.index));
        self.index_to_ptr.insert(ni!(node.index), dual_node_ptr.clone());
        // there is no need to notify embedded primal module, since it's capable of automatically handling it
    }

    fn resolve<D: DualModuleImpl>(
        &mut self,
        mut group_max_update_length: GroupMaxUpdateLength,
        interface_ptr: &DualModuleInterfacePtr,
        dual_module: &mut D,
    ) {
        debug_assert!(!group_max_update_length.is_empty() && group_max_update_length.get_none_zero_growth().is_none());
        let mut current_conflict_index = 0;
        let debug_resolve_only_one = self.debug_resolve_only_one;
        while let Some(conflict) = group_max_update_length.pop() {
            current_conflict_index += 1;
            if debug_resolve_only_one && current_conflict_index > 1 {
                break;
            }
            if matches!(
                conflict,
                fusion_blossom::dual_module::MaxUpdateLength::VertexShrinkStop { .. }
            ) {
                continue; // there is no need to handle it
            }
            let adapted_conflict = match conflict {
                fusion_blossom::dual_module::MaxUpdateLength::Conflicting((node_1, touch_1), (node_2, touch_2)) => {
                    micro_blossom_nostd::interface::MaxUpdateLength::Conflict {
                        node_1: *self.ptr_to_index.get(&node_1).unwrap(),
                        node_2: Some(*self.ptr_to_index.get(&node_2).unwrap()),
                        touch_1: *self.ptr_to_index.get(&touch_1).unwrap(),
                        touch_2: Some(*self.ptr_to_index.get(&touch_2).unwrap()),
                        vertex_1: ni!(0),
                        vertex_2: ni!(0),
                    }
                }
                fusion_blossom::dual_module::MaxUpdateLength::TouchingVirtual(
                    (node, touch),
                    (virtual_vertex, _is_mirror),
                ) => micro_blossom_nostd::interface::MaxUpdateLength::Conflict {
                    node_1: *self.ptr_to_index.get(&node).unwrap(),
                    node_2: None,
                    touch_1: *self.ptr_to_index.get(&touch).unwrap(),
                    touch_2: None,
                    vertex_1: ni!(0),
                    vertex_2: ni!(virtual_vertex),
                },
                fusion_blossom::dual_module::MaxUpdateLength::BlossomNeedExpand(blossom_node) => {
                    micro_blossom_nostd::interface::MaxUpdateLength::BlossomNeedExpand {
                        blossom: *self.ptr_to_index.get(&blossom_node).unwrap(),
                    }
                }
                _ => unimplemented!(),
            };
            #[cfg(all(test, debug_assertions))]
            println!("[primal] resolve({:?})", adapted_conflict);
            self.primal_module.resolve(
                &mut MockDualInterface {
                    index_to_ptr: &mut self.index_to_ptr,
                    ptr_to_index: &mut self.ptr_to_index,
                    interface_ptr,
                    dual_module,
                },
                adapted_conflict,
            );
        }
    }

    fn intermediate_matching<D: DualModuleImpl>(
        &mut self,
        _interface: &DualModuleInterfacePtr,
        _dual_module: &mut D,
    ) -> IntermediateMatching {
        let mut intermediate_matching = IntermediateMatching::new();
        self.primal_module.iterate_intermediate_matching(
            |_primal_module, node_index, match_target, link| match match_target {
                CompactMatchTarget::Peer(peer_index) => intermediate_matching.peer_matchings.push((
                    (
                        self.index_to_ptr.get(&node_index).unwrap().clone(),
                        self.index_to_ptr.get(&link.touch.unwrap()).unwrap().clone().downgrade(),
                    ),
                    (
                        self.index_to_ptr.get(&peer_index).unwrap().clone(),
                        self.index_to_ptr.get(&link.peer_touch.unwrap()).unwrap().clone().downgrade(),
                    ),
                )),
                CompactMatchTarget::VirtualVertex(virtual_vertex) => {
                    intermediate_matching.virtual_matchings.push((
                        (
                            self.index_to_ptr.get(&node_index).unwrap().clone(),
                            self.index_to_ptr.get(&link.touch.unwrap()).unwrap().clone().downgrade(),
                        ),
                        virtual_vertex.get() as VertexIndex,
                    ));
                }
            },
        );
        intermediate_matching
    }

    // fn perfect_matching<D: DualModuleImpl>(
    //     &mut self,
    //     _interface: &DualModuleInterfacePtr,
    //     _dual_module: &mut D,
    // ) -> PerfectMatching {
    //     let mut perfect_matching = PerfectMatching::new();
    //     self.primal_module
    //         .iterate_perfect_matching(|_primal_module, node_index, match_target, _link| match match_target {
    //             CompactMatchTarget::Peer(peer_index) => perfect_matching.peer_matchings.push((
    //                 self.index_to_ptr.get(&node_index).unwrap().clone(),
    //                 self.index_to_ptr.get(&peer_index).unwrap().clone(),
    //             )),
    //             CompactMatchTarget::VirtualVertex(virtual_vertex) => {
    //                 perfect_matching.virtual_matchings.push((
    //                     self.index_to_ptr.get(&node_index).unwrap().clone(),
    //                     virtual_vertex.get() as VertexIndex,
    //                 ));
    //             }
    //         });
    //     perfect_matching
    // }
}

impl FusionVisualizer for PrimalModuleEmbeddedAdaptor {
    fn snapshot(&self, _abbrev: bool) -> serde_json::Value {
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

    /// test a free node conflict with a virtual boundary
    #[test]
    fn primal_module_embedded_basic_2() {
        // cargo test primal_module_embedded_basic_2 -- --nocapture
        let visualize_filename = "primal_module_embedded_basic_2.json".to_string();
        let defect_vertices = vec![16];
        primal_module_embedded_basic_standard_syndrome(7, visualize_filename, defect_vertices, 1);
    }

    /// test a free node conflict with a matched node (with virtual boundary)
    #[test]
    fn primal_module_embedded_basic_3() {
        // cargo test primal_module_embedded_basic_3 -- --nocapture
        let visualize_filename = "primal_module_embedded_basic_3.json".to_string();
        let defect_vertices = vec![16, 26];
        primal_module_embedded_basic_standard_syndrome(7, visualize_filename, defect_vertices, 3);
    }

    /// test blossom shrinking and expanding
    #[test]
    fn primal_module_embedded_basic_4() {
        // cargo test primal_module_embedded_basic_4 -- --nocapture
        let visualize_filename = "primal_module_embedded_basic_4.json".to_string();
        let defect_vertices = vec![16, 52, 65, 76, 112];
        primal_module_embedded_basic_standard_syndrome(11, visualize_filename, defect_vertices, 10);
    }

    /// test blossom conflicts with vertex
    #[test]
    fn primal_module_embedded_basic_5() {
        // cargo test primal_module_embedded_basic_5 -- --nocapture
        let visualize_filename = "primal_module_embedded_basic_5.json".to_string();
        let defect_vertices = vec![39, 51, 61, 62, 63, 64, 65, 75, 87, 67];
        primal_module_embedded_basic_standard_syndrome(11, visualize_filename, defect_vertices, 6);
    }

    /// test cascaded blossom
    #[test]
    fn primal_module_embedded_basic_6() {
        // cargo test primal_module_embedded_basic_6 -- --nocapture
        let visualize_filename = "primal_module_embedded_basic_6.json".to_string();
        let defect_vertices = vec![39, 51, 61, 62, 63, 64, 65, 75, 87];
        primal_module_embedded_basic_standard_syndrome(11, visualize_filename, defect_vertices, 6);
    }

    /// test two alternating trees conflict with each other
    #[test]
    fn primal_module_embedded_basic_7() {
        // cargo test primal_module_embedded_basic_7 -- --nocapture
        let visualize_filename = "primal_module_embedded_basic_7.json".to_string();
        let defect_vertices = vec![37, 61, 63, 66, 68, 44];
        primal_module_embedded_basic_standard_syndrome(11, visualize_filename, defect_vertices, 7);
    }

    /// test an alternating tree touches a virtual boundary
    #[test]
    fn primal_module_embedded_basic_8() {
        // cargo test primal_module_embedded_basic_8 -- --nocapture
        let visualize_filename = "primal_module_embedded_basic_8.json".to_string();
        let defect_vertices = vec![61, 64, 67];
        primal_module_embedded_basic_standard_syndrome(11, visualize_filename, defect_vertices, 5);
    }

    /// test a matched node (with virtual boundary) conflicts with an alternating tree
    #[test]
    fn primal_module_embedded_basic_9() {
        // cargo test primal_module_embedded_basic_9 -- --nocapture
        let visualize_filename = "primal_module_embedded_basic_9.json".to_string();
        let defect_vertices = vec![60, 63, 66, 30];
        primal_module_embedded_basic_standard_syndrome(11, visualize_filename, defect_vertices, 6);
    }

    /// test the error pattern in the paper
    #[test]
    fn primal_module_embedded_basic_10() {
        // cargo test primal_module_embedded_basic_10 -- --nocapture
        let visualize_filename = "primal_module_embedded_basic_10.json".to_string();
        let defect_vertices = vec![39, 52, 63, 90, 100];
        primal_module_embedded_basic_standard_syndrome(11, visualize_filename, defect_vertices, 9);
    }

    /// test complex random case
    #[test]
    fn primal_module_embedded_basic_11() {
        // cargo test primal_module_embedded_basic_11 -- --nocapture
        let visualize_filename = "primal_module_embedded_basic_11.json".to_string();
        let defect_vertices = vec![
            13, 29, 52, 53, 58, 60, 71, 74, 76, 87, 96, 107, 112, 118, 121, 122, 134, 137, 141, 145, 152, 153, 154, 156,
            157, 169, 186, 202, 203, 204, 230, 231,
        ];
        primal_module_embedded_basic_standard_syndrome(15, visualize_filename, defect_vertices, 20);
    }

    /// debug a case where the blossom expansion is not implemented
    #[test]
    fn primal_module_embedded_debug_1() {
        // cargo test primal_module_embedded_debug_1 -- --nocapture
        let visualize_filename = "primal_module_embedded_debug_1.json".to_string();
        let defect_vertices = vec![49, 73, 74, 86, 97];
        primal_module_embedded_basic_standard_syndrome(11, visualize_filename, defect_vertices, 5);
    }

    /// run randomized test cases for coverage test, with deterministic seed for reproducibility
    #[test]
    fn primal_module_embedded_randomized_test() {
        // cargo test primal_module_embedded_randomized_test -- --nocapture
        #[cfg(not(debug_assertions))]
        crate::cli::execute_in_cli(
            [
                "".to_string(),
                "test".to_string(),
                "primal-embedded".to_string(),
                "--use-deterministic-seed".to_string(),
            ]
            .iter(),
            true,
        );
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
