create_project EdgeIsTightTester ./EdgeIsTightTester -part xc7z010clg400-1 -force
add_files ./EdgeIsTightTester.v
set_property top EdgeIsTightTester [current_fileset]

synth_design -rtl -rtl_skip_mlo -name rtl_1 -mode out_of_context

set_property DONT_TOUCH true [get_nets io_leftGrown[*]]
set_property DONT_TOUCH true [get_nets io_rightGrown[*]]
set_property DONT_TOUCH true [get_nets io_weight[*]]
set_property DONT_TOUCH true [get_nets io_isTight]

create_clock -name virt_clk -period 10000 -waveform {0 5000}
set_input_delay 0 -clock virt_clk [all_inputs]
set_output_delay 0 -clock virt_clk [all_outputs]

launch_runs synth_1 -jobs 8
wait_on_run synth_1

launch_runs impl_1 -jobs 8
wait_on_run impl_1

open_run impl_1

report_timing -from [all_inputs] -to [all_outputs] -nworst 10 -file ./EdgeIsTightTester/timing.txt
