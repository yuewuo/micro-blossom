set name vmk180_micro_blossom

set run_a72 0
set reset_system 1
if { $argc < 1 } {
    puts "Usage: <run_a72> [[quick]]"
    puts "Please try again."
    exit 1
} else {
    set target [lindex $argv 0]
    if { "$target" == "run_a72" } {
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
}

# first reset A72
targets -set -nocase -filter {name =~ "*A72*#0"}
rst -processor -clear-registers

# run on A72
if { $run_a72 } {
    targets -set -nocase -filter {name =~ "*A72*#0"}
    dow ./${name}_vitis/benchmark_a72/build/benchmark_a72.elf
    con
}
