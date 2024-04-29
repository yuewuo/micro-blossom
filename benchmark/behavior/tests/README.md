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

- [x] 0. {}
- [ ] 1. {'graph': '/Users/wuyue/Documents/GitHub/micro-blossom/resources/graphs/example_code_capacity_rotated_d3.json'}
- [x] 2. {'graph': '/Users/wuyue/Documents/GitHub/micro-blossom/resources/graphs/example_code_capacity_planar_d3.json'}
- [ ] 3. {'graph': '/Users/wuyue/Documents/GitHub/micro-blossom/resources/graphs/example_phenomenological_d3.json'}
- [ ] 4. {'graph': '/Users/wuyue/Documents/GitHub/micro-blossom/resources/graphs/example_circuit_level_d3.json'}
- [x] 5. {'broadcast_delay': 2}
- [x] 6. {'broadcast_delay': 3}
- [x] 7. {'convergecast_delay': 2}
- [x] 8. {'convergecast_delay': 3}
- [x] 9. {'clock_divide_by': 2}
- [x] 10. {'clock_divide_by': 3}
- [x] 11. {'clock_divide_by': 4}
- [x] 12. {'context_depth': 2}
- [x] 13. {'context_depth': 4}
- [x] 14. {'context_depth': 8}
- [x] 15. {'context_depth': 16}

