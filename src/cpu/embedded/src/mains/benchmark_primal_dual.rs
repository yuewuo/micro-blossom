use crate::binding::*;
use crate::dual_driver::*;
use crate::util::*;
use core::cell::UnsafeCell;
use core::hint::black_box;
use konst::{option, primitive::parse_usize, result::unwrap_ctx};
use micro_blossom_nostd::dual_driver_tracked::*;
use micro_blossom_nostd::dual_module_stackless::*;
use micro_blossom_nostd::interface::*;
use micro_blossom_nostd::primal_module_embedded::*;
use micro_blossom_nostd::util::*;

/*
EMBEDDED_BLOSSOM_MAIN=benchmark_primal_dual make aarch64
* simulation (in src/cpu/blossom)
EMBEDDED_BLOSSOM_MAIN=benchmark_primal_dual WITH_WAVEFORM=1 cargo run --release --bin embedded_simulator -- ../../../resources/syndromes/code_capacity_d3_p0.1.syndromes.json
* experiment (in this folder)
make -C ../../fpga/Xilinx/VMK180_Micro_Blossom clean
make -C ../../fpga/Xilinx/VMK180_Micro_Blossom DUAL_CONFIG_FILEPATH=$(pwd)/../../../resources/graphs/example_code_capacity_d3.json
make -C ../../fpga/Xilinx/VMK180_Micro_Blossom
make -C ../../fpga/Xilinx/VMK180_Micro_Blossom run_a72
*/

// guarantees decoding up to d=39
pub const MAX_NODE_NUM: usize = unwrap_ctx!(parse_usize(option::unwrap_or!(option_env!("MAX_NODE_NUM"), "65536")));

pub const MAX_CONFLICT_CHANNELS: usize = 8;

static mut PRIMAL_MODULE: UnsafeCell<PrimalModuleEmbedded<MAX_NODE_NUM>> = UnsafeCell::new(PrimalModuleEmbedded::new());
static mut DUAL_MODULE: UnsafeCell<DualModuleStackless<DualDriverTracked<DualDriver<MAX_CONFLICT_CHANNELS>, MAX_NODE_NUM>>> =
    UnsafeCell::new(DualModuleStackless::new(DualDriverTracked::new(DualDriver::new())));

pub fn main() {
    // obtain hardware information
    let hardware_info = unsafe { extern_c::get_hardware_info() };
    assert!(hardware_info.conflict_channels as usize <= MAX_CONFLICT_CHANNELS);
    assert!(hardware_info.conflict_channels >= 1);

    // create primal and dual modules
    let context_id = 0;
    let primal_module = unsafe { PRIMAL_MODULE.get().as_mut().unwrap() };
    let dual_module = unsafe { DUAL_MODULE.get().as_mut().unwrap() };
    dual_module
        .driver
        .driver
        .reconfigure(hardware_info.conflict_channels, context_id);

    primal_module.reset();
    dual_module.reset();

    println!("\n1. Primal Reset");
    let mut benchmarker = Benchmarker::new(|| {
        black_box(primal_module.reset());
    });
    benchmarker.autotune();
    benchmarker.run(3);

    println!("\n2. Dual Reset");
    let mut benchmarker = Benchmarker::new(|| {
        black_box(dual_module.reset());
    });
    benchmarker.autotune();
    benchmarker.run(3);

    println!("\n3. Dual Reset + Add Defect");
    let mut benchmarker = Benchmarker::new(|| {
        black_box(dual_module.add_defect(ni!(0), ni!(0)));
        black_box(dual_module.add_defect(ni!(1), ni!(1)));
        dual_module.reset();
    });
    benchmarker.autotune();
    benchmarker.run(3);

    println!("\n4. Dual Reset + Add Defect + Find Obstacle");
    let mut benchmarker = Benchmarker::new(|| {
        black_box(dual_module.add_defect(ni!(0), ni!(0)));
        black_box(dual_module.add_defect(ni!(1), ni!(1)));
        dual_module.find_obstacle();
        dual_module.reset();
    });
    benchmarker.autotune();
    benchmarker.run(3);

    println!("\n5. Dual Reset + Add Defect + Find Obstacle + Primal Resolve");
    let mut benchmarker = Benchmarker::new(|| {
        black_box(dual_module.add_defect(ni!(0), ni!(0)));
        black_box(dual_module.add_defect(ni!(1), ni!(1)));
        let (obstacle, _) = dual_module.find_obstacle();
        primal_module.resolve(dual_module, obstacle);
        dual_module.reset();
        primal_module.reset();
    });
    benchmarker.autotune();
    benchmarker.run(3);
}

/*
1. Primal Reset
[benchmarker] autotune ... batch size = 692480190
[1/3] per_op: 1.44 ns, freq: 692.48040 MHz
[2/3] per_op: 1.44 ns, freq: 692.48040 MHz
[3/3] per_op: 1.44 ns, freq: 692.48040 MHz

2. Dual Reset
[benchmarker] autotune ... batch size = 19718281
[1/3] per_op: 51.43 ns, freq: 19.44444 MHz
[2/3] per_op: 51.43 ns, freq: 19.44444 MHz
[3/3] per_op: 50.71 ns, freq: 19.71831 MHz

3. Dual Reset + Add Defect
[benchmarker] autotune ... batch size = 10523192
[1/3] per_op: 95.71 ns, freq: 10.44776 MHz
[2/3] per_op: 95.03 ns, freq: 10.52320 MHz
[3/3] per_op: 95.71 ns, freq: 10.44776 MHz

4. Dual Reset + Add Defect + Find Obstacle
[benchmarker] autotune ... batch size = 1492521
[1/3] per_op: 670.00 ns, freq: 1.49253 MHz
[2/3] per_op: 670.01 ns, freq: 1.49252 MHz
[3/3] per_op: 670.01 ns, freq: 1.49252 MHz

5. Dual Reset + Add Defect + Find Obstacle + Primal Resolve
[benchmarker] autotune ... batch size = 1257809
[1/3] per_op: 795.04 ns, freq: 1.25780 MHz
[2/3] per_op: 795.02 ns, freq: 1.25783 MHz
[3/3] per_op: 795.03 ns, freq: 1.25781 MHz
[exit]


I found `binding.c.obj` under src/fpga/Xilinx/VMK180_Micro_Blossom/vmk180_micro_blossom_vitis/benchmark_a72/build/CMakeFiles/benchmark_a72.elf.dir/

Run `/tools/Xilinx/Vitis/2023.2/gnu/aarch64/lin/aarch64-none/bin/aarch64-none-elf-objdump -D binding.c.obj > binding.asm`

The assembly ssems to be too complex.
Moreover, it doesn't seem to be the problem of the assembly code.
Even the BRAM access takes 127ns per read.
Each obstacle involves 127 * 3 = 381ns on pure reading,
adding at least 3 other write instructions of 66ns and some
dependencies between the write and read (about 575-381-66=128ns).
This does make sense somehow.
The only unreasonable thing is why it takes 381ns instead of 127ns + 44ns?
each write operation takes 22ns because the writes are in order and do not wait for complete.
Can't the CPU just issue 3 read transactions and let the hardware to process and then get back the results?
If the CPU can be extended with registers connected to the Micro Blossom module, can we eliminate this latency or reduce it to just tens of nanoseconds? (a few clock cycles at 200MHz)
These are open questions and doesn't seem to be solvable in the near term.

The `get_conflicts` function takes 580ns. (A little bit higher that I thought, because BRAM tests say only 380ns for memory access)
there are ~300 instructions in the `get_conflicts` function by checking the assembly code.
Suppose the clock frequency is 3.4GHz, then this takes about 100ns to run if the pipeline is full (ideally),
so it does make sense to have about 200ns additional time to run this function considering the branching things and stalls.

look at the `ldr` instructions. Those are reading from the Micro Blossom module.

 31c:	5d75afb0 	.inst	0x5d75afb0 ; undefined
 320:	1bcf448f 	.inst	0x1bcf448f ; undefined
 324:	bdcf2445 	.inst	0xbdcf2445 ; undefined
 328:	bd74fbbb 	ldr	s27, [x29, #13560]
 32c:	def6a0c7 	.inst	0xdef6a0c7 ; undefined
 330:	90e4461b 	adrp	x27, ffffffffc88c0000 <binding.c.d896e387+0xffffffffc88c0000>
 334:	10521c28 	adr	x8, a46b8 <binding.c.d896e387+0xa46b8>


 394:	0420420d 	index	z13.b, #-16, #0
 398:	84361a05 	prfb	pldl3strm, p6, [x16, z22.s, uxtw]
 39c:	3ac8b1f8 	.inst	0x3ac8b1f8 ; undefined
 3a0:	fc471d5e 	ldr	d30, [x10, #113]!
 3a4:	9c542886 	ldr	q6, a88b4 <binding.c.d896e387+0xa88b4>
 3a8:	90b715f8 	adrp	x24, ffffffff6e2bc000 <binding.c.d896e387+0xffffffff6e2bc000>
 3ac:	242d2c3f 	cmpls	p15.b, p3/z, z1.b, #52
 3b0:	388aeb52 	ldtrsb	x18, [x26, #174]
 3b4:	3c55b5a5 	ldr	b5, [x13], #-165
 3b8:	5d88fd62 	.inst	0x5d88fd62 ; undefined
 3bc:	178eb818 	b	fffffffffe3ae41c <binding.c.d896e387+0xfffffffffe3ae41c>
 3c0:	16aec67e 	b	fffffffffabb1db8 <binding.c.d896e387+0xfffffffffabb1db8>

*/
