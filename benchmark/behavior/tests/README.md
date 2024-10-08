# Behavior Correctness Tests

Since the model is highly configurable, we would like to make sure that the correctness is guaranteed in every configuration.
There are two classes of parameters: 1. explicit to the paper where this configuration will actually be evaluated and compared, e.g. enabling primal/dual offloading or not, pipeline latency (the summation of broadcast, convergecast and stage injections); 2. implicit parameters or used to construct those explicit parameters, such as the individual broadcast/convergecast latency, the number and position of the stage injection, etc.
For correctness tests, we would like to cover all these parameter spaces.
In order to explore the space, we will choose a defect setting, and each test will modify the configuration by a little.
We use 5 different code to test it, so that different graph structures can be explored for each configuration: repetition code $d=3$, rotated code capacity $d=3$, standard code capacity $d=3$, phenomenological $d=3$, circuit-level $d=3$ (we only use $d=3$ because it's relatively small and thus reduces the overall simulation time).
Each of them will also 
In testing it, we would prefer starting from the repetition code and run through every configuration, so that bugs can be found early.
Also, since the test usually runs for a fairly long time, we would hope the script will go to another test even if failures occur, so that more information could be obtained during the night.

The default configuration is:
- code type: repetition code $d=3$
- support offloading: false
- broadcast delay: 1
- convergecast delay: 1
- context depth: 1
- conflict channels: 1
- inject registers: none
- clock divided by: 1
- bus interface: AxiLite4

The variating configurations are:
- 4x code type: rotated code capacity $d=3$, standard code capacity $d=3$, phenomenological $d=3$, circuit-level $d=3$
- 2x broadcast delay: 2, 3
- 2x convergecast delay: 2, 3
- 5x context depth: 2, 4, 8, 16, 32
- 12x inject registers: 1 (every stage, there are 10 stages in total), 2 (default), 3 (default)
- 3x clock divided by: 2, 3, 4
- 5x (clock divided by, inject registers): (2, 1), (2, 2), (2, 3), (3, 1), (3, 2)
- 4x (clock divided by, context depth): (2, 2), (2, 4), (3, 2), (3, 4)
- 6x (clock divided by, broadcast delay, convergecast delay): (2, 2, 1), (2, 1, 2), (2, 2, 2), (3, 2, 1), (3, 1, 2), (3, 2, 2)
- 2x bus interfaces: Axi4, AxiLite4Bus32
- 4x (bus interfaces, clock divided by): (Axi4, 2), (Axi4, 3), (AxiLite4Bus32, 2), (AxiLite4Bus32, 3)
- 1x support offloading: true
- 2x (support offloading, clock divided by): (true, 2), (true, 3)
- 2x (support offloading, broadcast delay, clock divided by): (true, 2, 2),  (true, 2, 3)

There are 54 different configurations in total.
Since we only run $d=3$ codes, the verilator compilation should finish within 1 minute.
Thus, the test should run within one hour.

## Results 2024.4.28

This is a first try of the test script: I haven't developed the proper test yet, so I just use `test_micro_blossom` to roughly get the idea of which one might fail.
I'll develop more sophisticated and generic tests later on.

- [x] {}
- [x] {'graph': '/Users/wuyue/Documents/GitHub/micro-blossom/resources/graphs/example_code_capacity_rotated_d3.json'}
- [x] {'graph': '/Users/wuyue/Documents/GitHub/micro-blossom/resources/graphs/example_code_capacity_planar_d3.json'}
- [x] {'graph': '/Users/wuyue/Documents/GitHub/micro-blossom/resources/graphs/example_phenomenological_rotated_d3.json'}
- [x] {'graph': '/Users/wuyue/Documents/GitHub/micro-blossom/resources/graphs/example_circuit_level_d3.json'}
- [x] {'broadcast_delay': 2}
- [x] {'broadcast_delay': 3}
- [x] {'convergecast_delay': 2}
- [x] {'convergecast_delay': 3}
- [x] {'clock_divide_by': 3}
- [x] {'clock_divide_by': 4}
- [x] {'clock_divide_by': 5}
- [x] {'context_depth': 2}
- [x] {'context_depth': 4}
- [x] {'context_depth': 8}
- [x] {'context_depth': 16}
- [x] {'context_depth': 32}
- [x] {'inject_registers': 'offload'}
- [x] {'inject_registers': 'offload2'}
- [x] {'inject_registers': 'offload3'}
- [x] {'inject_registers': 'offload4'}
- [x] {'inject_registers': 'execute'}
- [x] {'inject_registers': 'execute2'}
- [x] {'inject_registers': 'execute3'}
- [x] {'inject_registers': 'update'}
- [x] {'inject_registers': 'update2'}
- [x] {'inject_registers': 'update3'}
- [x] {'inject_registers': 'offload4,update3'}
- [x] {'inject_registers': 'offload3,execute2,update'}
- [x] {'clock_divide_by': 3, 'inject_registers': 'execute2'}
- [x] {'clock_divide_by': 3, 'inject_registers': 'offload4,update3'}
- [x] {'clock_divide_by': 3, 'inject_registers': 'offload3,execute2,update'}
- [x] {'clock_divide_by': 4, 'inject_registers': 'execute2'}
- [x] {'clock_divide_by': 4, 'inject_registers': 'offload4,update3'}
- [x] {'clock_divide_by': 3, 'context_depth': 2}
- [x] {'clock_divide_by': 3, 'context_depth': 4}
- [x] {'clock_divide_by': 4, 'context_depth': 2}
- [x] {'clock_divide_by': 4, 'context_depth': 4}
- [x] {'clock_divide_by': 3, 'broadcast_delay': 2, 'convergecast_delay': 1}
- [x] {'clock_divide_by': 3, 'broadcast_delay': 1, 'convergecast_delay': 2}
- [x] {'clock_divide_by': 3, 'broadcast_delay': 2, 'convergecast_delay': 2}
- [x] {'clock_divide_by': 4, 'broadcast_delay': 2, 'convergecast_delay': 1}
- [x] {'clock_divide_by': 4, 'broadcast_delay': 1, 'convergecast_delay': 2}
- [x] {'clock_divide_by': 4, 'broadcast_delay': 2, 'convergecast_delay': 2}
- [x] {'bus_type': 'Axi4'}
- [x] {'use_32_bus': True}
- [x] {'bus_type': 'Axi4', 'clock_divide_by': 2}
- [x] {'bus_type': 'Axi4', 'clock_divide_by': 3}
- [x] {'use_32_bus': True, 'clock_divide_by': 2}
- [x] {'use_32_bus': True, 'clock_divide_by': 3}
- [x] {'support_offloading': True}
- [x] {'support_offloading': True, 'clock_divide_by': 3}
- [x] {'support_offloading': True, 'clock_divide_by': 4}
- [x] {'support_offloading': True, 'broadcast_delay': 2, 'clock_divide_by': 3}
- [x] {'support_offloading': True, 'broadcast_delay': 2, 'clock_divide_by': 4}

This test takes less than 8min on a Ubuntu machine, fast enough for feedback-based debugging!
The same tests should be run later on real hardware, which will take about 15 min * 54 = 14 hours.
A new test needs to be developed to 1. take into consideration of different graph structures and weights and 2. take into consideration of offloading, especially testing the efficiency of offloading (every single edge defect should be offloaded).
