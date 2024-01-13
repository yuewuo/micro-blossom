use crate::binding::*;
use crate::util::*;
use core::sync::atomic::{compiler_fence, Ordering};

/*
EMBEDDED_BLOSSOM_MAIN=test_axi4_timer make Xilinx && make -C ../../fpga/Xilinx/VMK180_AXI4_Timer
make -C ../../fpga/Xilinx/VMK180_AXI4_Timer run_a72
*/

/// check whether the get_time function works properly by using `nop_delay`
fn sanity_check() {
    let start = unsafe { extern_c::get_native_time() };
    compiler_fence(Ordering::SeqCst);
    nop_delay(2 * 1000 * 1000 * 5); // assuming the CPU is not faster than 5GHz, such delay should be more than 2ms
    compiler_fence(Ordering::SeqCst);
    let end = unsafe { extern_c::get_native_time() };
    println!("start: {start}");
    println!("end: {end}");
    if start == end {
        println!("[error] the timer has not changed as it supposed to be");
        panic!();
    }
    let diff = unsafe { extern_c::diff_native_time(start, end) };
    let time_per_nop = diff / 1.0e7;
    let estimated_frequency = 1. / time_per_nop;
    println!("diff: {diff}s after performing 10^7 nops");
    println!(
        "    roughly {}ns per nop or {} MHz",
        time_per_nop * 1.0e9,
        estimated_frequency / 1.0e6
    );
}

pub fn main() {
    println!("Test GetTime");

    println!("\n1. Sanity Check");
    sanity_check();

    println!("\n2. Test Sleep Function");
    println!("[start] sleep for 1s");
    sleep(1.);
    println!("[end] sleep for 1s");

    println!("\n3. Test AXI4 speed")
    let mut timer_benchmarker = Benchmarker::new(|| {
        unsafe { black_box(extern_c::get_native_time()) };
    });
    timer_benchmarker.autotune();
    timer_benchmarker.run(3);

    println!("\n4. Test alignment");
    for count in [3, 2, 1] {
        println!("    start in {count}");
        sleep(1.);
    }
    let mut start = unsafe { extern_c::get_native_time() };
    for idx in 0..10000 {
        loop {
            compiler_fence(Ordering::SeqCst);
            let end = unsafe { extern_c::get_native_time() };
            let diff = unsafe { extern_c::diff_native_time(start, end) } as f64;
            if diff >= idx as f64 {
                println!("tick {idx}");
                break;
            }
        }
    }
}
