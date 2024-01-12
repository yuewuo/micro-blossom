use crate::binding::*;
use crate::util::*;
use core::hint::black_box;

pub fn main() {
    println!("\n1. Simple BRAM read/write");
    let value = 1234;
    println!("write value: {}", value);
    unsafe { extern_c::test_write32(0, 1234) };
    println!("read value: {}", unsafe { extern_c::test_read32(0) });
    for i in 0..4 {
        unsafe { extern_c::test_write32(4 * i, i * 1000) };
    }
    for i in 0..4 {
        let expected = i * 1000;
        let value = unsafe { extern_c::test_read32(4 * i) };
        println!("read value expect to be {expected}, real is {value}");
        if value != expected {
            println!("abort");
            panic!();
        }
    }

    println!("\n2. Write Speed Test");
    let mut write_benchmarker = Benchmarker::new(|| {
        unsafe { black_box(extern_c::test_write32(0, 1234)) };
    });
    write_benchmarker.autotune();
    write_benchmarker.run(3);

    println!("\n3. Read Speed Test");
    let mut write_benchmarker = Benchmarker::new(|| {
        unsafe { black_box(extern_c::test_read32(0)) };
    });
    write_benchmarker.autotune();
    write_benchmarker.run(3);

    println!("\n4. Write-then-Read Speed Test");
    let mut write_benchmarker = Benchmarker::new(|| {
        unsafe { black_box(extern_c::test_write32(0, 1234)) };
        unsafe { black_box(extern_c::test_read32(0)) };
    });
    write_benchmarker.autotune();
    write_benchmarker.run(3);

    println!("\n5. Read-then-Write Speed Test");
    let mut write_benchmarker = Benchmarker::new(|| {
        unsafe { black_box(extern_c::test_read32(0)) };
        unsafe { black_box(extern_c::test_write32(0, 1234)) };
    });
    write_benchmarker.autotune();
    write_benchmarker.run(3);

    println!("\n6. Batch Write Test");
    let mut write_benchmarker = Benchmarker::new(|| {
        unsafe { black_box(extern_c::test_write32(0, 0)) };
        unsafe { black_box(extern_c::test_write32(4, 1)) };
        unsafe { black_box(extern_c::test_write32(8, 2)) };
        unsafe { black_box(extern_c::test_write32(12, 3)) };
    });
    write_benchmarker.inner_loops = 4;
    write_benchmarker.autotune();
    write_benchmarker.run(3);

    println!("\n7. Batch Read Test");
    let mut write_benchmarker = Benchmarker::new(|| {
        unsafe { black_box(extern_c::test_read32(0)) };
        unsafe { black_box(extern_c::test_read32(4)) };
        unsafe { black_box(extern_c::test_read32(8)) };
        unsafe { black_box(extern_c::test_read32(12)) };
    });
    write_benchmarker.inner_loops = 4;
    write_benchmarker.autotune();
    write_benchmarker.run(3);
}

/*

Results: APU is faster than RPU even when comparing latency
The round-trip time is 217ns in APU and it's 300ns in RPU, even though AXI in RPU is closer to the RPU without
going through complex interconnect.

A72:

2. Write Speed Test
[benchmarker] autotune ... batch size = 37837354
[1/3] per_op: 26.43 ns, freq: 37.83745 MHz
[2/3] per_op: 26.43 ns, freq: 37.83745 MHz
[3/3] per_op: 26.43 ns, freq: 37.83745 MHz

3. Read Speed Test
[benchmarker] autotune ... batch size = 7893760
[1/3] per_op: 126.72 ns, freq: 7.89147 MHz
[2/3] per_op: 126.71 ns, freq: 7.89224 MHz
[3/3] per_op: 126.62 ns, freq: 7.89745 MHz

4. Write-then-Read Speed Test
[benchmarker] autotune ... batch size = 4605211
[1/3] per_op: 217.14 ns, freq: 4.60522 MHz
[2/3] per_op: 217.14 ns, freq: 4.60522 MHz
[3/3] per_op: 217.14 ns, freq: 4.60522 MHz

5. Read-then-Write Speed Test
[benchmarker] autotune ... batch size = 4605210
[1/3] per_op: 217.14 ns, freq: 4.60522 MHz
[2/3] per_op: 217.14 ns, freq: 4.60522 MHz
[3/3] per_op: 217.14 ns, freq: 4.60522 MHz

6. Batch Write Test
[benchmarker] autotune ... batch size = 9459338
[1/3] per_op: 26.43 ns, freq: 37.83745 MHz
[2/3] per_op: 26.43 ns, freq: 37.83745 MHz
[3/3] per_op: 26.43 ns, freq: 37.83745 MHz

7. Batch Read Test
[benchmarker] autotune ... batch size = 1973655
[1/3] per_op: 126.71 ns, freq: 7.89208 MHz
[2/3] per_op: 126.72 ns, freq: 7.89147 MHz
[3/3] per_op: 126.63 ns, freq: 7.89712 MHz




R5F:

2. Write Speed Test
[benchmarker] autotune ... batch size = 6060536
[1/3] per_op: 160.00 ns, freq: 6.24994 MHz
[2/3] per_op: 160.00 ns, freq: 6.24994 MHz
[3/3] per_op: 165.00 ns, freq: 6.06054 MHz

3. Read Speed Test
[benchmarker] autotune ... batch size = 6060535
[1/3] per_op: 165.00 ns, freq: 6.06054 MHz
[2/3] per_op: 165.00 ns, freq: 6.06054 MHz
[3/3] per_op: 165.00 ns, freq: 6.06054 MHz

4. Write-then-Read Speed Test
[benchmarker] autotune ... batch size = 3076887
[1/3] per_op: 325.00 ns, freq: 3.07689 MHz
[2/3] per_op: 325.00 ns, freq: 3.07689 MHz
[3/3] per_op: 325.00 ns, freq: 3.07689 MHz

5. Read-then-Write Speed Test
[benchmarker] autotune ... batch size = 3124963
[1/3] per_op: 320.00 ns, freq: 3.12497 MHz
[2/3] per_op: 320.00 ns, freq: 3.12497 MHz
[3/3] per_op: 320.00 ns, freq: 3.12497 MHz

6. Batch Write Test
[benchmarker] autotune ... batch size = 1562481
[1/3] per_op: 160.00 ns, freq: 6.24994 MHz
[2/3] per_op: 161.25 ns, freq: 6.20149 MHz
[3/3] per_op: 160.00 ns, freq: 6.24994 MHz

7. Batch Read Test
[benchmarker] autotune ... batch size = 1550369
[1/3] per_op: 161.25 ns, freq: 6.20149 MHz
[2/3] per_op: 161.25 ns, freq: 6.20149 MHz
[3/3] per_op: 161.25 ns, freq: 6.20149 MHz


*/
