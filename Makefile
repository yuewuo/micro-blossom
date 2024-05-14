all: sim_test behavior_test fpga_test

fpga_test:
	sbt test

behavior_test:
	python3 benchmark/behavior/tests/run.py

sim_test:
	cd src/cpu/blossom && make
