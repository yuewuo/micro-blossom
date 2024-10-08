//! Dual Module implemented in Scala (SpinalHDL) and simulated via verilator
//!
//! This dual module will spawn a Scala program that compiles and runs a given decoding graph
//!

use crate::dual_module_adaptor::*;
use crate::mwpm_solver::*;
use crate::resources::*;
use crate::simulation_tcp_client::*;
use crate::util::*;
use fusion_blossom::dual_module::*;
use fusion_blossom::primal_module::*;
use fusion_blossom::util::*;
use fusion_blossom::visualize::*;
use micro_blossom_nostd::dual_driver_tracked::*;
use micro_blossom_nostd::dual_module_stackless::*;
use micro_blossom_nostd::interface::*;
use micro_blossom_nostd::util::*;
use scan_fmt::*;
use serde::*;
use std::io::prelude::*;
use std::io::{BufReader, LineWriter};
use std::net::{TcpListener, TcpStream};
use std::process::Child;
use std::sync::Mutex;
use wait_timeout::ChildExt;

pub struct DualModuleScalaDriver {
    pub link: Mutex<Link>,
    pub config: DualScalaConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DualScalaConfig {
    #[serde(default = "Default::default")]
    pub sim_config: SimulationConfig,
    #[serde(default = "random_name_16")]
    pub name: String,
}

pub struct Link {
    pub port: u16,
    pub child: Child,
    pub reader: BufReader<TcpStream>,
    pub writer: LineWriter<TcpStream>,
}

pub type DualModuleScala = DualModuleStackless<DualDriverTracked<DualModuleScalaDriver, MAX_NODE_NUM>>;

impl SolverTrackedDual for DualModuleScalaDriver {
    fn new_from_graph_config(graph: MicroBlossomSingle, config: serde_json::Value) -> Self {
        Self::new(graph, serde_json::from_value(config).unwrap()).unwrap()
    }
    fn fuse_layer(&mut self, layer_id: usize) {
        self.load_syndrome_external(ni!(layer_id));
    }
    fn get_pre_matchings(&self, _belonging: DualModuleInterfaceWeak) -> PerfectMatching {
        // TODO: implement pre matching fetching
        PerfectMatching::default()
    }
}

pub type DualModuleScalaAdaptor = DualModuleAdaptor<DualModuleScala>;

// https://stackoverflow.com/questions/30538004/how-do-i-ensure-that-a-spawned-child-process-is-killed-if-my-app-panics
impl Drop for DualModuleScalaDriver {
    fn drop(&mut self) {
        let need_to_kill: bool = (|| {
            if write!(self.link.lock().unwrap().writer, "quit\n").is_ok() {
                let wait_time = std::time::Duration::from_millis(1000);
                if let Ok(Some(status)) = self.link.lock().unwrap().child.wait_timeout(wait_time) {
                    return !status.success();
                }
            }
            true
        })();
        if need_to_kill {
            match self.link.lock().unwrap().child.kill() {
                Err(e) => println!("Could not kill Scala process: {}", e),
                Ok(_) => println!("Successfully killed Scala process"),
            }
        } else {
            println!("Scala process quit normally");
        }
        if cfg!(test) {
            // only delete binary but keep original waveforms
            match std::fs::remove_dir_all(format!("../../../simWorkspace/dualHost/{}/rtl", self.config.name)) {
                Err(e) => println!("Could not remove rtl folder: {}", e),
                Ok(_) => println!("Successfully remove rtl folder"),
            }
            match std::fs::remove_dir_all(format!("../../../simWorkspace/dualHost/{}/verilator", self.config.name)) {
                Err(e) => println!("Could not remove verilator folder: {}", e),
                Ok(_) => println!("Successfully remove verilator folder"),
            }
        } else {
            match std::fs::remove_dir_all(format!("../../../simWorkspace/dualHost/{}", self.config.name)) {
                Err(e) => println!("Could not remove build folder: {}", e),
                Ok(_) => println!("Successfully remove build folder"),
            }
        }
    }
}

impl DualModuleScalaDriver {
    pub fn new(micro_blossom: MicroBlossomSingle, config: DualScalaConfig) -> std::io::Result<Self> {
        let hostname = "127.0.0.1";
        let listener = TcpListener::bind(format!("{hostname}:0"))?;
        let port = listener.local_addr()?.port();
        // start the scala simulator host
        println!("Starting Scala simulator host... this may take a while (listening on {hostname}:{port})");
        let child = SCALA_MICRO_BLOSSOM_RUNNER.run(
            "microblossom.DualHost",
            [format!("{hostname}"), format!("{port}"), format!("{}", config.name)],
        )?;
        let (socket, _addr) = listener.accept()?;
        let mut reader = BufReader::new(socket.try_clone()?);
        let mut writer = LineWriter::new(socket);
        let mut line = String::new();
        reader.read_line(&mut line)?;
        assert_eq!(line, "DualHost v0.0.1, ask for decoding graph\n", "handshake error");
        write!(writer, "{}\n", serde_json::to_string(&micro_blossom).unwrap())?;
        let simulation_lock = SCALA_SIMULATION_LOCK.lock();
        write!(writer, "{}\n", if cfg!(test) { "with waveform" } else { "no waveform" })?;
        line.clear();
        reader.read_line(&mut line)?;
        assert_eq!(line, "simulation started\n");
        drop(simulation_lock);
        write!(writer, "reset()\n")?;
        Ok(Self {
            link: Mutex::new(Link {
                port,
                child,
                reader,
                writer,
            }),
            config,
        })
    }

    fn load_syndrome_external(&mut self, layer_id: CompactVertexIndex) {
        write!(self.link.lock().unwrap().writer, "load_syndrome_external({layer_id})\n").unwrap();
    }
}

impl DualStacklessDriver for DualModuleScalaDriver {
    fn reset(&mut self) {
        write!(self.link.lock().unwrap().writer, "reset()\n").unwrap();
    }
    fn set_speed(&mut self, _is_blossom: bool, node: CompactNodeIndex, speed: CompactGrowState) {
        write!(self.link.lock().unwrap().writer, "set_speed({node}, {speed:?})\n").unwrap();
    }
    fn set_blossom(&mut self, node: CompactNodeIndex, blossom: CompactNodeIndex) {
        write!(self.link.lock().unwrap().writer, "set_blossom({node}, {blossom})\n").unwrap();
    }
    fn find_obstacle(&mut self) -> (CompactObstacle, CompactWeight) {
        write!(self.link.lock().unwrap().writer, "find_obstacle()\n").unwrap();
        let mut line = String::new();
        self.link.lock().unwrap().reader.read_line(&mut line).unwrap();
        // println!("find_obstacle -> {}", line);
        if line.starts_with("NonZeroGrow(") {
            let (length, grown) = scan_fmt!(&line, "NonZeroGrow({d}), {d}", Weight, Weight).unwrap();
            (
                if length == i32::MAX as Weight {
                    CompactObstacle::None
                } else {
                    CompactObstacle::GrowLength {
                        length: length as CompactWeight,
                    }
                },
                grown as CompactWeight,
            )
        } else if line.starts_with("Conflict(") {
            let (node_1, node_2, touch_1, touch_2, vertex_1, vertex_2, grown) = scan_fmt!(
                &line,
                "Conflict({d}, {d}, {d}, {d}, {d}, {d}), {d}",
                NodeIndex,
                NodeIndex,
                NodeIndex,
                NodeIndex,
                NodeIndex,
                NodeIndex,
                Weight
            )
            .unwrap();
            (
                CompactObstacle::Conflict {
                    node_1: ni!(node_1).option(),
                    node_2: if node_2 == i32::MAX as NodeIndex {
                        None.into()
                    } else {
                        ni!(node_2).option()
                    },
                    touch_1: ni!(touch_1).option(),
                    touch_2: if touch_2 == i32::MAX as NodeIndex {
                        None.into()
                    } else {
                        ni!(touch_2).option()
                    },
                    vertex_1: ni!(vertex_1),
                    vertex_2: ni!(vertex_2),
                },
                grown as CompactWeight,
            )
        } else {
            unreachable!()
        }
    }
    fn add_defect(&mut self, vertex: CompactVertexIndex, node: CompactNodeIndex) {
        write!(self.link.lock().unwrap().writer, "add_defect({vertex}, {node})\n").unwrap();
    }
}

impl DualTrackedDriver for DualModuleScalaDriver {
    fn find_conflict(&mut self, maximum_growth: CompactWeight) -> (CompactObstacle, CompactWeight) {
        write!(self.link.lock().unwrap().writer, "set_maximum_growth({maximum_growth})\n").unwrap();
        self.find_obstacle()
    }
}

impl FusionVisualizer for DualModuleScalaDriver {
    #[allow(clippy::unnecessary_cast)]
    fn snapshot(&self, abbrev: bool) -> serde_json::Value {
        write!(self.link.lock().unwrap().writer, "snapshot({abbrev})\n").unwrap();
        let mut line = String::new();
        self.link.lock().unwrap().reader.read_line(&mut line).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(1000));
        serde_json::from_str(&line).unwrap()
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::dual_module_adaptor::tests::*;
    use crate::dual_module_comb::tests::*;
    use serde_json::json;

    // to use visualization, we need the folder of fusion-blossom repo
    // e.g. export FUSION_DIR=/Users/wuyue/Documents/GitHub/fusion-blossom

    /// reported the wrong virtual matching;
    /// reason: vertex 1 should have been propagated by its neighbor 0 but it's not
    /// reason: mis-type `when(updateValid) {` to `when(executeValid) {`
    #[test]
    fn dual_module_scala_basic_1() {
        // cargo test dual_module_scala_basic_1 -- --nocapture
        let visualize_filename = "dual_module_scala_basic_1.json".to_string();
        let defect_vertices = vec![0, 4, 8];
        dual_module_scala_basic_standard_syndrome(3, visualize_filename, defect_vertices);
    }

    #[test]
    #[cfg(not(debug_assertions))]  // only in release mode
    fn dual_module_scala_basic_2() {
        // cargo test --release dual_module_scala_basic_2 -- --nocapture
        let visualize_filename = "dual_module_scala_basic_2.json".to_string();
        let defect_vertices = vec![18, 26, 34];
        dual_module_scala_basic_standard_syndrome(7, visualize_filename, defect_vertices);
    }

    #[test]
    #[cfg(not(debug_assertions))]  // only in release mode
    fn dual_module_scala_basic_3() {
        // cargo test --release dual_module_scala_basic_3 -- --nocapture
        let visualize_filename = "dual_module_scala_basic_3.json".to_string();
        let defect_vertices = vec![16, 26];
        dual_module_scala_basic_standard_syndrome(7, visualize_filename, defect_vertices);
    }

    /// debug infinite loop
    /// reason: the write stage logic is implemented wrongly: only when the overall speed is positive
    ///   should it report an obstacle; otherwise just report whatever the maxGrowth value is
    #[test]
    #[cfg(not(debug_assertions))]  // only in release mode
    fn dual_module_scala_debug_1() {
        // cargo test --release dual_module_scala_debug_1 -- --nocapture
        let visualize_filename = "dual_module_scala_debug_1.json".to_string();
        let defect_vertices = vec![3, 4, 5, 11, 12, 13, 18, 19, 21, 26, 28, 37, 44];
        dual_module_scala_basic_standard_syndrome(7, visualize_filename, defect_vertices);
    }

    #[test]
    fn dual_module_scala_debug_compare_1() {
        // cargo test dual_module_scala_debug_compare_1 -- --nocapture
        let visualize_filename = "dual_module_scala_debug_compare_1.json".to_string();
        let defect_vertices = vec![3, 4, 5, 11, 12, 13, 18, 19, 21, 26, 28, 37, 44];
        dual_module_comb_basic_standard_syndrome(7, visualize_filename, defect_vertices, false, false);
    }

    pub fn dual_module_scala_basic_standard_syndrome(
        d: VertexNum,
        visualize_filename: String,
        defect_vertices: Vec<VertexIndex>,
    ) -> SolverEmbeddedScala {
        dual_module_standard_optional_viz(
            d,
            Some(visualize_filename.clone()),
            defect_vertices,
            |initializer, positions| {
                SolverEmbeddedScala::new(
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
