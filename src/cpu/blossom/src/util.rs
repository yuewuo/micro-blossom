use konst::{option, primitive::parse_usize, result::unwrap_ctx};
use lazy_static::lazy_static;
use std::process::{Child, Command};

// by default guarantees working at d=31 circuit-level-noise (30k vertices), but can increase if needed
pub const MAX_NODE_NUM: usize = unwrap_ctx!(parse_usize(option::unwrap_or!(option_env!("MAX_NODE_NUM"), "50000")));

/// a fusion group is a continuous subset of vertices which is recovered simultaneously;
/// it is required that
pub struct FusionGroups {}

/// the runner will first compile the jar package from /src/fpga/microblossom using `sbt`;
/// it allows running main functions in parallel without conflicts due to sbt.
pub struct ScalaMicroBlossomRunner {}

impl ScalaMicroBlossomRunner {
    /// private new function
    fn new() -> Self {
        // if MANUALLY_COMPILE_QEC is set, then ignore the compile process
        let manual_compile = match std::env::var("MANUALLY_COMPILE_QEC") {
            Ok(value) => value != "",
            Err(_) => false,
        };
        if !manual_compile {
            let mut child = Command::new("sbt")
                .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../"))
                .arg("assembly")
                .spawn()
                .unwrap();
            let status = child.wait().expect("failed to wait on child");
            assert!(status.success(), "sbt assembly failed");
        }
        Self {}
    }

    pub fn run<I, S>(&self, class_name: &str, parameters: I) -> std::io::Result<Child>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<std::ffi::OsStr>,
    {
        Command::new("java")
            .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../"))
            .args(["-Xmx32G", "-cp", "target/scala-2.12/microblossom.jar", class_name])
            .args(parameters)
            .spawn()
    }

    /// blocking call that gets the stdout
    pub fn get_output<I, S>(&self, class_name: &str, parameters: I) -> std::io::Result<String>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<std::ffi::OsStr>,
    {
        let output = Command::new("java")
            .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../"))
            .args(["-Xmx32G", "-cp", "target/scala-2.12/microblossom.jar", class_name])
            .args(parameters)
            .output()?;
        String::from_utf8(output.stdout).map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))
    }
}

lazy_static! {
    pub static ref SCALA_MICRO_BLOSSOM_RUNNER: ScalaMicroBlossomRunner = ScalaMicroBlossomRunner::new();
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn util_scala_micro_blossom_runner() {
        // cargo test util_scala_micro_blossom_runner -- --nocapture
        let help = SCALA_MICRO_BLOSSOM_RUNNER
            .get_output("microblossom.DualHost", vec!["--help"])
            .unwrap();
        println!("help: {help}");
    }
}
