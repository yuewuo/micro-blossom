use crate::interface::*;
use crate::util::*;

pub trait DualStacklessDriver {
    fn clear(&mut self);
    fn set_speed(&mut self, node: NodeIndex, speed: GrowState);
    fn set_blossom(&mut self, node: NodeIndex, blossom: NodeIndex);
    fn find_obstacle(&mut self) -> MaxUpdateLength;
    fn grow(&mut self, length: Weight);
}

/// a dual module implementation that removes the need to maintain a stack of blossom structure
pub struct DualModuleStackless<D: DualStacklessDriver> {
    pub driver: D,
}

impl<D: DualStacklessDriver> DualInterface for DualModuleStackless<D> {
    fn clear(&mut self) {
        self.driver.clear();
    }

    fn create_blossom(&mut self, primal_module: &impl PrimalInterface, blossom_index: NodeIndex) {
        primal_module.iterate_blossom_children(blossom_index, |_primal_module, child_index| {
            self.driver.set_blossom(child_index, blossom_index);
        })
    }

    fn expand_blossom(&mut self, primal_module: &impl PrimalInterface, blossom_index: NodeIndex) {
        primal_module.iterate_blossom_children(blossom_index, |primal_module, child_index| {
            self.iterative_expand_blossom(primal_module, child_index, child_index);
        });
    }

    fn set_grow_state(&mut self, node_index: NodeIndex, grow_state: GrowState) {
        self.driver.set_speed(node_index, grow_state);
    }

    fn compute_maximum_update_length(&mut self) -> MaxUpdateLength {
        self.driver.find_obstacle()
    }

    fn grow(&mut self, length: Weight) {
        self.driver.grow(length);
    }
}

impl<D: DualStacklessDriver> DualModuleStackless<D> {
    pub fn new(driver: D) -> Self {
        Self { driver }
    }

    fn iterative_expand_blossom(
        &mut self,
        primal_module: &impl PrimalInterface,
        blossom_index: NodeIndex,
        child_index: NodeIndex,
    ) {
        if primal_module.is_blossom(child_index) {
            primal_module.iterate_blossom_children(child_index, |primal_module, grandchild_index| {
                self.iterative_expand_blossom(primal_module, blossom_index, grandchild_index);
            });
        } else {
            self.driver.set_blossom(child_index, blossom_index);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, VecDeque};

    #[test]
    fn dual_module_stackless_basic_1() {
        // cargo test dual_module_stackless_basic_1 -- --nocapture
        let mut primal = MockPrimal::new_empty();
        primal.add_defect(0);
        primal.add_defect(1);
        primal.add_defect(2);
        primal.add_defect(3);
        primal.add_defect(4);
        primal.add_blossom(100, vec![0, 1, 3]);
        primal.add_blossom(101, vec![2, 100, 4]);
        let mut dual = DualModuleStackless::new(MockDualDriver::new());
        dual.create_blossom(&primal, 100);
        dual.driver
            .check(&["set_blossom(0, 100)", "set_blossom(1, 100)", "set_blossom(3, 100)"]);
        dual.create_blossom(&primal, 101);
        dual.driver
            .check(&["set_blossom(2, 101)", "set_blossom(100, 101)", "set_blossom(4, 101)"]);
        dual.expand_blossom(&primal, 100);
        dual.driver
            .check(&["set_blossom(0, 0)", "set_blossom(1, 1)", "set_blossom(3, 3)"]);
        dual.expand_blossom(&primal, 101);
        // this is the tricky part: only defect vertices are updated;
        // it's designed this way so that the hardware accelerator doesn't have to maintain
        // a stack of blossom nodes which is very expensive considering the worst case
        // this is acceptable because expanding a large blossom is exponentially unlikely to happen
        dual.driver.check(&[
            "set_blossom(2, 2)",
            "set_blossom(0, 100)",
            "set_blossom(1, 100)",
            "set_blossom(3, 100)",
            "set_blossom(4, 4)",
        ]);
    }

    pub struct MockPrimal {
        pub nodes: BTreeMap<NodeIndex, MockPrimalNode>,
    }

    pub struct MockPrimalNode {
        parent: Option<NodeIndex>,
        children: Vec<NodeIndex>,
    }

    impl PrimalInterface for MockPrimal {
        fn clear(&mut self) {}
        fn is_blossom(&self, node_index: NodeIndex) -> bool {
            !self.nodes[&node_index].children.is_empty()
        }
        fn iterate_blossom_children_with_touching(
            &self,
            _blossom_index: NodeIndex,
            _func: impl FnMut(&Self, NodeIndex, ((NodeIndex, VertexIndex), (NodeIndex, VertexIndex))),
        ) {
            unimplemented!()
        }
        fn iterate_blossom_children(&self, blossom_index: NodeIndex, mut func: impl FnMut(&Self, NodeIndex)) {
            for &child_index in self.nodes[&blossom_index].children.iter() {
                func(self, child_index);
            }
        }
        fn resolve(&mut self, _dual_module: &mut impl DualInterface, _max_update_length: MaxUpdateLength) {
            unimplemented!()
        }
        fn iterate_perfect_matching(&mut self, _func: impl FnMut(&Self, NodeIndex)) {
            unimplemented!()
        }
    }

    impl MockPrimal {
        fn new_empty() -> Self {
            Self { nodes: BTreeMap::new() }
        }
        pub fn add_defect(&mut self, node_index: NodeIndex) {
            assert!(!self.nodes.contains_key(&node_index));
            self.nodes.insert(
                node_index,
                MockPrimalNode {
                    parent: None,
                    children: vec![],
                },
            );
        }
        pub fn add_blossom(&mut self, blossom_index: NodeIndex, children: Vec<NodeIndex>) {
            assert!(!self.nodes.contains_key(&blossom_index));
            assert!(children.len() % 2 == 1, "blossom must be odd cycle");
            for child_index in children.iter() {
                assert!(self.nodes.contains_key(child_index));
                assert!(self.nodes[child_index].parent.is_none(), "child already has a parent");
                self.nodes.get_mut(child_index).unwrap().parent = Some(blossom_index);
            }
            self.nodes.insert(blossom_index, MockPrimalNode { parent: None, children });
        }
    }

    pub struct MockDualDriver {
        pub verbose: bool, // whether print every log
        pub logs: Vec<String>,
        pub pending_obstacles: VecDeque<MaxUpdateLength>,
    }

    impl MockDualDriver {
        pub fn new() -> Self {
            Self {
                verbose: true,
                logs: vec![],
                pending_obstacles: VecDeque::new(),
            }
        }
        pub fn log(&mut self, message: String) {
            if self.verbose {
                println!("{}", message);
            }
            self.logs.push(message);
        }
        pub fn check(&mut self, messages: &[&str]) {
            assert_eq!(self.logs, messages);
            self.logs.clear();
            if self.verbose {
                println!("[checked]");
            }
        }
    }

    impl DualStacklessDriver for MockDualDriver {
        fn clear(&mut self) {
            self.log(format!("clear()"));
        }
        fn set_speed(&mut self, node: NodeIndex, speed: GrowState) {
            self.log(format!("set_speed({node}, {speed:?})"));
        }
        fn set_blossom(&mut self, node: NodeIndex, blossom: NodeIndex) {
            self.log(format!("set_blossom({node}, {blossom})"));
        }
        fn find_obstacle(&mut self) -> MaxUpdateLength {
            self.log(format!("find_obstacle()"));
            self.pending_obstacles.pop_front().unwrap()
        }
        fn grow(&mut self, length: Weight) {
            self.log(format!("grow({length})"));
        }
    }
}
