import matplotlib.pyplot as plt
import numpy as np


d_vec = [3, 5, 7, 9, 11, 13]
f_vec = [200, 141, 105, 92, 78, 61]
cycle_vec = [1000 / f for f in f_vec]

# plot frequency
plt.cla()
plt.plot(d_vec, f_vec, "o-")
plt.xlabel("code distance")
plt.ylabel("maximum frequency (Mhz)")
plt.title("Explore maximum frequency of circuit-level noise")
plt.savefig("maximum_frequency.pdf")

# plot cycle
plt.cla()
plt.plot(d_vec, cycle_vec, "o-")
plt.xlabel("code distance")
plt.ylabel("minimum clock cycle (ns)")
plt.savefig("minimum_clock_cycle.pdf")


# plot cycle
n_vec = [d**3 for d in d_vec]
plt.cla()
plt.plot(n_vec, cycle_vec, "o-")
# this looks the best for fitting
fit_n_vec = np.array(n_vec[2:])
fit_cycle_vec = cycle_vec[2:]
m, b = np.polyfit(fit_n_vec, fit_cycle_vec, 1)
print(f"heuristic minimum clock cycle fitting: {m:.3e} * d^3 + {b:.3f}")
plt.plot(fit_n_vec, m * fit_n_vec + b, "--k")
plt.xlabel("number of vertices $|V|$")
plt.ylabel("minimum clock cycle (ns)")
plt.savefig("minimum_clock_cycle_vertices.pdf")


# test heuristic curve
import git, sys, os

git_root_dir = git.Repo(".", search_parent_directories=True).working_tree_dir
sys.path.insert(0, os.path.join(git_root_dir, "benchmark"))
sys.path.insert(0, os.path.join(git_root_dir, "src", "fpga", "utils"))
from vivado_builder import *

for d, f in zip(d_vec, f_vec):
    heuristic_f = HeuristicFrequencyCircuitLevel.of(d)
    print(f"d = {d}, real frequency = {f}MHz, heuristic frequency = {heuristic_f}MHz")
