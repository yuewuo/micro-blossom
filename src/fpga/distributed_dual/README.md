# Distributed Dual Module

The interface of the distributed dual module is simple.
For the type definition, please see `../interface/interface.h` and `../interface/interface.sv`

- input:
    - `Grow(length)`
    - `SetSpeed(node, speed)`
    - `SetParent(node, blossom)`
- output:
    - `Conflict{ node_1, touch_1, node_2, touch_2 }`
    - `TouchingVirtual{ node, touch, vertex }`
    - `BlossomNeedExpand{ node }`
    - `NonZeroGrow{ length }`
