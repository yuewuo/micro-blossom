use crate::binding::*;
use crate::util::*;
use core::hint::black_box;

pub fn main() {
    println!("\n1. Simple BRAM read/write");
    let value = 1234;
    println!("write value: {}", value);
    unsafe { extern_c::test_write32(1234) };
    println!("read value: {}", unsafe { extern_c::test_read32() });

    println!("\n2. Write Speed Test");
    let mut write_benchmarker = Benchmarker::new(|| {
        unsafe { black_box(extern_c::test_write32(1234)) };
    });
    write_benchmarker.autotune();
    write_benchmarker.run(3);

    println!("\n3. Read Speed Test");
    let mut write_benchmarker = Benchmarker::new(|| {
        unsafe { black_box(extern_c::test_read32()) };
    });
    write_benchmarker.autotune();
    write_benchmarker.run(3);

    println!("\n4. Write-then-Read Speed Test");
    let mut write_benchmarker = Benchmarker::new(|| {
        unsafe { black_box(extern_c::test_write32(1234)) };
        unsafe { black_box(extern_c::test_read32()) };
    });
    write_benchmarker.autotune();
    write_benchmarker.run(3);

    println!("\n5. Read-then-Write Speed Test");
    let mut write_benchmarker = Benchmarker::new(|| {
        unsafe { black_box(extern_c::test_read32()) };
        unsafe { black_box(extern_c::test_write32(1234)) };
    });
    write_benchmarker.autotune();
    write_benchmarker.run(3);
}

/*

Results: APU is faster than RPU even when comparing latency
The round-trip time is 217ns in APU and it's 300ns in RPU, even though AXI in RPU is closer to the RPU without
going through complex interconnect.

A72:

2. Write Speed Test
[benchmarker] autotune
[benchmarker] automatic batch size = 46666124
[1/3] per_op: 16.43 ns, freq: 60.86893 MHz
[2/3] per_op: 16.43 ns, freq: 60.86893 MHz
[3/3] per_op: 21.43 ns, freq: 46.66619 MHz

3. Read Speed Test
[benchmarker] autotune
[benchmarker] automatic batch size = 7954451
[1/3] per_op: 125.72 ns, freq: 7.95446 MHz
[2/3] per_op: 125.72 ns, freq: 7.95446 MHz
[3/3] per_op: 125.72 ns, freq: 7.95446 MHz

4. Write-then-Read Speed Test
[benchmarker] autotune
[benchmarker] automatic batch size = 4615755
[1/3] per_op: 216.65 ns, freq: 4.61582 MHz
[2/3] per_op: 216.64 ns, freq: 4.61593 MHz
[3/3] per_op: 216.64 ns, freq: 4.61588 MHz

5. Read-then-Write Speed Test
[benchmarker] autotune
[benchmarker] automatic batch size = 4616127
[1/3] per_op: 216.63 ns, freq: 4.61606 MHz
[2/3] per_op: 216.62 ns, freq: 4.61627 MHz
[3/3] per_op: 216.63 ns, freq: 4.61622 MHz



R5F:

2. Write Speed Test
[benchmarker] autotune
[benchmarker] automatic batch size = 6451538
[1/3] per_op: 155.00 ns, freq: 6.45155 MHz
[2/3] per_op: 155.00 ns, freq: 6.45155 MHz
[3/3] per_op: 155.00 ns, freq: 6.45155 MHz

3. Read Speed Test
[benchmarker] autotune
[benchmarker] automatic batch size = 6896470
[1/3] per_op: 145.00 ns, freq: 6.89648 MHz
[2/3] per_op: 145.00 ns, freq: 6.89648 MHz
[3/3] per_op: 145.00 ns, freq: 6.89648 MHz

4. Write-then-Read Speed Test
[benchmarker] autotune
[benchmarker] automatic batch size = 3333294
[1/3] per_op: 300.00 ns, freq: 3.33330 MHz
[2/3] per_op: 300.00 ns, freq: 3.33330 MHz
[3/3] per_op: 300.00 ns, freq: 3.33330 MHz

5. Read-then-Write Speed Test
[benchmarker] autotune
[benchmarker] automatic batch size = 3389790
[1/3] per_op: 295.00 ns, freq: 3.38980 MHz
[2/3] per_op: 295.00 ns, freq: 3.38980 MHz
[3/3] per_op: 295.00 ns, freq: 3.38980 MHz


*/
