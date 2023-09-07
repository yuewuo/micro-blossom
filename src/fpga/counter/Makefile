all: clean counter_test

# Generate C++ in executable form
VERILATOR_FLAGS += -cc --exe
# Generate makefile dependencies (not shown as complicates the Makefile)
#VERILATOR_FLAGS += -MMD
# Optimize
VERILATOR_FLAGS += -x-assign fast
# Warn abount lint issues; may not want this on less solid designs
VERILATOR_FLAGS += -Wall
# Make waveforms
VERILATOR_FLAGS += --trace
# Check SystemVerilog assertions
VERILATOR_FLAGS += --assert
# Generate coverage analysis
VERILATOR_FLAGS += --coverage
# Run Verilator in debug mode
# VERILATOR_FLAGS += --debug
# Add this trace to get a backtrace in gdb
#VERILATOR_FLAGS += --gdbbt

clean:
	rm -rf obj_dir
	rm -rf logs

counter_test:
	verilator $(VERILATOR_FLAGS) -f input.vc counter.sv counter_test.cpp
	$(MAKE) -j -C obj_dir -f Vcounter.mk
	@rm -rf logs
	@mkdir -p logs
	obj_dir/Vcounter +trace
	# @rm -rf logs/annotated
	# verilator_coverage --annotate logs/annotated logs/coverage.dat
