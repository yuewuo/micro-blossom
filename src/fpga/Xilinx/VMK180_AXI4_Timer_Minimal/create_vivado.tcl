set name vmk180_axi4_timer_minimal
set ip_name Axi4Timer

create_project ${name} ./${name}_vivado -part xcvm1802-vsva2197-2MP-e-S
set_property board_part xilinx.com:vmk180:part0:3.2 [current_project]
create_bd_design "${name}" -mode batch

instantiate_example_design -template xilinx.com:design:Versal_APU_RPU_perf:1.0 -design ${name} -options { Preset.VALUE LPDDR4 }



# create IP
exec rm -rf ${name}_verilog
exec python3 ./create_verilog.py
ipx::infer_core -vendor user.org -library user -taxonomy /UserIP ./${name}_verilog
ipx::unload_core ${name}_verilog/component.xml
set_property ip_repo_paths ${name}_verilog [current_project]
update_ip_catalog -rebuild -scan_changes


# configure CIPS
# expose 200MHz clock from PS
# expose AXI_FPD
# expose a PL reset
startgroup
set_property -dict [list \
  CONFIG.PS_PMC_CONFIG { \
    PS_USE_PMCPL_CLK0 {1} \
    PMC_CRP_PL0_REF_CTRL_FREQMHZ {200} \
    PS_USE_M_AXI_FPD {1} \
    PS_M_AXI_FPD_DATA_WIDTH {64} \
    PS_NUM_FABRIC_RESETS {1} \
  } \
] [get_bd_cells versal_cips_0]
endgroup

# create and connect my AXI4 IP via SmartConnect
create_bd_cell -type ip -vlnv user.org:user:${ip_name}:1.0 ${ip_name}_0
create_bd_cell -type ip -vlnv xilinx.com:ip:smartconnect:1.0 smartconnect_0
set_property CONFIG.NUM_SI {1} [get_bd_cells smartconnect_0]
connect_bd_intf_net [get_bd_intf_pins versal_cips_0/M_AXI_FPD] [get_bd_intf_pins smartconnect_0/S00_AXI]
connect_bd_intf_net [get_bd_intf_pins smartconnect_0/M00_AXI] [get_bd_intf_pins ${ip_name}_0/s0]
connect_bd_net [get_bd_pins ${ip_name}_0/clk] [get_bd_pins versal_cips_0/pl0_ref_clk]
connect_bd_net [get_bd_pins smartconnect_0/aclk] [get_bd_pins versal_cips_0/pl0_ref_clk]
connect_bd_net [get_bd_pins versal_cips_0/pl0_ref_clk] [get_bd_pins versal_cips_0/m_axi_fpd_aclk]

# create reset system
create_bd_cell -type ip -vlnv xilinx.com:ip:proc_sys_reset:5.0 proc_sys_reset_0
connect_bd_net [get_bd_pins proc_sys_reset_0/peripheral_reset] [get_bd_pins ${ip_name}_0/reset]
connect_bd_net [get_bd_pins proc_sys_reset_0/peripheral_aresetn] [get_bd_pins smartconnect_0/aresetn]
connect_bd_net [get_bd_pins proc_sys_reset_0/ext_reset_in] [get_bd_pins versal_cips_0/pl0_resetn]
connect_bd_net [get_bd_pins versal_cips_0/pl0_ref_clk] [get_bd_pins proc_sys_reset_0/slowest_sync_clk]

# assign address
assign_bd_address -target_address_space /versal_cips_0/M_AXI_FPD [get_bd_addr_segs ${ip_name}_0/s0/reg0] -force
set_property range 4K [get_bd_addr_segs versal_cips_0/M_AXI_FPD/SEG_${ip_name}_0_reg0]
set_property offset 0xA4000000 [get_bd_addr_segs versal_cips_0/M_AXI_FPD/SEG_${ip_name}_0_reg0]

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
