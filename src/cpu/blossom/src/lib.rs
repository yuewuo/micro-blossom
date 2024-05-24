pub mod cli;
pub mod dual_module_adaptor;
pub mod dual_module_axi4;
pub mod dual_module_comb;
pub mod dual_module_comb_edge;
pub mod dual_module_comb_offloading;
pub mod dual_module_comb_vertex;
pub mod dual_module_looper;
pub mod dual_module_scala;
pub mod mwpm_solver;
pub mod primal_module_embedded_adaptor;
pub mod resources;
pub mod simulation_tcp_client;
pub mod transform_syndromes;
pub mod util;

use lazy_static::lazy_static;
use std::sync::Mutex;

lazy_static! {
    /// any method that uses environment variable to pass a parameter should lock this;
    /// see dual_module_comb.test for example
    static ref ENV_PARAMETER_LOCK: Mutex<()> = Mutex::new(());
}
