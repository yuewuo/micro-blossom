import os
import matplotlib.pyplot as plt

this_dir = os.path.dirname(os.path.abspath(__file__))

names = [
    "code_capacity_final",
    "phenomenological_final",
    # "circuit_level_no_offloading",
    "circuit_level_final",
]

ds = []
ys = {name: [] for name in names}

for name in names:
    with open(os.path.join(this_dir, name, "best_frequencies.txt"), "r") as f:
        vertex_num_vec = []
        d_vec = []
        f_vec = []
        y_err = []
        for line in f.readlines():
            line = line.strip("\r\n ")
            if line.startswith("#") or line == "":
                continue
            distance, frequency, estimated_frequency, vertex_num = line.split(" ")
            assert distance.startswith("d_")
            d = int(distance[2:])
            f = int(frequency)  # this is the frequency that the project is running
            max_f = float(estimated_frequency)
            vertex_num = int(vertex_num)
            assert max_f >= f, "the timing constraint must have been satisfied"
            d_vec.append(d)
            f_vec.append(f)
            y_err.append(max_f - f)
            vertex_num_vec.append(vertex_num)
        plt.loglog(
            vertex_num_vec,
            f_vec,
            # yerr=[np.zeros(len(y_err)), y_err],
            ".-",
            label=name,
        )

# plt.xlabel("code distance $d$")
plt.xlabel("Number of Vertices $|V|$")
plt.ylabel("frequency (MHz)")
plt.title("Best frequency")
plt.legend()
plt.savefig(os.path.join(this_dir, "best_frequency.pdf"))
