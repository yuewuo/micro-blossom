p_per_10 = 3
p_vec = [1e-4 * (10 ** (i / p_per_10)) for i in range(-1, p_per_10 * 2 + 1)]
d_vec = [3, 5, 7, 9, 11, 13, 15]


def plot_data(data, d_vec: list[int], p_vec: list[int]):
    print(data)
