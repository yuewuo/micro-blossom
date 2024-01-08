NAME = vmk180_bram

VIVADO_DIR = $(NAME)_vivado
XSA_FILE = $(NAME).xsa
VITIS_DIR = $(NAME)_vitis

all: vitis

$(XSA_FILE):
	@echo generating hardware platform description \($(XSA_FILE)\) using Vivado

	@if [ -d "$(VIVADO_DIR)" ]; then \
		read -p "$(VIVADO_DIR) exists. Do you want to delete it? [y/N]: " answer; \
		if [ "$$answer" != "y" ]; then \
			echo "Aborting. You can manually run commands in ./create_vivado.tcl to generate the XSA file"; \
			exit 1; \
		else \
			echo "Deleting $(VIVADO_DIR)..."; \
			rm -rf $(VIVADO_DIR); \
		fi \
	fi
	vivado -mode batch -s create_vivado.tcl

vitis: $(XSA_FILE)
	@echo creating vitis workspace at $(VITIS_DIR)
	vitis -s create_vitis.py

clean:
	rm -f *.xsa
