use crate::binding::*;
use crate::util::*;
use core::sync::atomic::{compiler_fence, Ordering};

/*
EMBEDDED_BLOSSOM_MAIN=test_get_time make Xilinx && make -C ../../fpga/Xilinx/VMK180_BRAM
make -C ../../fpga/Xilinx/VMK180_BRAM run_a72
make -C ../../fpga/Xilinx/VMK180_BRAM run_r5
*/

/// check whether the get_time function works properly by using `nop_delay`
pub fn sanity_check() {
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

    println!("\n3. Test alignment");
    for count in [3, 2, 1] {
        println!("    start in {count}");
        sleep(1.);
    }
    let mut start = unsafe { extern_c::get_native_time() };
    let mut global_diff = 0.;
    // note: this complex implementation is needed because some timer implementation is not capable of
    // recording long time difference, e.g., in Versal board A72 they have 32 bit timer clocked at 150MHz: only capable
    // of recording 28.6s difference. We need to actively accumulating the global timer.
    for idx in 0..10000 {
        loop {
            compiler_fence(Ordering::SeqCst);
            let end = unsafe { extern_c::get_native_time() };
            let local_diff = unsafe { extern_c::diff_native_time(start, end) } as f64;
            let diff = global_diff + local_diff;
            // avoid overflow by moving the start every 0.5s
            if local_diff > 0.5 {
                start = end;
                global_diff += local_diff;
            }
            if diff >= idx as f64 {
                println!("tick {idx}");
                break;
            }
        }
    }
}
