set name vmk180_micro_blossom

puts "use this script only if you are sure the design is complete"

open_project ./${name}_vivado/${name}.xpr

reset_run synth_1
reset_run impl_1

# run synthesis, implementation and write bitstream
set_property strategy Flow_AlternateRoutability [get_runs synth_1]
launch_runs synth_1 -jobs 4
wait_on_run synth_1

set_property strategy Congestion_SpreadLogic_high [get_runs impl_1]
set_property STEPS.PHYS_OPT_DESIGN.ARGS.DIRECTIVE AggressiveFanoutOpt [get_runs impl_1]
launch_runs impl_1 -jobs 4
wait_on_run impl_1

launch_runs impl_1 -to_step write_device_image -jobs 4
wait_on_run impl_1

# export hardware XSA file
write_hw_platform -fixed -include_bit -force -file ./${name}.xsa
validate_hw_platform ./${name}.xsa
