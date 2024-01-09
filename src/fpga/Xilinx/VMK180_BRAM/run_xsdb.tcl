set name vmk180_bram

set run_r5 0
set run_a72 0
set reset_system 1
if { $argc < 1 } {
    puts "Usage: <run_r5|run_a72> [[quick]]"
    puts "Please try again."
    exit 1
} else {
    set target [lindex $argv 0]
    if { "$target" == "run_r5" } {
        set run_r5 1
    } elseif { "$target" == "run_a72" } {
        set run_a72 1
    } else {
        puts "unrecognized run target"
        exit 1
    }
    if { $argc ==  2} {
        set reset_system 0
    }
}

connect

# (optional) reset whole device and program the DPI image
if { $reset_system } {
    targets -set -nocase -filter {name =~"APU*"}
    rst
    device program ./${name}_vitis/${name}/hw/sdt/${name}.pdi
    # loadhw -hw ./${name}_vitis/${name}/export/${name}/hw/${name}.xsa
        # -mem-ranges [list {0x80000000 0x9fffffff} {0xa4000000 0xafffffff} {0xb0000000 0xbfffffff}]
}

# first reset both R5 and A72
targets -set -nocase -filter {name =~ "*R5*#0"}
rst -processor -clear-registers
targets -set -nocase -filter {name =~ "*A72*#0"}
rst -processor -clear-registers

# run on R5
if { $run_r5 } {
    targets -set -nocase -filter {name =~ "*R5*#0"}
    dow ./${name}_vitis/benchmark_r5/build/benchmark_r5.elf
    con
}

# run on A72
if { $run_a72 } {
    targets -set -nocase -filter {name =~ "*A72*#0"}
    dow ./${name}_vitis/benchmark_a72/build/benchmark_a72.elf
    con
}
