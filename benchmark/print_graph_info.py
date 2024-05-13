from micro_util import SingleGraph
import argparse


def main(args=None):
    parser = argparse.ArgumentParser(description="Print Graph Information")
    parser.add_argument(
        "-g",
        "--graph",
        required=True,
        help="the graph passed as the argument --graph in MicroBlossomGenerator; it also searches in /resources/graphs/",
    )
    args, parameters = parser.parse_known_args(args=args)
    graph = SingleGraph.from_file(args.graph)
    print(f"|V| = {graph.vertex_num}")
    print(f"|E| = {len(graph.weighted_edges)}")
    print(f"|P| = {len(graph.offloading)}")


if __name__ == "__main__":
    main()
