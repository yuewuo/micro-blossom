import os
from run import *
import traceback


def main(config: Configuration):
    for d in config.d_vec:
        configuration = config.config_of(d)
        project = configuration.get_project()
        project.build()
        assert not project.timing_sanity_check_failed()

        # test hello world application
        project.build_embedded_binary()
        project.build_vivado_project(force_recompile_binary=True)

        if project.is_hardware_connected():
            tty_output = project.run_application()
            with open(
                os.path.join(project.hardware_proj_dir(), f"hello.log"), "w"
            ) as log:
                log.write(tty_output)
                assert "Hello world!" in tty_output
        else:
            print("skip hello world test because hardware is not connected")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Estimate Resource Usage")
    parser.add_argument(
        "--base-name",
        help="if provided, only run the matched configuration",
    )
    args = parser.parse_args()

    errors = []
    for configuration in configurations:
        if args.base_name is None or args.base_name == configuration.base_name:
            try:
                main(configuration)
            except Exception as e:
                error_str = traceback.format_exc()
                print(error_str)
                errors.append(error_str)
    if len(errors) > 0:
        for error in errors:
            print(error)
        raise Exception("have errors above")
