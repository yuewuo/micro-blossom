NAME = code_capacity_d_5

MODULE_NAME = MicroBlossom
VIVADO_DIR = $(NAME)_vivado
VERILOG_DIR = $(NAME)_verilog
XSA_FILE = $(NAME).xsa
VITIS_DIR = $(NAME)_vitis

DUAL_CONFIG_FILEPATH ?=
CLOCK_FREQUENCY ?= 10.0

all: vitis

$(VERILOG_DIR)/$(MODULE_NAME).v:
	@if ! [ -f "./create_verilog.py" ]; then \
		echo "don't know how to create the verilog"; \
		exit 1; \
	fi
	@if [ "$(DUAL_CONFIG_FILEPATH)" = "" ]; then \
		echo "DUAL_CONFIG_FILEPATH is not set, required for building vivado project"; \
		exit 1; \
	fi
	@if ! [ -f "$(DUAL_CONFIG_FILEPATH)" ]; then \
		echo "$(DUAL_CONFIG_FILEPATH) does not exist"; \
		exit 1; \
	fi
	rm -rf $(VERILOG_DIR)
	python3 ./create_verilog.py "$(DUAL_CONFIG_FILEPATH)"

$(XSA_FILE): $(VERILOG_DIR)/$(MODULE_NAME).v
	@echo generating hardware platform description \($(XSA_FILE)\) using Vivado

	@if [ -d "$(VIVADO_DIR)" ]; then \
		read -p "$(VIVADO_DIR) exists. Do you want to delete it? [y/N]: " answer; \
		if [ "$$answer" = "y" ]; then \
			echo "Deleting $(VIVADO_DIR)..."; \
			rm -rf $(VIVADO_DIR); \
		else \
			echo "Aborting. You can manually run commands in ./create_vivado.tcl to generate the XSA file"; \
			exit 1; \
		fi \
	fi
	vivado -mode batch -s create_vivado.tcl -tclargs $(CLOCK_FREQUENCY)

vitis: $(XSA_FILE)
	@echo creating vitis workspace at $(VITIS_DIR)
	vitis -s create_vitis.py

A72_ELF = ./$(VITIS_DIR)/benchmark_a72/build/benchmark_a72.elf

$(A72_ELF):
	$(MAKE) vitis

run_a72: $(A72_ELF)
	xsdb ./run_xsdb.tcl run_a72
run_a72_q: $(A72_ELF)
	xsdb ./run_xsdb.tcl run_a72 quick

clean:
	@read -p "Building the project from scratch takes a long time. Do you want to delete it? [y/N]: " answer; \
	if [ "$$answer" = "y" ]; then \
		echo "cleaning..."; \
	else \
		echo "Do nothing"; \
		exit 0; \
	fi
	@if [ -f "./create_verilog.py" ]; then \
		rm -rf $(NAME)_verilog; \
	else \
		echo "not deleting verilog folder because there is no create_verilog.py file"; \
	fi
	rm -rf $(NAME)_vitis
	rm -rf $(NAME)_vivado
	rm -f *.xsa
