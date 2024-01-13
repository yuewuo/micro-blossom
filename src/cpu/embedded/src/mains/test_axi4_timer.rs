use crate::binding::*;
use crate::util::*;
use core::hint::black_box;
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

    println!("\n3. Test AXI4 speed");
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
    let start = unsafe { extern_c::get_native_time() };
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

/*

Results:

## With full AXI4 (44 bit address, all 16 bit userId, awUser, arUser, etc.)

3. Test AXI4 speed
[benchmarker] autotune ... batch size = 9090907
[1/3] per_op: 110.00 ns, freq: 9.09091 MHz
[2/3] per_op: 110.00 ns, freq: 9.09091 MHz
[3/3] per_op: 110.00 ns, freq: 9.09091 MHz

## AXI4 Minimal with AXI4 SmartConnect in the middle

this will incur additional latency because according to AXI4 spec, userID width must be the same
and SmartConnect must did something in the middle to maintain the userID.
Since everything runs at 200MHz at the PL side, the additional 40ns corresponds to 8 clock cycles (5ns per clock cycle).
This is a little bit higher than I thought, so it's better to just use the wider AXI interface although it does consume
more resources.

3. Test AXI4 speed
[benchmarker] autotune ... batch size = 6641713
[1/3] per_op: 150.56 ns, freq: 6.64174 MHz
[2/3] per_op: 150.56 ns, freq: 6.64168 MHz
[3/3] per_op: 150.56 ns, freq: 6.64173 MHz


*/
