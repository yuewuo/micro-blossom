use fusion_blossom::dual_module::*;
use fusion_blossom::util::*;
use micro_blossom_nostd::interface::*;
use micro_blossom_nostd::util::*;

pub trait DualInterfaceWithInitializer {
    fn new_with_initializer(initializer: &SolverInitializer) -> Self;
}

pub struct DualModuleAdaptor<D: DualInterface + DualInterfaceWithInitializer> {
    // always reconstruct the whole graph when reset
    pub dual_module: D,
    /// the nodes that interact with dual module interface
    pub nodes: Vec<DualNodePtr>,
    /// temporary list of synchronize requests, not used until hardware fusion
    pub sync_requests: Vec<SyncRequest>,
    pub grown: Weight,
}

impl<D: DualInterface + DualInterfaceWithInitializer> DualModuleImpl for DualModuleAdaptor<D> {
    fn new_empty(initializer: &SolverInitializer) -> Self {
        Self {
            dual_module: D::new_with_initializer(initializer),
            nodes: vec![],
            sync_requests: vec![],
            grown: 0,
        }
    }

    fn clear(&mut self) {
        self.dual_module.reset();
        // clear nodes
        self.nodes.clear();
        self.grown = 0;
    }

    fn add_dual_node(&mut self, dual_node_ptr: &DualNodePtr) {
        let node = dual_node_ptr.read_recursive();
        assert_eq!(node.index, self.nodes.len());
        self.nodes.push(dual_node_ptr.clone());
        match &node.class {
            DualNodeClass::Blossom { .. } => {
                self.dual_module
                    .create_blossom(&mut MockPrimalInterface { nodes: &mut self.nodes }, ni!(node.index));
            }
            DualNodeClass::DefectVertex { defect_index } => {
                self.dual_module.add_defect(ni!(*defect_index), ni!(node.index));
            }
        }
    }

    fn remove_blossom(&mut self, dual_node_ptr: DualNodePtr) {
        // remove blossom is expensive because the vertices doesn't remember all the chain of blossom
        let node = dual_node_ptr.read_recursive();
        self.dual_module
            .expand_blossom(&mut MockPrimalInterface { nodes: &mut self.nodes }, ni!(node.index));
    }

    fn set_grow_state(&mut self, dual_node_ptr: &DualNodePtr, grow_state: DualNodeGrowState) {
        let node = dual_node_ptr.read_recursive();
        self.dual_module.set_speed(
            node.class.is_blossom(),
            ni!(node.index),
            match grow_state {
                DualNodeGrowState::Grow => CompactGrowState::Grow,
                DualNodeGrowState::Shrink => CompactGrowState::Shrink,
                DualNodeGrowState::Stay => CompactGrowState::Stay,
            },
        );
    }

    fn compute_maximum_update_length(&mut self) -> GroupMaxUpdateLength {
        assert!(
            self.grown == 0,
            "must clear grown value before next round, to make sure interface is notified; see `SolverDualRTL` for more info"
        );
        let (obstacle, grown) = self.dual_module.find_obstacle();
        self.grown = grown as Weight;
        let max_update_length = match obstacle {
            CompactObstacle::GrowLength { length } => MaxUpdateLength::NonZeroGrow((length as Weight, false)),
            CompactObstacle::Conflict {
                node_1,
                node_2,
                touch_1,
                touch_2,
                vertex_1: _,
                vertex_2,
            } => {
                if node_2.is_some() {
                    MaxUpdateLength::Conflicting(
                        (
                            self.nodes[node_1.unwrap().get() as usize].clone(),
                            self.nodes[touch_1.unwrap().get() as usize].clone(),
                        ),
                        (
                            self.nodes[node_2.unwrap().get() as usize].clone(),
                            self.nodes[touch_2.unwrap().get() as usize].clone(),
                        ),
                    )
                } else {
                    MaxUpdateLength::TouchingVirtual(
                        (
                            self.nodes[node_1.unwrap().get() as usize].clone(),
                            self.nodes[touch_1.unwrap().get() as usize].clone(),
                        ),
                        (vertex_2.get() as VertexIndex, false),
                    )
                }
            }
            CompactObstacle::BlossomNeedExpand { blossom } => {
                MaxUpdateLength::BlossomNeedExpand(self.nodes[blossom.get() as usize].clone())
            }
            CompactObstacle::None => MaxUpdateLength::NonZeroGrow((Weight::MAX, false)),
        };
        let mut group_max_update_length = GroupMaxUpdateLength::new();
        group_max_update_length.add(max_update_length);
        group_max_update_length
    }

    fn grow(&mut self, _length: Weight) {
        unimplemented!("RTL dual module doesn't allow explicit grow command")
    }

    fn prepare_nodes_shrink(&mut self, _nodes_circle: &[DualNodePtr]) -> &mut Vec<SyncRequest> {
        self.sync_requests.clear();
        &mut self.sync_requests
    }
}

/// mocking the interface of the embedded primal module
#[derive(Debug)]
pub struct MockPrimalInterface<'a> {
    pub nodes: &'a mut Vec<DualNodePtr>,
}

impl<'a> PrimalInterface for MockPrimalInterface<'a> {
    fn reset(&mut self) {
        unreachable!("should not be called")
    }
    fn is_blossom(&self, node_index: CompactNodeIndex) -> bool {
        self.nodes[node_index.get() as usize].read_recursive().class.is_blossom()
    }
    fn iterate_blossom_children(
        &self,
        blossom_index: CompactNodeIndex,
        mut func: impl FnMut(&Self, CompactNodeIndex, &TouchingLink),
    ) {
        match &self.nodes[blossom_index.get() as usize].read_recursive().class {
            DualNodeClass::Blossom {
                nodes_circle,
                touching_children,
            } => {
                for (idx, node_weak) in nodes_circle.iter().enumerate() {
                    let peer_index = node_weak.upgrade_force().read_recursive().index;
                    let touch = touching_children[idx].1.upgrade_force().read_recursive().index;
                    let peer_touch = touching_children[(idx + 1) % nodes_circle.len()]
                        .0
                        .upgrade_force()
                        .read_recursive()
                        .index;
                    let link = TouchingLink {
                        touch: Some(ni!(touch)),
                        through: Some(ni!(0)),
                        peer_touch: Some(ni!(peer_touch)),
                        peer_through: Some(ni!(0)),
                    };
                    func(self, ni!(peer_index), &link);
                }
            }
            _ => unreachable!(),
        }
    }
    fn resolve(&mut self, _dual_module: &mut impl DualInterface, _max_update_length: CompactObstacle) -> bool {
        unreachable!("should not be called")
    }
    fn iterate_perfect_matching(&mut self, _func: impl FnMut(&Self, CompactNodeIndex, CompactMatchTarget, &TouchingLink)) {
        unreachable!("should not be called")
    }
    fn break_with_virtual_vertex(
        &mut self,
        _dual_module: &mut impl DualInterface,
        _virtual_vertex: CompactVertexIndex,
        _hint_node_index: CompactNodeIndex,
    ) -> bool {
        unreachable!("should not be called")
    }
}
