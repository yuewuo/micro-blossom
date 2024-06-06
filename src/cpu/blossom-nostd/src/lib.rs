#![cfg_attr(all(not(test), not(feature = "std")), no_std)]
#![cfg_attr(feature = "hls", feature(option_result_unwrap_unchecked))]
#![feature(const_fn_floating_point_arithmetic)]

pub mod benchmark;
pub mod blossom_tracker;
pub mod dual_driver_tracked;
pub mod dual_module_stackless;
pub mod heapless;
pub mod instruction;
pub mod interface;
pub mod latency_benchmarker;
pub mod layer_fusion;
pub mod nonmax;
pub mod primal_module_embedded;
pub mod primal_nodes;
pub mod util;
