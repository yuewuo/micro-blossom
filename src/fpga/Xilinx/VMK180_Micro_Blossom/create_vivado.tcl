set name vmk180_micro_blossom
set ip_name MicroBlossomBus

if { $argc != 2 } {
    puts "Usage: <clock frequency in MHz> <slow clock division>"
    puts "Please try again."
    exit 1
} else {
    scan [lindex $argv 0] %f clock_frequency
    scan [lindex $argv 1] %f clock_divide_by
}
set slow_clock_frequency [expr {$clock_frequency / $clock_divide_by}]

create_project ${name} ./${name}_vivado -part xcvm1802-vsva2197-2MP-e-S
set_property board_part xilinx.com:vmk180:part0:3.2 [current_project]
create_bd_design "${name}" -mode batch

instantiate_example_design -template xilinx.com:design:Versal_APU_RPU_perf:1.0 -design ${name} -options { Preset.VALUE LPDDR4 }


# create IP
if { [file exists "${name}_verilog/$ip_name.v"] == 0} {               
    puts "Error: verilog file not generated, expecting ${name}_verilog/$ip_name.v"
    exit 1
}
ipx::infer_core -vendor user.org -library user -taxonomy /UserIP ./${name}_verilog
ipx::unload_core ${name}_verilog/component.xml
set_property ip_repo_paths ${name}_verilog [current_project]
update_ip_catalog -rebuild -scan_changes


# configure CIPS
# expose AXI_FPD
# expose a PL reset
set_property -dict [list \
  CONFIG.PS_PMC_CONFIG { \
    PS_USE_PMCPL_CLK0 {1} \
    PS_USE_M_AXI_FPD {1} \
    PS_M_AXI_FPD_DATA_WIDTH {64} \
    PS_NUM_FABRIC_RESETS {1} \
  } \
] [get_bd_cells versal_cips_0]
set_property CONFIG.PS_PMC_CONFIG "PMC_CRP_PL0_REF_CTRL_FREQMHZ $clock_frequency" [get_bd_cells versal_cips_0]

# create clock with integer division
create_bd_cell -type ip -vlnv xilinx.com:ip:clk_wizard:1.0 clk_wizard_0
set_property CONFIG.PRIM_SOURCE {Global_buffer} [get_bd_cells clk_wizard_0]
connect_bd_net [get_bd_pins versal_cips_0/pl0_ref_clk] [get_bd_pins clk_wizard_0/clk_in1]
set_property CONFIG.CLKOUT_USED "true,true" [get_bd_cells clk_wizard_0]
set_property CONFIG.CLKOUT_REQUESTED_PHASE "0.000,0.000" [get_bd_cells clk_wizard_0]
set_property CONFIG.CLKOUT_REQUESTED_OUT_FREQUENCY "$clock_frequency,$slow_clock_frequency" [get_bd_cells clk_wizard_0]

# create and connect my AXI4 IP
create_bd_cell -type ip -vlnv user.org:user:${ip_name}:1.0 ${ip_name}_0
connect_bd_intf_net [get_bd_intf_pins ${ip_name}_0/s0] [get_bd_intf_pins versal_cips_0/M_AXI_FPD]
connect_bd_net [get_bd_pins ${ip_name}_0/clk] [get_bd_pins clk_wizard_0/clk_out1]
connect_bd_net [get_bd_pins ${ip_name}_0/slow_clk] [get_bd_pins clk_wizard_0/clk_out2]
connect_bd_net [get_bd_pins ${ip_name}_0/clk] [get_bd_pins versal_cips_0/m_axi_fpd_aclk]

# create reset system
create_bd_cell -type ip -vlnv xilinx.com:ip:proc_sys_reset:5.0 proc_sys_reset_0
connect_bd_net [get_bd_pins proc_sys_reset_0/peripheral_reset] [get_bd_pins ${ip_name}_0/reset]
connect_bd_net [get_bd_pins proc_sys_reset_0/ext_reset_in] [get_bd_pins versal_cips_0/pl0_resetn]
connect_bd_net [get_bd_pins ${ip_name}_0/slow_clk] [get_bd_pins proc_sys_reset_0/slowest_sync_clk]

# assign address
assign_bd_address -target_address_space /versal_cips_0/M_AXI_FPD [get_bd_addr_segs ${ip_name}_0/s0/reg0] -force
set_property range 4M [get_bd_addr_segs versal_cips_0/M_AXI_FPD/SEG_${ip_name}_0_reg0]
set_property offset 0x400000000 [get_bd_addr_segs versal_cips_0/M_AXI_FPD/SEG_${ip_name}_0_reg0]

# create an ILA to monitor the transactions
# create_bd_cell -type ip -vlnv xilinx.com:ip:axis_ila:1.2 axis_ila_0
# set_property -dict [list \
#   CONFIG.C_MON_TYPE {Interface_Monitor} \
#   CONFIG.C_NUM_MONITOR_SLOTS {1} \
#   CONFIG.C_NUM_OF_PROBES {1} \
# ] [get_bd_cells axis_ila_0]
# connect_bd_intf_net [get_bd_intf_pins axis_ila_0/SLOT_0_AXI] [get_bd_intf_pins versal_cips_0/M_AXI_FPD]
# connect_bd_net [get_bd_pins axis_ila_0/clk] [get_bd_pins ${ip_name}_0/clk]
# connect_bd_net [get_bd_pins axis_ila_0/resetn] [get_bd_pins proc_sys_reset_0/peripheral_aresetn]

regenerate_bd_layout
save_bd_design

# get_nets -hier -filter { NAME =~ "vmk180_micro_blossom_i/MicroBlossom_0/inst/dual/broadcastRegInserted_valid" }

# do not use more jobs because they cause segmentation fault
# https://www.reddit.com/r/FPGA/comments/1846nds/i_am_suffering_from_segfault_with_vivado_on_arm/
# https://support.xilinx.com/s/question/0D54U00008NnuzVSAR/vivado-20222-fails-with-segmentation-fault-during-synthesis?language=en_US

# run synthesis, implementation and write bitstream
set_property strategy Flow_AlternateRoutability [get_runs synth_1]
launch_runs synth_1 -jobs 3
wait_on_run synth_1

set_property strategy Congestion_SpreadLogic_high [get_runs impl_1]
set_property STEPS.PHYS_OPT_DESIGN.ARGS.DIRECTIVE AggressiveFanoutOpt [get_runs impl_1]
launch_runs impl_1 -jobs 3
wait_on_run impl_1

launch_runs impl_1 -to_step write_device_image -jobs 3
wait_on_run impl_1

# export hardware XSA file
write_hw_platform -fixed -include_bit -force -file ./${name}.xsa
validate_hw_platform ./${name}.xsa

# extract the hardware specification, useful for observing what's inside
# open_hw_platform ./${name}.xsa
