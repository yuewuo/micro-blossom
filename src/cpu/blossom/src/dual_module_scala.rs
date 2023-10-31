//! Dual Module implemented in Scala (SpinalHDL) and simulated via verilator
//!
//! This dual module will spawn a Scala program that compiles and runs a given decoding graph
//!

use crate::resources::*;
use crate::util::*;
use derivative::Derivative;
use fusion_blossom::dual_module::*;
use fusion_blossom::util::*;
use fusion_blossom::visualize::*;
use micro_blossom_nostd::blossom_tracker::*;
use micro_blossom_nostd::util::*;
use serde_json::json;
use std::io::prelude::*;
use std::io::{BufReader, LineWriter};
use std::net::{TcpListener, TcpStream};
use std::process::{Child, Command};
use wait_timeout::ChildExt;

#[derive(Derivative)]
#[derivative(Debug)]
pub struct DualModuleScala {
    #[derivative(Debug = "ignore")]
    pub scala_driver: ScalaDriver,
    pub blossom_tracker: Box<BlossomTracker<MAX_NODE_NUM>>,
    /// temporary list of synchronize requests, not used until hardware fusion
    pub sync_requests: Vec<SyncRequest>,
}

pub struct ScalaDriver {
    pub port: u16,
    pub child: Child,
    pub reader: BufReader<TcpStream>,
    pub writer: LineWriter<TcpStream>,
}

// https://stackoverflow.com/questions/30538004/how-do-i-ensure-that-a-spawned-child-process-is-killed-if-my-app-panics
impl Drop for ScalaDriver {
    fn drop(&mut self) {
        let need_to_kill: bool = (|| {
            if write!(self.writer, "quit\n").is_ok() {
                let wait_time = std::time::Duration::from_millis(1000);
                if let Ok(Some(status)) = self.child.wait_timeout(wait_time) {
                    return !status.success();
                }
            }
            true
        })();
        if need_to_kill {
            match self.child.kill() {
                Err(e) => println!("Could not kill Scala process: {}", e),
                Ok(_) => println!("Successfully killed Scala process"),
            }
        } else {
            println!("Scala process quit normally");
        }
        match std::fs::remove_dir_all(format!("../../../simWorkspace/dualHost/{}/rtl", self.port)) {
            Err(e) => println!("Could not remove rtl folder: {}", e),
            Ok(_) => println!("Successfully remove rtl folder"),
        }
        match std::fs::remove_dir_all(format!("../../../simWorkspace/dualHost/{}/verilator", self.port)) {
            Err(e) => println!("Could not remove verilator folder: {}", e),
            Ok(_) => println!("Successfully remove verilator folder"),
        }
    }
}

impl ScalaDriver {
    pub fn new(initializer: &SolverInitializer) -> std::io::Result<Self> {
        let hostname = "127.0.0.1";
        let listener = TcpListener::bind(format!("{hostname}:0"))?;
        let port = listener.local_addr()?.port();
        // start the scala simulator host
        println!("Starting Scala simulator host... this may take a while (listening on {hostname}:{port})");
        let child = Command::new("sbt")
            .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../"))
            .arg(format!("runMain microblossom.DualHost {hostname} {port}"))
            .spawn()?;
        let (socket, _addr) = listener.accept()?;
        let mut reader = BufReader::new(socket.try_clone()?);
        let mut writer = LineWriter::new(socket);
        let mut line = String::new();
        reader.read_line(&mut line)?;
        assert_eq!(line, "DualHost v0.0.1, ask for decoding graph\n", "handshake error");
        // in simulation, positions doesn't matter because it's not going to affect the timing constraint
        let micro_blossom = MicroBlossomSingle::new_initializer_only(initializer);
        write!(writer, "{}\n", serde_json::to_string(&micro_blossom).unwrap())?;
        write!(writer, "{}\n", if cfg!(test) { "with waveform" } else { "no waveform" })?;
        line.clear();
        reader.read_line(&mut line)?;
        assert_eq!(line, "simulation started\n");
        Ok(Self {
            port,
            child,
            reader,
            writer,
        })
    }
}

impl DualModuleImpl for DualModuleScala {
    fn new_empty(initializer: &SolverInitializer) -> Self {
        let mut dual_module = Self {
            scala_driver: ScalaDriver::new(initializer).unwrap(),
            blossom_tracker: Box::new(BlossomTracker::new()),
            sync_requests: vec![],
        };
        dual_module.clear();
        dual_module
    }

    fn clear(&mut self) {
        unimplemented!()
    }

    fn add_dual_node(&mut self, dual_node_ptr: &DualNodePtr) {
        unimplemented!()
    }

    fn remove_blossom(&mut self, dual_node_ptr: DualNodePtr) {
        unimplemented!()
    }

    fn set_grow_state(&mut self, dual_node_ptr: &DualNodePtr, grow_state: DualNodeGrowState) {
        unimplemented!()
    }

    fn compute_maximum_update_length(&mut self) -> GroupMaxUpdateLength {
        unimplemented!()
    }

    fn grow(&mut self, length: Weight) {
        unimplemented!()
    }

    fn prepare_nodes_shrink(&mut self, _nodes_circle: &[DualNodePtr]) -> &mut Vec<SyncRequest> {
        self.sync_requests.clear();
        &mut self.sync_requests
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fusion_blossom::example_codes::*;

    // to use visualization, we need the folder of fusion-blossom repo
    // e.g. export FUSION_DIR=/Users/wuyue/Documents/GitHub/fusion-blossom

    #[test]
    fn dual_module_scala_basic_1() {
        // cargo test dual_module_scala_basic_1 -- --nocapture
        let visualize_filename = "dual_module_scala_basic_1.json".to_string();
        let half_weight = 500;
        let mut code = CodeCapacityPlanarCode::new(7, 0.1, half_weight);
        let mut visualizer = Visualizer::new(
            option_env!("FUSION_DIR").map(|dir| dir.to_owned() + "/visualize/data/" + visualize_filename.as_str()),
            code.get_positions(),
            true,
        )
        .unwrap();
        print_visualize_link(visualize_filename.clone());
        // create dual module
        let initializer = code.get_initializer();
        let mut dual_module = DualModuleScala::new_empty(&initializer);
    }
}
