

ROOT_DIR := $(shell dirname $(realpath $(firstword $(MAKEFILE_LIST))))
GIT_DIR := $(ROOT_DIR)/..


all:
	@echo "use the following supported targets":
	@echo "    partial-clean: revert the folder back to its initial state of artifact evaluation"
	@echo "    complete-clean: remove all the generated files including the Vivado project and trace files"
	@echo "        a complete clean will require the user to install Vivado Design Suite and a VMK180"
	@echo "        evaluation board to rerun everything from scratch"

partial-clean: clean-example-graphs clean-resource-estimate clean-rust-build clean-embedded-build
	@echo ""
	@echo "Clean the build files that are only created during the artifact run"
	@echo "This command will not clean the Vivado projects and trace files shipped with the artifact"
	@echo "    try complete-clean.sh for cleaning everything"
	@echo ""


clean-rust-build:
	rm -rf $(GIT_DIR)/src/cpu/blossom/target

clean-embedded-build:
	rm -rf $(GIT_DIR)/src/cpu/embedded/target
	rm -f $(GIT_DIR)/src/cpu/embedded/embedded.defects

clean-simulation:
	rm -rf $(GIT_DIR)/simWorkspace

clean-resource-estimate:
	rm -f $(GIT_DIR)/benchmark/hardware/resource_estimate/circuit_level/tmp/*

clean-example-graphs:
	rm -f $(GIT_DIR)/resources/graphs/*.json

clean-sbt:
	rm -rf $(GIT_DIR)/project/project
	rm -rf $(GIT_DIR)/project/target
	rm -rf $(GIT_DIR)/target
	rm -rf $(GIT_DIR)/tmp
