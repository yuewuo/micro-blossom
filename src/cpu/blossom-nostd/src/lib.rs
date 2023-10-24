#![cfg_attr(all(not(test), not(feature = "std")), no_std)]
#![feature(option_result_unwrap_unchecked)]

pub mod benchmark;
pub mod blossom_tracker;
pub mod dual_module_stackless;
pub mod heapless;
pub mod interface;
pub mod primal_module_embedded;
pub mod primal_nodes;
pub mod util;
