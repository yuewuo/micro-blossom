#[macro_export]
macro_rules! bind_fusion_visualizer {
    ($struct_name:ident) => {
        impl FusionVisualizer for $struct_name {
            fn snapshot(&self, abbrev: bool) -> serde_json::Value {
                let mut value = self.primal_module.snapshot(abbrev);
                snapshot_combine_values(&mut value, self.dual_module.snapshot(abbrev), abbrev);
                snapshot_combine_values(&mut value, self.interface_ptr.snapshot(abbrev), abbrev);
                value
            }
        }
    };
    ($struct_name:ident, 1) => {
        impl FusionVisualizer for $struct_name {
            fn snapshot(&self, abbrev: bool) -> serde_json::Value {
                let mut value = self.dual_module.driver.driver.snapshot(abbrev);
                snapshot_combine_values(&mut value, self.primal_module.snapshot(abbrev), abbrev);
                snapshot_combine_values(
                    &mut value,
                    DualNodesOf::new(&self.primal_module).snapshot(abbrev),
                    abbrev,
                );
                value
            }
        }
    };
}

#[allow(unused_imports)]
pub use bind_fusion_visualizer;

#[macro_export]
macro_rules! common_perfect_matching_visualizer {
    () => {
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
    };
}

#[allow(unused_imports)]
pub use common_perfect_matching_visualizer;

#[macro_export]
macro_rules! common_subgraph_visualizer {
    () => {
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
    };
}

#[allow(unused_imports)]
pub use common_subgraph_visualizer;

#[macro_export]
macro_rules! common_sum_dual_variables {
    () => {
        fn sum_dual_variables(&self) -> Weight {
            self.interface_ptr.read_recursive().sum_dual_variables
        }
    };
}

#[allow(unused_imports)]
pub use common_sum_dual_variables;

#[macro_export]
macro_rules! common_generate_profiler_report {
    () => {
        fn generate_profiler_report(&self) -> serde_json::Value {
            json!({
                "dual": self.dual_module.generate_profiler_report(),
                "primal": self.primal_module.generate_profiler_report(),
            })
        }
    }
}

#[allow(unused_imports)]
pub use common_generate_profiler_report;
