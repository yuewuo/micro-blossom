```sh
python3 prepare.py  # create hardware folder and build the hardware projects
python3 run.py
```

# d=3

# d=5

# d=9

2024.5.18: failed due to routing congestion level 6
try different strategy

default:

```sh
13399 registers (matches with expectation)
145729 LUT as logic (/899840 = 16%)
```

I try to use the `Flow_Alternate Routability` Preconfigured Strategies for synthesis and see what happens.
No changes...

The congestion is the problem of placement? And also high fanout?
When I (accidentally) set the attribute of `broadcastRegInserted.addAttribute("mark_debug = \"true\"")`,
d=9 implementation succeeds without a problem.
When it's not, the binary replication tree of the broadcast signal causes congestion.
So I should set some options in the implementation to let it automatically replicate the signal using LUT to reduce congestion.
And also, maybe it's because the higher density of the logic (more optimized) that causes congestion.
Is there a placement option to scatter the logic more?

Next, try changing the implementation setting of Place Design directive to `AltSpreadLogic_high`.
Still doesn't work....

Try changing the implementation strategy to `Congestion_SpreadLogic_high`.

Try changing phys_opt_design directive to `AggressiveFanoutOpt`.
Try changing the clock frequency to 25MHz.
Wait, it works????

reading https://support.xilinx.com/s/article/66314?language=en_US

Use global buffers on non-clock high-fanout nets. The opt_design command can automatically insert BUFGs on high fanout nets.
Using global clocking resources can help congestion due to high fanout nets. Consult the report_high_fanout report from the routed design to see if there are potential candidates. Also, automatic BUFG insertion by opt_design can be adjusted. See (Xilinx Answer 54177) for more information.
Specifically `set_property CLOCK_BUFFER_TYPE BUFG [get_nets <net_name>]`
Try reducing/removing LUT combining from synthesis (-no_lc). This can reduce the number of nets entering CLBs that become congested due to LUT inputs.

# d=11
