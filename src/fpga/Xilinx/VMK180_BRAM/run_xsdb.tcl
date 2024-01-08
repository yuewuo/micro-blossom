set name vmk180_bram

connect

# (optional) reset whole device and program the DPI image
targets -set -nocase -filter {name =~"APU*"}
rst
device program ./${name}_vitis/${name}/hw/sdt/${name}.pdi

# loadhw -hw ./${name}_vitis/${name}/export/${name}/hw/${name}.xsa -mem-ranges [list {0x80000000 0x9fffffff} {0xa4000000 0xafffffff} {0xb0000000 0xbfffffff}]

# run on R5
targets -set -nocase -filter {name =~ "*R5*#0"}
rst -processor -clear-registers
dow ./${name}_vitis/benchmark_r5/build/benchmark_r5.elf
con


# run on A72
targets -set -nocase -filter {name =~ "*A72*#0"}
rst -processor -clear-registers
dow ./${name}_vitis/benchmark_a72/build/benchmark_a72.elf
con
