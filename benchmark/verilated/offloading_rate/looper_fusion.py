from prepare import *


if __name__ == "__main__":
    run(
        name="looper_fusion",
        primal_dual_type="embedded-looper",
        primal_dual_config={
            "dual": {
                # "log_instructions": True,
                "sim_config": {
                    "support_offloading": True,
                    "support_layer_fusion": True,
                },
            }
        },
    )
