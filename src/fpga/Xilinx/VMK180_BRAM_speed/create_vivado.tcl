set name vmk180_bram

if { $argc != 1 } {
    puts "Usage: <clock frequency in MHz>"
    puts "Please try again."
    exit 1
} else {
    scan [lindex $argv 0] %f clock_frequency
}

create_project ${name} ./${name}_vivado -part xcvm1802-vsva2197-2MP-e-S
set_property board_part xilinx.com:vmk180:part0:3.2 [current_project]
create_bd_design "${name}" -mode batch

instantiate_example_design -template xilinx.com:design:Versal_APU_RPU_perf:1.0 -design ${name} -options { Preset.VALUE LPDDR4 }

update_compile_order -fileset sources_1

# configure CIPS
# expose {clock_frequency}MHz clock from PS
# expose AXI_FPD and AXI_LPD
# expose a PL reset
startgroup
set_property -dict [list \
  CONFIG.PS_PMC_CONFIG { \
    PS_USE_PMCPL_CLK0 {1} \
    PS_USE_M_AXI_FPD {1} \
    PS_USE_M_AXI_LPD {1} \
    PS_M_AXI_FPD_DATA_WIDTH {64} \
    PS_M_AXI_LPD_DATA_WIDTH {64} \
    PS_NUM_FABRIC_RESETS {1} \
  } \
] [get_bd_cells versal_cips_0]
endgroup
set_property -dict [list CONFIG.PS_PMC_CONFIG "PMC_CRP_PL0_REF_CTRL_FREQMHZ $clock_frequency"] [get_bd_cells versal_cips_0]

# add a BRAM with two ports
create_bd_cell -type ip -vlnv xilinx.com:ip:emb_mem_gen:1.0 emb_mem_gen_0
set_property CONFIG.MEMORY_TYPE {True_Dual_Port_RAM} [get_bd_cells emb_mem_gen_0]
# add two BRAM controllers
create_bd_cell -type ip -vlnv xilinx.com:ip:axi_bram_ctrl:4.1 axi_bram_ctrl_0
create_bd_cell -type ip -vlnv xilinx.com:ip:axi_bram_ctrl:4.1 axi_bram_ctrl_1
set_property -dict [list \
  CONFIG.DATA_WIDTH {64} \
  CONFIG.SINGLE_PORT_BRAM {1} \
] [get_bd_cells axi_bram_ctrl_0]
set_property -dict [list \
  CONFIG.DATA_WIDTH {64} \
  CONFIG.SINGLE_PORT_BRAM {1} \
] [get_bd_cells axi_bram_ctrl_1]
# connect them
connect_bd_intf_net [get_bd_intf_pins emb_mem_gen_0/BRAM_PORTA] [get_bd_intf_pins axi_bram_ctrl_0/BRAM_PORTA]
connect_bd_intf_net [get_bd_intf_pins emb_mem_gen_0/BRAM_PORTB] [get_bd_intf_pins axi_bram_ctrl_1/BRAM_PORTA]

# create reset system
create_bd_cell -type ip -vlnv xilinx.com:ip:proc_sys_reset:5.0 proc_sys_reset_0
connect_bd_net [get_bd_pins proc_sys_reset_0/peripheral_aresetn] [get_bd_pins axi_bram_ctrl_0/s_axi_aresetn]
connect_bd_net [get_bd_pins proc_sys_reset_0/peripheral_aresetn] [get_bd_pins axi_bram_ctrl_1/s_axi_aresetn]
connect_bd_net [get_bd_pins proc_sys_reset_0/ext_reset_in] [get_bd_pins versal_cips_0/pl0_resetn]

# connect clock
connect_bd_net [get_bd_pins versal_cips_0/pl0_ref_clk] [get_bd_pins proc_sys_reset_0/slowest_sync_clk]
connect_bd_net [get_bd_pins versal_cips_0/pl0_ref_clk] [get_bd_pins axi_bram_ctrl_0/s_axi_aclk]
connect_bd_net [get_bd_pins versal_cips_0/pl0_ref_clk] [get_bd_pins axi_bram_ctrl_1/s_axi_aclk]
connect_bd_net [get_bd_pins versal_cips_0/pl0_ref_clk] [get_bd_pins versal_cips_0/m_axi_fpd_aclk]
connect_bd_net [get_bd_pins versal_cips_0/pl0_ref_clk] [get_bd_pins versal_cips_0/m_axi_lpd_aclk]

# connect the FPD and LPD AXIs to the BRAM AXIs
connect_bd_intf_net [get_bd_intf_pins axi_bram_ctrl_0/S_AXI] [get_bd_intf_pins versal_cips_0/M_AXI_FPD]
connect_bd_intf_net [get_bd_intf_pins axi_bram_ctrl_1/S_AXI] [get_bd_intf_pins versal_cips_0/M_AXI_LPD]

# assign address: A72 uses 0xA4000000 and R5F uses 0x80000000
assign_bd_address -target_address_space /versal_cips_0/M_AXI_FPD [get_bd_addr_segs axi_bram_ctrl_0/S_AXI/Mem0] -force
set_property offset 0xA4000000 [get_bd_addr_segs {versal_cips_0/M_AXI_FPD/SEG_axi_bram_ctrl_0_Mem0}]
set_property range 4K [get_bd_addr_segs {versal_cips_0/M_AXI_LPD/SEG_axi_bram_ctrl_0_Mem0}]
assign_bd_address -target_address_space /versal_cips_0/M_AXI_LPD [get_bd_addr_segs axi_bram_ctrl_1/S_AXI/Mem0] -force
set_property offset 0x80000000 [get_bd_addr_segs {versal_cips_0/M_AXI_LPD/SEG_axi_bram_ctrl_1_Mem0}]
set_property range 4K [get_bd_addr_segs {versal_cips_0/M_AXI_LPD/SEG_axi_bram_ctrl_1_Mem0}]

# # create an ILA to monitor the transactions
# create_bd_cell -type ip -vlnv xilinx.com:ip:axis_ila:1.2 axis_ila_0
# set_property -dict [list \
#   CONFIG.C_MON_TYPE {Interface_Monitor} \
#   CONFIG.C_NUM_MONITOR_SLOTS {4} \
#   CONFIG.C_NUM_OF_PROBES {4} \
#   CONFIG.C_SLOT_1_INTF_TYPE {xilinx.com:interface:bram_rtl:1.0} \
#   CONFIG.C_SLOT_3_INTF_TYPE {xilinx.com:interface:bram_rtl:1.0} \
# ] [get_bd_cells axis_ila_0]
# connect_bd_intf_net [get_bd_intf_pins axis_ila_0/SLOT_0_AXI] [get_bd_intf_pins versal_cips_0/M_AXI_FPD]
# connect_bd_intf_net [get_bd_intf_pins axis_ila_0/SLOT_1_BRAM] [get_bd_intf_pins axi_bram_ctrl_0/BRAM_PORTA]
# connect_bd_intf_net [get_bd_intf_pins axis_ila_0/SLOT_2_AXI] [get_bd_intf_pins versal_cips_0/M_AXI_LPD]
# connect_bd_intf_net [get_bd_intf_pins axis_ila_0/SLOT_3_BRAM] [get_bd_intf_pins axi_bram_ctrl_1/BRAM_PORTA]
# connect_bd_net [get_bd_pins axis_ila_0/clk] [get_bd_pins versal_cips_0/pl0_ref_clk]
# connect_bd_net [get_bd_pins axis_ila_0/resetn] [get_bd_pins proc_sys_reset_0/peripheral_aresetn]

regenerate_bd_layout
save_bd_design

# run synthesis, implementation and write bitstream
launch_runs synth_1 -jobs 10
wait_on_run synth_1

launch_runs impl_1 -jobs 10
wait_on_run impl_1

launch_runs impl_1 -to_step write_device_image -jobs 10
wait_on_run impl_1

# export hardware XSA file
write_hw_platform -fixed -include_bit -force -file ./${name}.xsa
validate_hw_platform ./${name}.xsa

# extract the hardware specification, useful for observing what's inside
# open_hw_platform ./${name}.xsa
