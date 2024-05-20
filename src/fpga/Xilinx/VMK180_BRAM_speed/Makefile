NAME = vmk180_bram

VIVADO_DIR = $(NAME)_vivado
XSA_FILE = $(NAME).xsa
VITIS_DIR = $(NAME)_vitis

CLOCK_FREQUENCY ?= 200

all: vitis

$(XSA_FILE):
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

R5_ELF = ./$(VITIS_DIR)/benchmark_r5/build/benchmark_r5.elf
A72_ELF = ./$(VITIS_DIR)/benchmark_a72/build/benchmark_a72.elf

$(R5_ELF):
	$(MAKE) vitis
$(A72_ELF):
	$(MAKE) vitis

run_r5: $(R5_ELF)
	xsdb ./run_xsdb.tcl run_r5
run_r5_q: $(R5_ELF)
	xsdb ./run_xsdb.tcl run_r5 quick

run_a72: $(A72_ELF)
	xsdb ./run_xsdb.tcl run_a72
run_a72_q: $(A72_ELF)
	xsdb ./run_xsdb.tcl run_a72 quick

clean:
	@read -p "Building the project from scratch takes a long time. Do you want to delete it? [y/N]: " answer; \
	if [ "$$answer" = "y" ]; then \
		echo "cleaning..."; \
		rm -rf $(NAME)_vitis; \
		rm -rf $(NAME)_vivado; \
		rm -f *.xsa; \
	else \
		echo "Do nothing"; \
	fi
