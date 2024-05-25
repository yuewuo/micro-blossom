//! Dual Module implemented in Scala (SpinalHDL) with Stream interface and simulated via verilator
//!
//! This dual module will spawn a Scala program that compiles and runs a given decoding graph.
//! It simulates the complete MicroBlossomLooper module, which provides a stream interface.
//! (A wrapper around the DistributedDual module)
//!

use crate::mwpm_solver::*;
use crate::resources::*;
use crate::simulation_tcp_client::*;
use crate::util::*;
use fusion_blossom::dual_module::*;
use fusion_blossom::primal_module::*;
use fusion_blossom::visualize::*;
use micro_blossom_nostd::dual_driver_tracked::*;
use micro_blossom_nostd::dual_module_stackless::*;
use micro_blossom_nostd::instruction::*;
use micro_blossom_nostd::interface::*;
use micro_blossom_nostd::util::*;
use serde::*;

pub struct DualModuleLooperDriver {
    pub client: SimulationTcpClient,
    pub context_id: u16,
    pub instruction_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DualLooperConfig {
    #[serde(default = "Default::default")]
    pub sim_config: SimulationConfig,
    #[serde(default = "random_name_16")]
    pub name: String,
}

pub type DualModuleLooper = DualModuleStackless<DualDriverTracked<DualModuleLooperDriver, MAX_NODE_NUM>>;

impl SolverTrackedDual for DualModuleLooperDriver {
    fn new_from_graph_config(graph: MicroBlossomSingle, config: serde_json::Value) -> Self {
        Self::new(graph, serde_json::from_value(config).unwrap()).unwrap()
    }
    fn fuse_layer(&mut self, layer_id: usize) {
        self.execute_instruction(Instruction32::load_syndrome_external(ni!(layer_id)), self.context_id)
            .unwrap();
    }
    fn get_pre_matchings(&self, _belonging: DualModuleInterfaceWeak) -> PerfectMatching {
        // TODO: implement pre matching fetching
        PerfectMatching::default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputData {
    pub instruction: u32, // use Instruction32
    pub context_id: u16,
    pub instruction_id: u16,
    pub maximum_growth: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutputData {
    pub context_id: u16,
    pub instruction_id: u16,
    pub max_growable: u16,
    pub conflict: ConvergecastConflict,
    pub grown: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConvergecastConflict {
    pub node1: u16,
    pub node2: Option<u16>,
    pub touch1: u16,
    pub touch2: Option<u16>,
    pub vertex1: u16,
    pub vertex2: u16,
    pub valid: bool,
}

impl DualModuleLooperDriver {
    pub fn new(micro_blossom: MicroBlossomSingle, config: DualLooperConfig) -> std::io::Result<Self> {
        let mut value = Self {
            client: SimulationTcpClient::new("LooperHost", micro_blossom, config.name, config.sim_config)?,
            context_id: 0,
            instruction_count: 0,
        };
        value.reset();
        Ok(value)
    }

    fn execute(&mut self, input: InputData) -> std::io::Result<OutputData> {
        let line = self
            .client
            .read_line(format!("execute: {}", serde_json::to_string(&input)?))?;
        self.instruction_count += 1;
        Ok(serde_json::from_str(line.as_str())?)
    }

    pub fn execute_instruction(&mut self, instruction: Instruction32, context_id: u16) -> std::io::Result<OutputData> {
        self.execute(InputData {
            instruction: instruction.into(),
            context_id,
            instruction_id: self.instruction_count as u16,
            maximum_growth: 0,
        })
    }

    pub fn execute_find_obstacle(
        &mut self,
        context_id: u16,
        maximum_growth: u16,
    ) -> std::io::Result<(CompactObstacle, CompactWeight)> {
        let instruction_id = self.instruction_count as u16;
        let output = self.execute(InputData {
            instruction: Instruction32::find_obstacle().into(),
            context_id,
            instruction_id,
            maximum_growth,
        })?;
        assert_eq!(output.instruction_id, instruction_id);
        let grown = CompactWeight::from(output.grown);
        if output.max_growable == u16::MAX {
            assert!(!output.conflict.valid, "growable must be finite when conflict is detected");
            return Ok((CompactObstacle::None, grown));
        }
        if output.conflict.valid {
            return Ok((
                CompactObstacle::Conflict {
                    node_1: ni!(output.conflict.node1).option(),
                    node_2: output.conflict.node2.map(|v| ni!(v)).into(),
                    touch_1: ni!(output.conflict.touch1).option(),
                    touch_2: output.conflict.touch2.map(|v| ni!(v)).into(),
                    vertex_1: ni!(output.conflict.vertex1),
                    vertex_2: ni!(output.conflict.vertex2),
                },
                grown,
            ));
        }
        return Ok((
            CompactObstacle::GrowLength {
                length: CompactWeight::from(output.max_growable),
            },
            grown,
        ));
    }
}

impl DualStacklessDriver for DualModuleLooperDriver {
    fn reset(&mut self) {
        self.execute_instruction(Instruction32::reset(), self.context_id).unwrap();
        self.instruction_count = 0;
    }
    fn set_speed(&mut self, _is_blossom: bool, node: CompactNodeIndex, speed: CompactGrowState) {
        self.execute_instruction(Instruction32::set_speed(node, speed), self.context_id)
            .unwrap();
    }
    fn set_blossom(&mut self, node: CompactNodeIndex, blossom: CompactNodeIndex) {
        self.execute_instruction(Instruction32::set_blossom(node, blossom), self.context_id)
            .unwrap();
    }
    fn find_obstacle(&mut self) -> (CompactObstacle, CompactWeight) {
        self.execute_find_obstacle(self.context_id, u16::MAX).unwrap()
    }
    fn add_defect(&mut self, vertex: CompactVertexIndex, node: CompactNodeIndex) {
        self.execute_instruction(Instruction32::add_defect_vertex(vertex, node), self.context_id)
            .unwrap();
    }
}

impl DualTrackedDriver for DualModuleLooperDriver {
    fn find_conflict(&mut self, maximum_growth: CompactWeight) -> (CompactObstacle, CompactWeight) {
        self.execute_find_obstacle(self.context_id, maximum_growth as u16).unwrap()
    }
}

impl FusionVisualizer for DualModuleLooperDriver {
    #[allow(clippy::unnecessary_cast)]
    fn snapshot(&self, abbrev: bool) -> serde_json::Value {
        assert_eq!(
            self.client.sim_config.context_depth, 1,
            "context snapshot is not yet supported"
        );
        let line = self.client.read_line(format!("snapshot({abbrev})")).unwrap();
        serde_json::from_str(&line).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dual_module_adaptor::tests::*;
    use fusion_blossom::util::*;
    use serde_json::json;

    // to use visualization, we need the folder of fusion-blossom repo
    // e.g. export FUSION_DIR=/Users/wuyue/Documents/GitHub/fusion-blossom

    #[test]
    fn dual_module_looper_basic_1() {
        // cargo test dual_module_looper_basic_1 -- --nocapture
        // WITH_WAVEFORM=1 KEEP_RTL_FOLDER=1 cargo test dual_module_looper_basic_1 -- --nocapture
        // WITH_WAVEFORM=1 KEEP_RTL_FOLDER=1 BROADCAST_DELAY=2 cargo test dual_module_looper_basic_1 -- --nocapture
        let visualize_filename = "dual_module_looper_basic_1.json".to_string();
        let defect_vertices = vec![0, 4, 8];
        dual_module_looper_basic_standard_syndrome(3, visualize_filename, defect_vertices);
    }

    pub fn dual_module_looper_basic_standard_syndrome(
        d: VertexNum,
        visualize_filename: String,
        defect_vertices: Vec<VertexIndex>,
    ) -> SolverEmbeddedLooper {
        dual_module_standard_optional_viz(
            d,
            Some(visualize_filename.clone()),
            defect_vertices,
            |initializer, positions| {
                SolverEmbeddedLooper::new(
                    MicroBlossomSingle::new(initializer, positions),
                    json!({
                        "dual": {
                            "name": visualize_filename.as_str().trim_end_matches(".json").to_string()
                            // "with_max_iterations": 30, // this is helpful when debugging infinite loops
                        }
                    }),
                )
            },
        )
    }
}
