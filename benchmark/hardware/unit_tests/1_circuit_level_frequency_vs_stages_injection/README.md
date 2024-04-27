```sh
python3 prepare.py
```

It seems like the highest frequency is achieved by injecting 2 registers

```sh
d=3, inj=0 wns: 0.081ns, potential new frequency is 100.81661457808245MHz
d=3, inj=1 wns: 0.687ns, potential new frequency is 107.37678513905293MHz
d=3, inj=2 wns: 3.25ns, potential new frequency is 148.14814814814815MHz
d=3, inj=3 wns: 1.991ns, potential new frequency is 124.85953302534651MHz
d=5, inj=0 wns: 0.009ns, potential new frequency is 95.08129450680332MHz
d=5, inj=1 wns: 0.417ns, potential new frequency is 98.91866282792837MHz
d=5, inj=2 wns: 1.466ns, potential new frequency is 110.37142890337272MHz
d=5, inj=3 wns: 0.706ns, potential new frequency is 101.82971927154234MHz
d=7, inj=0 wns: 0.025ns, potential new frequency is 76.14467488227632MHz
d=7, inj=1 wns: 0.132ns, potential new frequency is 76.77015822733667MHz
d=7, inj=2 wns: 1.328ns, potential new frequency is 84.53160592255126MHz
d=7, inj=3 wns: 0.343ns, potential new frequency is 78.03419540583943MHz
d=9, inj=0 wns: 0.02ns, potential new frequency is 66.08723515039853MHz
d=9, inj=1 wns: 0.174ns, potential new frequency is 66.76674934953MHz
d=9, inj=2 wns: 0.803ns, potential new frequency is 69.69362261114549MHz
d=9, inj=3 wns: 0.634ns, potential new frequency is 68.88231143989078MHz
```

It seems like injecting 2 stages will always benefit the largest, but as the code distance increases, the benefit vanishes.
For example, d=3 we saw 46% improvement in clock frequency from inj=0 to inj=2.
When d=5, we only saw 15% improvement; d=7: 10%, d=9: 5%.
This is possibly because of the latency introduced in broadcasting and convergecasting?
However, I suppose the `retiming` option could move the registers around to achieve a better clock frequency.
This is confusing... Did I enable the retiming option?

The functionality of all build instances are confirmed
