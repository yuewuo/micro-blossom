# Hardware Tests

The same as benchmark/behavior/tests, but on actual hardware.

```py
- [x] {}
- [x] {'graph': '/home/wuyue/GitHub/micro-blossom/resources/graphs/example_code_capacity_rotated_d3.json'}
- [x] {'graph': '/home/wuyue/GitHub/micro-blossom/resources/graphs/example_code_capacity_planar_d3.json'}
- [x] {'graph': '/home/wuyue/GitHub/micro-blossom/resources/graphs/example_phenomenological_rotated_d3.json'}
- [x] {'graph': '/home/wuyue/GitHub/micro-blossom/resources/graphs/example_circuit_level_d3.json'}
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
- [ ] {'clock_divide_by': 4, 'context_depth': 4}
- [x] {'clock_divide_by': 3, 'broadcast_delay': 2, 'convergecast_delay': 1}
- [x] {'clock_divide_by': 3, 'broadcast_delay': 1, 'convergecast_delay': 2}
- [x] {'clock_divide_by': 3, 'broadcast_delay': 2, 'convergecast_delay': 2}
- [x] {'clock_divide_by': 4, 'broadcast_delay': 2, 'convergecast_delay': 1}
- [x] {'clock_divide_by': 4, 'broadcast_delay': 1, 'convergecast_delay': 2}
- [x] {'clock_divide_by': 4, 'broadcast_delay': 2, 'convergecast_delay': 2}
- [ ] {'support_offloading': True}
- [ ] {'support_offloading': True, 'clock_divide_by': 3}
- [ ] {'support_offloading': True, 'clock_divide_by': 4}
- [ ] {'support_offloading': True, 'broadcast_delay': 2, 'clock_divide_by': 3}
- [ ] {'support_offloading': True, 'broadcast_delay': 2, 'clock_divide_by': 4}
```
