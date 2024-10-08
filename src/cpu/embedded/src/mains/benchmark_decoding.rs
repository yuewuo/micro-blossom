use crate::binding::*;
use crate::defects_reader::*;
use crate::dual_driver::*;
use crate::util::*;
use core::cell::UnsafeCell;
use core::hint::black_box;
use include_bytes_plus::include_bytes;
use konst::{option, primitive::parse_usize, result::unwrap_ctx};
use micro_blossom_nostd::dual_driver_tracked::*;
use micro_blossom_nostd::dual_module_stackless::*;
use micro_blossom_nostd::instruction::*;
use micro_blossom_nostd::interface::*;
use micro_blossom_nostd::latency_benchmarker::*;
use micro_blossom_nostd::primal_module_embedded::*;
#[allow(unused_imports)]
use num_traits::float::FloatCore;

/*
cp ../../../resources/syndromes/code_capacity_d3_p0.1.syndromes.defects ../embedded/embedded.defects
EMBEDDED_BLOSSOM_MAIN=benchmark_decoding make aarch64
* simulation (in src/cpu/blossom)
EMBEDDED_BLOSSOM_MAIN=benchmark_decoding SUPPORT_LAYER_FUSION=1 SUPPORT_LOAD_STALL_EMULATOR=1 WITH_WAVEFORM=1 NUM_LAYER_FUSION=1 cargo run --release --bin embedded_simulator -- ../../../resources/syndromes/code_capacity_d3_p0.1.syndromes.json
EMBEDDED_BLOSSOM_MAIN=benchmark_decoding SUPPORT_LAYER_FUSION=1 SUPPORT_LOAD_STALL_EMULATOR=1 SUPPORT_OFFLOADING=1 WITH_WAVEFORM=1 NUM_LAYER_FUSION=1 cargo run --release --bin embedded_simulator -- ../../../resources/syndromes/code_capacity_d3_p0.1.syndromes.json
* experiment (in this folder)
make -C ../../fpga/Xilinx/VMK180_Micro_Blossom clean
make -C ../../fpga/Xilinx/VMK180_Micro_Blossom DUAL_CONFIG_FILEPATH=$(pwd)/../../../resources/graphs/example_code_capacity_d3.json
make -C ../../fpga/Xilinx/VMK180_Micro_Blossom
make -C ../../fpga/Xilinx/VMK180_Micro_Blossom run_a72
*/

// guarantees decoding up to d=39
pub const MAX_NODE_NUM: usize = unwrap_ctx!(parse_usize(option::unwrap_or!(option_env!("MAX_NODE_NUM"), "65536")));
pub const DEFECTS: &'static [u32] = &include_bytes!("./embedded.defects" as u32le);

/// by default using batch decoding
pub const USE_LAYER_FUSION: bool = option_env!("USE_LAYER_FUSION").is_some();
/// if layer fusion is enabled, we use this value as interval; by default 1us = 1000ns
pub const MEASUREMENT_CYCLE_NS: usize =
    unwrap_ctx!(parse_usize(option::unwrap_or!(option_env!("MEASUREMENT_CYCLE_NS"), "1000")));
/// the number of layer fusion
pub const NUM_LAYER_FUSION: usize = unwrap_ctx!(parse_usize(option::unwrap_or!(option_env!("NUM_LAYER_FUSION"), "0")));
/// enable multiple fusion at once, by default enable
pub const MULTIPLE_FUSION: bool = !option_env!("DISABLE_MULTIPLE_FUSION").is_some();
pub const IGNORE_EMPTY_DEFECT: bool = option_env!("IGNORE_EMPTY_DEFECT").is_some();
pub const MAX_ROUND: usize = unwrap_ctx!(parse_usize(option::unwrap_or!(option_env!("MAX_ROUND"), "0")));
/// disabling the detail print will significantly speed up the process
pub const DISABLE_DETAIL_PRINT: bool = option_env!("DISABLE_DETAIL_PRINT").is_some();

static mut PRIMAL_MODULE: UnsafeCell<PrimalModuleEmbedded<MAX_NODE_NUM>> = UnsafeCell::new(PrimalModuleEmbedded::new());
static mut DUAL_MODULE: UnsafeCell<DualModuleStackless<DualDriverTracked<DualDriver, MAX_NODE_NUM>>> =
    UnsafeCell::new(DualModuleStackless::new(DualDriverTracked::new(DualDriver::new())));
static mut LATENCY_BENCHMARKER: UnsafeCell<LatencyBenchmarker> = UnsafeCell::new(LatencyBenchmarker::new_default());
static mut CPU_WALL_BENCHMARKER: UnsafeCell<LatencyBenchmarker> = UnsafeCell::new(LatencyBenchmarker::new_default());

pub fn main() {
    // first print all the configurations for trace
    println!("------- start of build parameters -------");
    println!("USE_LAYER_FUSION: {USE_LAYER_FUSION:?}");
    println!("MEASUREMENT_CYCLE_NS: {MEASUREMENT_CYCLE_NS:?}");
    println!("NUM_LAYER_FUSION: {NUM_LAYER_FUSION:?}");
    println!("MULTIPLE_FUSION: {MULTIPLE_FUSION:?}");
    println!("IGNORE_EMPTY_DEFECT: {IGNORE_EMPTY_DEFECT:?}");
    println!("MAX_ROUND: {MAX_ROUND:?}");
    println!("DISABLE_DETAIL_PRINT: {DISABLE_DETAIL_PRINT:?}");
    println!("-------- end of build parameters --------");

    // obtain hardware information
    let hardware_info = unsafe { extern_c::get_hardware_info() };
    assert!(hardware_info.conflict_channels == 1);
    println!("hardware_info: {hardware_info:?}");
    unsafe { hardware_info.reset_all() };
    assert!(hardware_info.flags.contains(
        extern_c::MicroBlossomHardwareFlags::SUPPORT_LAYER_FUSION
            | extern_c::MicroBlossomHardwareFlags::SUPPORT_LOAD_STALL_EMULATOR
    ));
    assert!(NUM_LAYER_FUSION > 0, "must contain at least 1 layer fusion");
    let native_frequency = unsafe { extern_c::get_native_frequency() };
    println!("native_frequency: {:.3}MHz", native_frequency * 1e-6);
    let measurement_cycle_native = ((MEASUREMENT_CYCLE_NS as f32) * 1e-9 * native_frequency) as u64;
    let actual_measurement_cycle = unsafe { extern_c::diff_native_time(0, measurement_cycle_native) };
    println!(
        "measurement_cycle_native: {measurement_cycle_native}, actual measurement cycle: {:.3}us",
        actual_measurement_cycle * 1e6
    );
    // delay the syndrome by 1us to make sure the syndrome is stalled at the first decoding instance
    let syndrome_start_delay_ns = 1000u64;
    let syndrome_start_delay_cycle = ((syndrome_start_delay_ns as f32) * 1e-9 * native_frequency) as u64;

    // create primal and dual modules
    let context_id = 0;
    let primal_module = unsafe { PRIMAL_MODULE.get().as_mut().unwrap() };
    // adapt bit width of primal module so that node index will not overflow
    primal_module.nodes.blossom_begin = (1 << hardware_info.vertex_bits) / 2;
    let dual_module = unsafe { DUAL_MODULE.get().as_mut().unwrap() };
    dual_module.driver.driver.context_id = context_id;
    let mut defects_reader = DefectsReader::new(DEFECTS);
    // calculate useful constant across the evaluations
    let finish_delta = if USE_LAYER_FUSION {
        (NUM_LAYER_FUSION as u64 - 1) * (measurement_cycle_native as u64)
    } else {
        0
    };
    let interval = if USE_LAYER_FUSION { measurement_cycle_native } else { 0 } as u32;

    // optional: test whether the fast timer aligns with the native timer
    // test_fast_timer();

    let latency_benchmarker = unsafe { LATENCY_BENCHMARKER.get().as_mut().unwrap() };
    let cpu_wall_benchmarker = unsafe { CPU_WALL_BENCHMARKER.get().as_mut().unwrap() };
    let all_begin_native_time = unsafe { extern_c::get_native_time() };
    let mut last_native_time = unsafe { extern_c::get_native_time() };
    while let Some(defects) = defects_reader.next() {
        if IGNORE_EMPTY_DEFECT && defects.is_empty() {
            continue;
        }
        unsafe { extern_c::clear_instruction_counter() };
        // reset and load defects
        for (node_index, &vertex_index) in defects.iter().enumerate() {
            dual_module.add_defect(ni!(vertex_index), ni!(node_index));
        }
        let mut layer_id = 0;
        if !USE_LAYER_FUSION {
            // load all layers except for 1
            for layer_id in 0..NUM_LAYER_FUSION - 1 {
                unsafe {
                    extern_c::execute_instruction(Instruction32::load_syndrome_external(ni!(layer_id)).into(), context_id)
                };
            }
            layer_id = NUM_LAYER_FUSION - 1;
        };
        // start timer and set up load stall emulator: the syndrome won't be loaded until this time has passed
        let native_start = unsafe { extern_c::get_native_time() };
        let fast_start = unsafe { extern_c::get_fast_cpu_time() };
        let syndrome_start = native_start + syndrome_start_delay_cycle; // wait for setting up everything
        let syndrome_finish = syndrome_start + finish_delta;
        unsafe { extern_c::setup_load_stall_emulator(syndrome_start, interval, context_id) };
        // solve it
        while layer_id < NUM_LAYER_FUSION {
            unsafe {
                extern_c::execute_instruction(Instruction32::load_syndrome_external(ni!(layer_id)).into(), context_id)
            };
            layer_id += 1;
            if USE_LAYER_FUSION && MULTIPLE_FUSION {
                // fuse multiple layers at once if it can be done without any stall
                let duration_ns = unsafe { extern_c::get_fast_cpu_duration_ns(fast_start) };
                while duration_ns > syndrome_start_delay_ns + (layer_id as u64) * (MEASUREMENT_CYCLE_NS as u64)
                    && layer_id < NUM_LAYER_FUSION
                {
                    unsafe {
                        extern_c::execute_instruction(
                            Instruction32::load_syndrome_external(ni!(layer_id)).into(),
                            context_id,
                        )
                    };
                    layer_id += 1;
                }
            }
            // solve until no obstacle is found
            let (mut obstacle, _) = dual_module.find_obstacle();
            while !obstacle.is_none() {
                // println!("obstacle: {obstacle:?}");
                primal_module.resolve(dual_module, obstacle);
                (obstacle, _) = dual_module.find_obstacle();
            }
        }
        let cpu_wall_diff = (unsafe { extern_c::get_fast_cpu_duration_ns(fast_start) } as f64) * 1e-9;
        let counter = unsafe { extern_c::get_instruction_counter() };
        // get time from hardware
        let load_time = unsafe { extern_c::get_last_load_time(context_id) };
        assert!(
            load_time >= syndrome_finish,
            "load stall emulator enforces that syndrome is not loaded before it's ready"
        );
        let finish_time = unsafe { extern_c::get_last_finish_time(context_id) };
        let hardware_diff = unsafe { extern_c::diff_native_time(syndrome_finish, finish_time) } as f64;
        if !DISABLE_DETAIL_PRINT {
            println!(
                "[{}] time: {:.3}us, counter: {counter}, wall: {:.3}us",
                defects_reader.count,
                hardware_diff * 1e6,
                cpu_wall_diff * 1e6
            );
        }
        latency_benchmarker.record(hardware_diff);
        cpu_wall_benchmarker.record(cpu_wall_diff);
        primal_module.reset();
        dual_module.reset();
        // early break if reaching the limit
        if MAX_ROUND != 0 {
            if defects_reader.count >= MAX_ROUND {
                break;
            }
        }
        // print something every 5s
        if unsafe { extern_c::diff_native_time(last_native_time, native_start) } > 5. {
            println!("[info] have run {} samples", defects_reader.count);
            last_native_time = native_start;
            println!("latency_benchmarker statistics:");
            latency_benchmarker.print_statistics();
        }
    }
    // print out results
    if !DISABLE_DETAIL_PRINT {
        cpu_wall_benchmarker.debug_println();
        latency_benchmarker.debug_println();
    }
    print!("cpu_wall_benchmarker");
    cpu_wall_benchmarker.println();
    print!("latency_benchmarker");
    latency_benchmarker.println();
    println!("cpu_wall_benchmarker statistics:");
    cpu_wall_benchmarker.print_statistics();
    println!("latency_benchmarker statistics:");
    latency_benchmarker.print_statistics();
    // print overall time consumption for use of estimation
    let all_end_native_time = unsafe { extern_c::get_native_time() };
    let all_duration = unsafe { extern_c::diff_native_time(all_begin_native_time, all_end_native_time) };
    println!("evaluation duration: {all_duration}s (running the benchmark)");
    let overall_duration = unsafe { extern_c::diff_native_time(0, all_end_native_time) };
    println!("overall duration: {overall_duration}s (from FPGA boot to program end)");
}

pub fn test_fast_timer() {
    println!("\nBenchmark Fast Time Speed");
    let begin_native = unsafe { black_box(extern_c::get_native_time()) };
    let begin_fast = unsafe { black_box(extern_c::get_fast_cpu_time()) };
    // run benchmark
    let mut benchmarker = Benchmarker::new(|| {
        unsafe { black_box(extern_c::get_fast_cpu_time()) };
    });
    benchmarker.autotune();
    benchmarker.run(3);
    // benchmark finished, now see whether the duration agrees with each other (up to 5% difference)
    let epsilon = 0.05;
    let duration_fast = unsafe { black_box(extern_c::get_fast_cpu_duration_ns(begin_fast)) };
    let end_native = unsafe { black_box(extern_c::get_native_time()) };
    let duration_native = unsafe { extern_c::diff_native_time(begin_native, end_native) };
    println!("native duration: {}ns", duration_native * 1e9);
    println!("fast duration: {}ns", duration_fast);
    assert!((duration_native * 1e9 - duration_fast as f32).abs() <= duration_native * 1e9 * epsilon);
}

/*

Result: handling a conflict takes about 670ns, including computation and the initial transaction.
It seems like the computation is extremely fast (670ns - 590ns = 80ns) per pair of defects.
The most time-consuming operation is reading the bus... about 127ns * 3 = 381ns (according to `test_bram.rs`)
This could potentially be reduced if read operation could happen in a burst? Is this possible in Xilinx library?
This roughly agrees with the results in `benchmark_primal_simple_match.rs`.
The computation time corresponds to only 16 clock cycle at the hardware, and the hardware, if fully pipelined, only requires 5 clock cycles.
It seems like about 3 CPUs could make the FPGA busy.

[974] time: 0.0000005900000132896821, counter: 2
[975] time: 0.000001249999968422344, counter: 5
[976] time: 0.000001249999968422344, counter: 5
[977] time: 0.0000012550000292321783, counter: 5
[978] time: 0.0000005900000132896821, counter: 2
[979] time: 0.0000005900000132896821, counter: 2
[980] time: 0.0000012550000292321783, counter: 5
[981] time: 0.000001249999968422344, counter: 5
[982] time: 0.0000005900000132896821, counter: 2
[983] time: 0.000001259999976355175, counter: 5
[984] time: 0.0000005850000093232666, counter: 2
[985] time: 0.0000005900000132896821, counter: 2
[986] time: 0.0000013899999657951412, counter: 7
[987] time: 0.0000005900000132896821, counter: 2
[988] time: 0.0000005900000132896821, counter: 2
[989] time: 0.0000005900000132896821, counter: 2

2024.6.4 We have redesigned the AXI4 bus module and define a new register interface such that
reading an obstacle only takes a single 128 bit read. As shown in benchmark/hardware/bram_speed,
this uses AXI4 read burst and consumes roughly the same time as a 64 bit read. In fact, 128 bit
read is the maximum AXI4 burst that can be triggered using `memcpy`. Using this new interface,
we evaluate the performance.

Now this script assumes that layer fusion is enabled.

* debugging using code capacity noise
cp ../../../resources/syndromes/code_capacity_planar_d3_p0.05.syndromes.defects ../embedded/embedded.defects
*     no offloading
EMBEDDED_BLOSSOM_MAIN=benchmark_decoding WITH_WAVEFORM=1 SUPPORT_LAYER_FUSION=1 SUPPORT_LOAD_STALL_EMULATOR=1 MAX_ROUND=100 \
    NUM_LAYER_FUSION=3 cargo run --release --bin embedded_simulator -- \
    ../../../resources/syndromes/code_capacity_planar_d3_p0.05.syndromes.json
*     with offloading
EMBEDDED_BLOSSOM_MAIN=benchmark_decoding WITH_WAVEFORM=1 SUPPORT_LAYER_FUSION=1 SUPPORT_LOAD_STALL_EMULATOR=1 MAX_ROUND=100 \
    SUPPORT_OFFLOADING=1 \
    NUM_LAYER_FUSION=3 cargo run --release --bin embedded_simulator -- \
    ../../../resources/syndromes/code_capacity_planar_d3_p0.05.syndromes.json
*     layer fusion with offloading
EMBEDDED_BLOSSOM_MAIN=benchmark_decoding WITH_WAVEFORM=1 SUPPORT_LAYER_FUSION=1 SUPPORT_LOAD_STALL_EMULATOR=1 MAX_ROUND=100 \
    SUPPORT_OFFLOADING=1 USE_LAYER_FUSION=1 \
    NUM_LAYER_FUSION=3 cargo run --release --bin embedded_simulator -- \
    ../../../resources/syndromes/code_capacity_planar_d3_p0.05.syndromes.json

* debugging using circuit-level noise (too slow)
cp ../../../resources/syndromes/circuit_level_d3_p0.001.syndromes.defects ../embedded/embedded.defects
EMBEDDED_BLOSSOM_MAIN=benchmark_decoding WITH_WAVEFORM=1 SUPPORT_LAYER_FUSION=1 SUPPORT_LOAD_STALL_EMULATOR=1 NUM_LAYER_FUSION=4 cargo run --release --bin embedded_simulator -- ../../../resources/syndromes/circuit_level_d3_p0.001.syndromes.json

*/
