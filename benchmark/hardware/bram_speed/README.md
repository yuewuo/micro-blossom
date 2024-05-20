# BRAM Speed Test

Understand how the clock frequency of the AXI4 BRAM effect the latency

```sh
frequency=100.0 wns: 5.914ns, potential new frequency is 244.73813020068528MHz
frequency=107.0 wns: 5.167ns, potential new frequency is 239.3034703476162MHz
frequency=114.0 wns: 4.867ns, potential new frequency is 256.0865482678217MHz
frequency=123.0 wns: 4.19ns, potential new frequency is 253.80186946742884MHz
frequency=131.0 wns: 3.69ns, potential new frequency is 253.5761986798552MHz
frequency=141.0 wns: 3.521ns, potential new frequency is 280.01803236690705MHz
frequency=151.0 wns: 2.528ns, potential new frequency is 244.22907716991878MHz
frequency=162.0 wns: 2.528ns, potential new frequency is 274.36050292651214MHz
frequency=174.0 wns: 2.015ns, potential new frequency is 267.94376260798595MHz
frequency=186.0 wns: 2.015ns, potential new frequency is 297.50003998656456MHz
frequency=200.0 wns: 1.285ns, potential new frequency is 269.17900403768505MHz
frequency=214.0 wns: 1.285ns, potential new frequency is 295.1683425056206MHz
frequency=229.0 wns: 0.818ns, potential new frequency is 281.7844213821465MHz
frequency=246.0 wns: 0.721ns, potential new frequency is 299.0394270112833MHz
frequency=263.0 wns: 0.844ns, potential new frequency is 338.0341067416597MHz
frequency=282.0 wns: 0.698ns, potential new frequency is 351.11135459258634MHz
frequency=303.0 wns: 0.396ns, potential new frequency is 344.31348663427315MHz
frequency=324.0 wns: 0.396ns, potential new frequency is 371.68921275306985MHz
frequency=348.0 wns: 0.372ns, potential new frequency is 399.75004135345256MHz
```

(from the above data, we see vivado only try to optimize the clock frequency when we push it to do so)
