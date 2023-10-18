use crate::interface::*;
use crate::util::*;

#[derive(Debug)]
pub enum CommanderResponse {
    NonZeroGrow {
        length: Weight,
    },
    Conflict {
        node_1: NodeIndex,
        node_2: NodeIndex,
        touch_1: NodeIndex,
        touch_2: NodeIndex,
        vertex_1: VertexIndex,
        vertex_2: VertexIndex,
    },
    BlossomNeedExpand {
        blossom: NodeIndex,
    },
}

pub trait DualCommanderDriver {
    fn set_speed(&mut self, node: NodeIndex, speed: GrowState);
    fn set_blossom(&mut self, node: NodeIndex, blossom: NodeIndex);
    fn find_obstacle(&mut self) -> CommanderResponse;
    fn grow(&mut self, length: Weight);
}

/// a dual module implementation that calls the driver to do its jobs
pub struct DualModuleCommander<D: DualCommanderDriver> {
    pub driver: D,
}

impl<D: DualCommanderDriver> DualInterface for DualModuleCommander<D> {
    fn new_empty() -> Self {
        unimplemented!("use `new` instead, providing the driver")
    }

    fn clear(&mut self) {
        unimplemented!()
    }

    fn create_blossom(&mut self, primal_module: &impl PrimalInterface, blossom_index: NodeIndex) {
        primal_module.iterate_blossom_children(blossom_index, |_primal_module, child_index| {
            self.driver.set_blossom(child_index, blossom_index);
        })
    }

    fn expand_blossom(&mut self, primal_module: &impl PrimalInterface, blossom_index: NodeIndex) {
        primal_module.iterate_blossom_children(blossom_index, |primal_module, child_index| {
            self.iterative_expand_blossom(primal_module, blossom_index, child_index);
        });
    }

    fn set_grow_state(&mut self, node_index: NodeIndex, grow_state: GrowState) {
        self.driver.set_speed(node_index, grow_state);
    }
}

impl<D: DualCommanderDriver> DualModuleCommander<D> {
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
    use std::collections::BTreeMap;

    #[test]
    fn dual_module_commander_basic_1() {
        // cargo test dual_module_commander_basic_1 -- --nocapture
        let mut primal = MockPrimal::new_empty();
        primal.add_defect(0);
        primal.add_defect(1);
        primal.add_defect(2);
        primal.add_defect(3);
        primal.add_defect(4);
        primal.add_blossom(100, vec![0, 1, 3]);
        primal.add_blossom(101, vec![2, 100, 4]);
        let mut dual = DualModuleCommander::new(MockDualDriver::new());
        dual.create_blossom(&primal, 100);
        dual.driver
            .check(&["set_blossom(0, 100)", "set_blossom(1, 100)", "set_blossom(3, 100)"]);
        dual.create_blossom(&primal, 101);
        dual.driver
            .check(&["set_blossom(2, 101)", "set_blossom(100, 101)", "set_blossom(4, 101)"]);
        dual.expand_blossom(&primal, 100);
        dual.driver.check(&[]);
        dual.expand_blossom(&primal, 101);
        dual.driver.check(&[]);
    }

    pub struct MockPrimal {
        pub nodes: BTreeMap<NodeIndex, MockPrimalNode>,
    }

    pub struct MockPrimalNode {
        parent: Option<NodeIndex>,
        children: Vec<NodeIndex>,
    }

    impl PrimalInterface for MockPrimal {
        fn new_empty() -> Self {
            Self { nodes: BTreeMap::new() }
        }
        fn clear(&mut self) {}
        fn is_blossom(&self, node_index: NodeIndex) -> bool {
            !self.nodes[&node_index].children.is_empty()
        }
        fn iterate_blossom_children(&self, blossom_index: NodeIndex, mut func: impl FnMut(&Self, NodeIndex)) {
            for &child_index in self.nodes[&blossom_index].children.iter() {
                func(self, child_index);
            }
        }
    }

    impl MockPrimal {
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
    }

    impl MockDualDriver {
        pub fn new() -> Self {
            Self {
                verbose: true,
                logs: vec![],
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

    impl DualCommanderDriver for MockDualDriver {
        fn set_speed(&mut self, node: NodeIndex, speed: GrowState) {
            self.log(format!("set_speed({node}, {speed:?})"));
        }
        fn set_blossom(&mut self, node: NodeIndex, blossom: NodeIndex) {
            self.log(format!("set_blossom({node}, {blossom})"));
        }
        fn find_obstacle(&mut self) -> CommanderResponse {
            self.log(format!("find_obstacle()"));
            unimplemented!()
        }
        fn grow(&mut self, length: Weight) {
            self.log(format!("grow({length})"));
        }
    }
}
