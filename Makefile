all: fpga_test behavior_test

fpga_test:
	sbt test

behavior_test:
	python3 benchmark/behavior/tests/run.py
