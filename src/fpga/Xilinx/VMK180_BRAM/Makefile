NAME = vmk180_bram

VIVADO_DIR = $(NAME)_vivado
XSA_FILE = $(NAME).xsa
VITIS_DIR = $(NAME)_vitis

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
	vivado -mode batch -s create_vivado.tcl

vitis: $(XSA_FILE)
	@echo creating vitis workspace at $(VITIS_DIR)
	vitis -s create_vitis.py

run_r5:
	xsdb ./run_xsdb.tcl run_r5
run_r5_quick:
	xsdb ./run_xsdb.tcl run_r5 quick

run_a72:
	xsdb ./run_xsdb.tcl run_a72
run_a72_quick:
	xsdb ./run_xsdb.tcl run_a72 quick

clean:
	@read -p "Building the project from scratch takes a long time. Do you want to delete it? [y/N]: " answer; \
	if [ "$$answer" = "y" ]; then \
		echo "cleaning..."; \
		rm -rf vmk180_bram_vitis; \
		rm -rf vmk180_bram_vivado; \
		rm -f *.xsa; \
	else \
		echo "Do nothing"; \
	fi
