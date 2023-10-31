use fusion_blossom::dual_module::*;
use fusion_blossom::util::*;
use micro_blossom_nostd::interface::*;
use micro_blossom_nostd::util::*;

trait DualInterfaceWithInitializer {
    fn new_with_initializer(initializer: &SolverInitializer) -> Self;
}

#[derive(Debug)]
pub struct DualModuleAdaptor<D: DualInterface + DualInterfaceWithInitializer> {
    pub dual_module: D,
    pub nodes: Vec<DualNodePtr>,
}

impl<D: DualInterface + DualInterfaceWithInitializer> DualModuleImpl for DualModuleAdaptor<D> {
    fn new_empty(initializer: &SolverInitializer) -> Self {
        Self {
            dual_module: D::new_with_initializer(initializer),
            nodes: vec![],
        }
    }

    fn clear(&mut self) {
        self.dual_module.clear();
        self.nodes.clear();
    }

    fn add_dual_node(&mut self, dual_node_ptr: &DualNodePtr) {
        let node = dual_node_ptr.read_recursive();
        assert_eq!(node.index, self.nodes.len());
        self.nodes.push(dual_node_ptr.clone());
        match &node.class {
            DualNodeClass::Blossom { nodes_circle, .. } => {
                self.blossom_tracker
                    .create_blossom(micro_blossom_nostd::util::ni!(node.index));
                // creating blossom is cheap
                for weak_ptr in nodes_circle.iter() {
                    let child_node_ptr = weak_ptr.upgrade_force();
                    let child_node = child_node_ptr.read_recursive();
                    self.execute_instruction(Instruction::SetBlossom {
                        node: child_node.index,
                        blossom: node.index,
                    });
                    if matches!(child_node.class, DualNodeClass::Blossom { .. }) {
                        self.blossom_tracker
                            .set_speed(micro_blossom_nostd::util::ni!(child_node.index), CompactGrowState::Stay);
                    }
                }
                // TODO: use priority queue to track shrinking blossom constraint
            }
            DualNodeClass::DefectVertex { defect_index } => {
                assert!(!self.vertices[*defect_index].is_defect, "cannot set defect twice");
                self.execute_instruction(Instruction::AddDefectVertex {
                    vertex: *defect_index,
                    node: node.index,
                });
            }
        }
    }
}
